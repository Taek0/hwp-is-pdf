//! Inspect OpenType horizontal/vertical glyph metrics without modifying fonts.
fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    anyhow::ensure!(
        args.len() >= 2 && args.len() <= 3,
        "FONT_DIRECTORY FONT_FAMILY [CHARACTERS]"
    );
    let mut f = hwp_engine::pdf::Fonts::new(std::path::Path::new(&args[0]));
    let id = f
        .family(&args[1], false, false)?
        .ok_or_else(|| anyhow::anyhow!("Missing face"))?;
    let face = ttf_parser::Face::parse(&f.list[id].bytes, 0)?;
    println!(
        "{} units={} ascent={} descent={} vertical={:?}/{:?}",
        f.list[id].name,
        face.units_per_em(),
        face.ascender(),
        face.descender(),
        face.vertical_ascender(),
        face.vertical_descender()
    );
    for c in args
        .get(2)
        .map(String::as_str)
        .unwrap_or("PA가나한(")
        .chars()
    {
        let Some(g) = face.glyph_index(c) else {
            continue;
        };
        println!(
            "U+{:04X} gid={} h={:?} v={:?} tsb={:?} origin={:?} bbox={:?}",
            c as u32,
            g.0,
            face.glyph_hor_advance(g),
            face.glyph_ver_advance(g),
            face.glyph_ver_side_bearing(g),
            face.glyph_y_origin(g),
            face.glyph_bounding_box(g)
        );
    }
    Ok(())
}
