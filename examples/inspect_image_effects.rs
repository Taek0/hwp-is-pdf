//! Export source and derived raster images for local comparison (never edits input).
use anyhow::{Context, Result, ensure};
use std::path::Path;

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    ensure!(
        args.len() == 2,
        "Usage: inspect_image_effects INPUT OUTPUT_DIR"
    );
    let doc = hwp_engine::model::Document::open(Path::new(&args[0]))?;
    let out = Path::new(&args[1]);
    std::fs::create_dir_all(out)?;
    let mut metadata = Vec::new();
    for (id, bytes) in &doc.binaries {
        if let Ok(image) = image::load_from_memory(bytes) {
            ensure!(
                id.chars()
                    .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
                    && !id.contains(".."),
                "Unsafe image identifier"
            );
            let path = out.join(format!("{id}.png"));
            ensure!(!path.exists(), "Output already exists: {}", path.display());
            image.save(&path).context("Saving diagnostic image")?;
            metadata
                .push(serde_json::json!({"id":id,"width":image.width(),"height":image.height()}));
        }
    }
    println!("{}", serde_json::to_string_pretty(&metadata)?);
    Ok(())
}
