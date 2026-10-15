//! Inspect installed or embedded TrueType font families and face indices.
fn main() -> anyhow::Result<()> {
    let root = std::env::args()
        .nth(1)
        .unwrap_or("/mnt/c/Windows/Fonts".into());
    let mut paths: Vec<_> = std::fs::read_dir(root)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .collect();
    paths.sort();
    for path in paths {
        if !path.extension().is_some_and(|e| {
            matches!(
                e.to_string_lossy().to_ascii_lowercase().as_str(),
                "ttf" | "ttc"
            )
        }) {
            continue;
        }
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        for index in 0..ttf_parser::fonts_in_collection(&bytes).unwrap_or(1) {
            let Ok(face) = ttf_parser::Face::parse(&bytes, index) else {
                continue;
            };
            let names: std::collections::BTreeSet<_> = face
                .names()
                .into_iter()
                .filter(|n| matches!(n.name_id, 1 | 2 | 4 | 6 | 16 | 17))
                .filter_map(|n| n.to_string())
                .collect();
            println!(
                "{}#{index}: weight={} italic={} {:?}",
                path.file_name().unwrap().to_string_lossy(),
                face.weight().to_number(),
                face.is_italic(),
                names
            );
        }
    }
    Ok(())
}
