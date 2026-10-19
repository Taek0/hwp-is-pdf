//! Compose matching-style direction probes in memory; source documents are read-only.
use std::path::Path;
fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    anyhow::ensure!(
        args.len() == 4,
        "NORMAL_INPUT VERTICAL_INPUT OUTPUT_PDF FONT_DIRECTORY"
    );
    let mut d = hwp_engine::model::Document::open(Path::new(&args[0]))?;
    let v = hwp_engine::model::Document::open(Path::new(&args[1]))?;
    anyhow::ensure!(
        d.chars == v.chars && d.paras == v.paras && d.fonts == v.fonts && d.borders == v.borders,
        "Probe inputs must share style and binary definitions"
    );
    for (key, bytes) in &v.binaries {
        if let Some(existing) = d.binaries.get(key) {
            if existing != bytes {
                let is_section =
                    |b: &[u8]| hwp_engine::model::xml(b).is_ok_and(|n| n.name == "sec");
                anyhow::ensure!(
                    is_section(existing) && is_section(bytes),
                    "Conflicting binary asset {key}"
                );
            }
        } else {
            d.binaries.insert(key.clone(), bytes.clone());
        }
    }
    d.sections.extend(v.sections);
    let mut e = hwp_engine::layout::Engine::new(&d, Path::new(&args[3]));
    e.render()?;
    let out = Path::new(&args[2]);
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let qa = hwp_engine::qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    hwp_engine::pdf::save(out, &e.pages, &mut e.fonts, &d.binaries)?;
    std::fs::write(
        out.with_extension("qa.json"),
        serde_json::to_vec_pretty(&qa)?,
    )?;
    std::fs::write(
        out.with_extension("report.json"),
        serde_json::to_vec_pretty(&e.report)?,
    )?;
    println!("{} pages", e.pages.len());
    Ok(())
}
