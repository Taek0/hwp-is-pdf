//! Read-only source-cache / rendered-root-line comparison for column diagnostics.
use anyhow::Result;
use hwp_engine::{
    layout::Engine,
    model::{Document, text},
};
use std::path::Path;
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let mut doc = Document::open(Path::new(&args[0]))?;
    let mut rows = Vec::new();
    for (section, s) in doc.sections.iter_mut().enumerate() {
        for (index, p) in s.children.iter_mut().filter(|p| p.name == "p").enumerate() {
            let id = format!("diagnostic-{section}-{index}");
            let original_id = p.a("id").to_owned();
            let cache: Vec<_> = p.child("linesegarray").into_iter().flat_map(|a| &a.children)
                .map(|l| serde_json::json!({"y":l.n("vertpos")/100.,"height":l.n("vertsize")/100.,"spacing":l.n("spacing")/100.})).collect();
            rows.push(serde_json::json!({"id":id,"sourceId":original_id,"section":section,"index":index,"empty":text(p).trim().is_empty(),"cache":cache}));
            p.attrs.insert("id".into(), id);
        }
    }
    let fonts = args.get(1).map(String::as_str).unwrap_or(if cfg!(windows) {
        "C:/Windows/Fonts"
    } else {
        "/mnt/c/Windows/Fonts"
    });
    let mut engine = Engine::new(&doc, Path::new(fonts));
    engine.render()?;
    for row in &mut rows {
        row["rendered"] = serde_json::to_value(
            engine
                .report
                .bounds
                .iter()
                .filter(|b| b.kind == "line" && b.id == row["id"].as_str().unwrap())
                .collect::<Vec<_>>(),
        )?;
    }
    println!("{}", serde_json::to_string_pretty(&rows)?);
    Ok(())
}
