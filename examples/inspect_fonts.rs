fn main() -> anyhow::Result<()> {
    let dir = std::env::args()
        .nth(1)
        .unwrap_or("/mnt/c/Windows/Fonts".into());
    for name in ["wingding.ttf", "symbol.ttf", "seguisym.ttf"] {
        let bytes = std::fs::read(std::path::Path::new(&dir).join(name))?;
        let face = ttf_parser::Face::parse(&bytes, 0)?;
        println!(
            "{name}: F06C={:?} 006C={:?}",
            face.glyph_index('\u{f06c}'),
            face.glyph_index('l')
        );
        for table in face.tables().cmap.unwrap().subtables {
            println!(
                "  {:?}/{}: F06C={:?}",
                table.platform_id,
                table.encoding_id,
                table.glyph_index(0xf06c)
            );
        }
    }
    let doc =
        hwp_engine::model::Document::open(std::path::Path::new("samples/features/numbering.hwp"))?;
    println!(
        "fonts {:?}; bullets {:?}",
        doc.fonts,
        doc.header.children_named("bullet").collect::<Vec<_>>()
    );
    Ok(())
}
