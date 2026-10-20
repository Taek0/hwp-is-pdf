"""Compare the one-shot Docker CLI with a separately built WSL/Linux CLI.

Python is a verification driver only; both conversions use the Rust engine.
Requires pypdf; optional pdftoppm renders representative and changed pages.
All generated files and private font inventories belong in ignored output/.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import time
import uuid

from pypdf import PdfReader


def digest(path):
    with open(path, "rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def save(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2), encoding="utf-8")


def read(path):
    return json.loads(path.read_text(encoding="utf-8-sig"))


def linux_path(path):
    path = str(Path(path).resolve()).replace("\\", "/")
    if len(path) > 1 and path[1] == ":":
        return "/mnt/" + path[0].lower() + path[2:]
    return path


def run(command, timeout=180):
    start = time.monotonic()
    try:
        result = subprocess.run(command, capture_output=True, text=True,
                                encoding="utf-8", errors="replace", timeout=timeout)
        return {"command": command, "exit": result.returncode,
                "seconds": round(time.monotonic() - start, 3),
                "stdout": result.stdout, "stderr": result.stderr}
    except subprocess.TimeoutExpired:
        return {"command": command, "exit": "host-timeout", "seconds": timeout}


def pdf_info(path):
    reader = PdfReader(path)
    texts = [page.extract_text() or "" for page in reader.pages]
    return {"sha256": digest(path), "pages": len(reader.pages),
            "characters": [len(text) for text in texts],
            "text_sha256": [hashlib.sha256(t.encode()).hexdigest() for t in texts]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--image", required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--fonts", type=Path, required=True)
    parser.add_argument("--local-binary", required=True, help="Linux/WSL executable path")
    parser.add_argument("--wsl", help="WSL distribution; omit when running on Linux")
    parser.add_argument("--source-root", type=Path, required=True,
                        help="Search here by hash if a historical manifest path is gone")
    parser.add_argument("--output", type=Path, required=True, help="New, empty run directory")
    parser.add_argument("--select", help="Comma-separated manifest name prefixes")
    parser.add_argument("--pdftoppm", help="Optional Poppler executable")
    parser.add_argument("--jobs", type=int, default=1, choices=range(1, 5),
                        help="Concurrent document comparisons (each container is limited to 2 GiB)")
    args = parser.parse_args()
    out = args.output.resolve()
    if out.exists() and any(out.iterdir()):
        parser.error("--output must be empty; preserve previous evidence in a different directory")
    for part in ["inputs", "local", "container", "images", "logs"]:
        (out / part).mkdir(parents=True, exist_ok=True)
    items = read(args.manifest)
    if args.select:
        items = [i for i in items if any(i["name"].startswith(p) for p in args.select.split(","))]
    if not items:
        parser.error("no inputs selected")
    fonts = args.fonts.resolve()
    font_files = sorted(p for p in fonts.iterdir() if p.suffix.lower() in (".ttf", ".ttc"))
    if not font_files:
        parser.error("font directory contains no TTF/TTC files")
    font_snapshot = [{"name": p.name, "sha256": digest(p)} for p in font_files]
    save(out / "fonts.json", font_snapshot)
    sources = {}
    preserved = {}
    unavailable = []
    for item in items:
        for name in dict.fromkeys([item["input"], *item.get("copies", [])]):
            path = Path(name)
            if path.is_file():
                sha = digest(path)
                preserved[str(path)] = sha
                if sha != item["sha256"].lower():
                    raise ValueError(f"Original hash mismatch: {path}")
                sources[item["sha256"].lower()] = path
            else:
                unavailable.append(name)
    needed = {i["sha256"].lower() for i in items} - sources.keys()
    if needed:
        for root, dirs, files in os.walk(args.source_root):
            dirs[:] = [d for d in dirs if d not in {".git", "target", "node_modules", ".rust-tools", "Fonts"}]
            for name in files:
                path = Path(root) / name
                if path.suffix.lower() in (".hwp", ".hwpx"):
                    sha = digest(path)
                    if sha in needed:
                        sources[sha] = path
                        preserved[str(path)] = sha
            if needed <= sources.keys():
                break
    missing = sorted(needed - sources.keys())
    save(out / "source-inventory.json", {"checked": preserved, "unavailable": unavailable,
                                        "missing_hashes": missing})
    baseline = {i["name"]: i for i in read(args.baseline / "corpus-report.json")}
    image = run(["docker", "image", "inspect", args.image])
    save(out / "image.json", image)
    if image["exit"] != 0:
        raise RuntimeError(image)
    image_id = json.loads(image["stdout"])[0]["Id"]
    results = []
    def convert_case(pair):
        index, item = pair
        if item["sha256"].lower() not in sources:
            return {"name": item["name"], "local_exit": "source-unavailable",
                    "container_exit": "source-unavailable", "source_sha256": item["sha256"].lower()}
        source = sources[item["sha256"].lower()]
        staged = out / "inputs" / (item["name"] + source.suffix)
        shutil.copyfile(source, staged)
        local = out / "local" / (item["name"] + ".pdf")
        target = out / "container" / local.name
        flags = ["--reflow"] if item.get("reflow", False) else []
        local_cmd = [args.local_binary, linux_path(staged), linux_path(local), linux_path(fonts), *flags]
        if args.wsl:
            local_cmd = ["wsl", "-d", args.wsl, "--", *local_cmd]
        local_run = run(local_cmd)
        container_name = "hwp-verify-" + uuid.uuid4().hex[:16]
        docker_cmd = ["docker", "run", "--rm", "--name", container_name,
                      "--network", "none", "--read-only", "--cap-drop", "ALL",
                      "--security-opt", "no-new-privileges", "--cpus", "2",
                      "--memory", "2g", "--memory-swap", "2g", "--pids-limit", "64",
                      "--mount", f"type=bind,source={out / 'inputs'},target=/input,readonly",
                      "--mount", f"type=bind,source={out / 'container'},target=/output",
                      "--mount", f"type=bind,source={fonts},target=/fonts,readonly",
                      "--entrypoint", "/usr/bin/timeout", image_id, "--kill-after=5s", "120s",
                      "/usr/local/bin/hwp-engine", "/input/" + staged.name,
                      "/output/" + target.name, "/fonts", *flags]
        container_run = run(docker_cmd)
        if container_run["exit"] == "host-timeout":
            container_run["cleanup"] = run(["docker", "rm", "-f", container_name])
        save(out / "logs" / (item["name"] + ".json"), {"local": local_run, "container": container_run})
        row = {"name": item["name"], "source": str(source), "source_sha256": digest(source),
               "reflow": bool(flags), "local_exit": local_run["exit"], "container_exit": container_run["exit"]}
        if local_run["exit"] == container_run["exit"] == 0:
            row["local"] = pdf_info(local)
            row["container"] = pdf_info(target)
            row["pdf_identical"] = row["local"]["sha256"] == row["container"]["sha256"]
            row["text_identical"] = row["local"]["text_sha256"] == row["container"]["text_sha256"]
            for suffix in ("report.json", "qa.json"):
                row[suffix + "_identical"] = read(local.with_suffix("." + suffix)) == read(target.with_suffix("." + suffix))
            report = read(target.with_suffix(".report.json"))
            row["warnings"] = report["warnings"]
            row["qa"] = read(target.with_suffix(".qa.json"))
            old = args.baseline / item["group"] / local.name
            row["baseline_pdf_identical"] = old.is_file() and digest(old) == row["container"]["sha256"]
            row["baseline_pages_equal"] = baseline[item["name"]]["pages"] == row["container"]["pages"]
            row["baseline_warnings_equal"] = baseline[item["name"]]["warnings"] == row["warnings"]
            # Compare original sidecars: an aggregate ledger may have rounded floats
            # when it was parsed and serialized again. Keep both checks visible.
            row["baseline_ledger_qa_equal"] = baseline[item["name"]]["qa"] == row["qa"]
            old_qa = old.with_suffix(".qa.json")
            row["baseline_qa_equal"] = old_qa.is_file() and read(old_qa) == row["qa"]
            if args.pdftoppm and (index <= 3 or not row["baseline_pdf_identical"] or not row["pdf_identical"]):
                # Render every page of changed documents; first page for smoke checks.
                for label, pdf in [("container", target), ("local", local), ("baseline", old)]:
                    if pdf.is_file():
                        cmd = [args.pdftoppm, "-scale-to", "1400", "-png"]
                        if row["baseline_pdf_identical"] and row["pdf_identical"]:
                            cmd += ["-f", "1", "-singlefile"]
                        rendered = run([*cmd, str(pdf), str(out / "images" / (item["name"] + "-" + label))])
                        if rendered["exit"] != 0:
                            raise RuntimeError(rendered)
        return row
    with ThreadPoolExecutor(max_workers=args.jobs) as executor:
        for row in executor.map(convert_case, enumerate(items, 1)):
            results.append(row)
            save(out / "results.json", results)
            print(f"{len(results)}/{len(items)} {row['name']}: local={row['local_exit']} docker={row['container_exit']} identical={row.get('pdf_identical')}", flush=True)
    changes = [name for name, sha in preserved.items() if digest(name) != sha]
    font_changes = [f["name"] for f in font_snapshot if digest(fonts / f["name"]) != f["sha256"]]
    success = [r for r in results if r["local_exit"] == r["container_exit"] == 0]
    checks = ["pdf_identical", "text_identical", "report.json_identical", "qa.json_identical",
              "baseline_pdf_identical", "baseline_pages_equal", "baseline_warnings_equal", "baseline_qa_equal"]
    summary = {"inputs": len(items), "converted": len(success),
               "pages": sum(r["container"]["pages"] for r in success),
               "source_copies_checked": len(preserved), "unavailable_historical_paths": unavailable,
               "source_changes": changes, "font_changes": font_changes,
               "font_files": len(font_snapshot),
               "differences": {key: [r["name"] for r in results if not r.get(key)] for key in checks},
               "missing_glyph_warnings": [r["name"] for r in success if any("missing glyph" in w.lower() for w in r["warnings"])],
               "limits": "Engine/environment parity; does not establish new independent Hancom fidelity. Existing incomplete references and raw QA findings remain."}
    save(out / "summary.json", summary)
    print(json.dumps(summary, ensure_ascii=False, indent=2))
    return 1 if changes or font_changes or len(success) != len(items) or any(summary["differences"].values()) else 0


if __name__ == "__main__":
    raise SystemExit(main())
