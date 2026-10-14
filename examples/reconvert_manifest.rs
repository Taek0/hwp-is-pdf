//! Reconvert a saved input manifest without changing source fixtures.
fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let bytes = std::fs::read(&args[0])?;
    let manifest: Vec<serde_json::Value> =
        serde_json::from_slice(bytes.strip_prefix(&[239, 187, 191]).unwrap_or(&bytes))?;
    let out = std::path::Path::new(&args[1]);
    std::fs::create_dir_all(out)?;
    let mut results = vec![];
    for item in manifest {
        let source = item["input"]
            .as_str()
            .unwrap()
            .replace('\\', "/")
            .replace("D:/", "/mnt/d/");
        let target = out
            .join(item["group"].as_str().unwrap())
            .join(item["name"].as_str().unwrap())
            .with_extension("pdf");
        let r = hwp_engine::convert(
            std::path::Path::new(&source),
            &target,
            std::path::Path::new("/mnt/c/Windows/Fonts"),
        )?;
        let qa: serde_json::Value =
            serde_json::from_slice(&std::fs::read(target.with_extension("qa.json"))?)?;
        println!("{}: {} pages", item["name"], r.pages);
        results.push(serde_json::json!({"group":item["group"],"name":item["name"],"pages":r.pages,"qa":qa,"warnings":r.warnings}));
        std::fs::write(
            out.join("corpus-report.json"),
            serde_json::to_vec_pretty(&results)?,
        )?;
    }
    Ok(())
}
