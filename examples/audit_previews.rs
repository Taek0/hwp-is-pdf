//! Extract saved first-page thumbnails as secondary, potentially stale evidence.
use anyhow::{Context, Result, ensure};
use std::{
    io::{Cursor, Read},
    path::Path,
};
fn preview(path: &Path) -> Result<Option<Vec<u8>>> {
    let mut magic = [0; 4];
    std::fs::File::open(path)?.read_exact(&mut magic)?;
    let mut bytes = Vec::new();
    if magic.starts_with(b"PK") {
        let mut z = zip::ZipArchive::new(std::fs::File::open(path)?)?;
        let name = [
            "Preview/PrvImage.png",
            "Preview/PrvImage.jpg",
            "Preview/PrvImage.bmp",
        ]
        .into_iter()
        .find(|n| z.index_for_name(n).is_some());
        let Some(name) = name else { return Ok(None) };
        z.by_name(name)?
            .take(8 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)?;
    } else {
        let mut c = cfb::open(path)?;
        if !c.exists("/PrvImage") {
            return Ok(None);
        }
        c.open_stream("/PrvImage")?
            .take(8 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)?;
    }
    ensure!(bytes.len() <= 8 * 1024 * 1024, "Preview exceeds byte limit");
    if bytes.is_empty() {
        return Ok(None);
    }
    Ok(Some(bytes))
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let data = std::fs::read(&args[0])?;
    let inputs: Vec<serde_json::Value> =
        serde_json::from_slice(data.strip_prefix(&[239, 187, 191]).unwrap_or(&data))?;
    let out = Path::new(&args[1]);
    std::fs::create_dir_all(out)?;
    let mut results = Vec::new();
    for entry in inputs {
        let name = entry["name"].as_str().context("Name")?;
        ensure!(
            Path::new(name).components().count() == 1,
            "Unsafe preview name"
        );
        let input = entry["input"]
            .as_str()
            .context("Input")?
            .replace('\\', "/")
            .replace("D:/", "/mnt/d/");
        let result = (|| -> Result<Option<(u32, u32)>> {
            let Some(bytes) = preview(Path::new(&input))? else {
                return Ok(None);
            };
            std::fs::write(out.join(format!("{name}.source")), &bytes)?;
            let (w, h) = image::ImageReader::new(Cursor::new(&bytes))
                .with_guessed_format()?
                .into_dimensions()?;
            ensure!(
                u64::from(w) * u64::from(h) <= 16_000_000,
                "Oversized preview dimensions"
            );
            image::load_from_memory(&bytes)?.save(out.join(format!("{name}.png")))?;
            Ok(Some((w, h)))
        })();
        results.push(match result {
            Ok(Some((w,h)))=>serde_json::json!({"name":name,"status":"saved_thumbnail_not_an_oracle","width":w,"height":h}),
            Ok(None)=>serde_json::json!({"name":name,"status":"absent"}),
            Err(e)=>serde_json::json!({"name":name,"status":if out.join(format!("{name}.source")).exists(){"extracted_raw_needs_decoder"}else{"unavailable"},"reason":format!("{e:#}")}),
        });
    }
    std::fs::write(
        out.join("inventory.json"),
        serde_json::to_vec_pretty(&results)?,
    )?;
    println!("Audited {} saved previews", results.len());
    Ok(())
}
