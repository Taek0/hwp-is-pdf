use hwp_engine::{layout::Engine, model::Document, pdf::Op};
#[test]
fn binary_char_flags_preserve_italic_and_double_strike() -> anyhow::Result<()> {
    use std::io::{Read, Write};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = std::env::temp_dir().join(format!("hwp-strike-{}.hwp", std::process::id()));
    std::fs::copy(root.join("samples/additional/underline-styles.hwp"), &path)?;
    {
        let mut f = cfb::open_rw(&path)?;
        let mut compressed = vec![];
        f.open_stream("/DocInfo")?.read_to_end(&mut compressed)?;
        let mut data = vec![];
        flate2::read::DeflateDecoder::new(compressed.as_slice()).read_to_end(&mut data)?;
        let mut pos = 0;
        let mut char_index = 0usize;
        while pos + 4 <= data.len() {
            let header = u32::from_le_bytes(data[pos..pos + 4].try_into()?);
            pos += 4;
            let mut len = (header >> 20) as usize;
            if len == 4095 {
                len = u32::from_le_bytes(data[pos..pos + 4].try_into()?) as usize;
                pos += 4;
            }
            if header & 1023 == 21 {
                let flags = u32::from_le_bytes(data[pos + 46..pos + 50].try_into()?);
                let script = if char_index.is_multiple_of(2) {
                    1 << 15
                } else {
                    1 << 16
                };
                let flags = (flags & !((7 << 18) | (15 << 26) | (3 << 15) | (15 << 21)))
                    | 1
                    | (1 << 18)
                    | (7 << 26)
                    | script
                    | (((char_index % 8) as u32) << 21);
                data[pos + 46..pos + 50].copy_from_slice(&flags.to_le_bytes());
                data[pos + 35..pos + 42].fill((-20i8) as u8);
                char_index += 1;
            }
            pos += len;
        }
        let mut encoder =
            flate2::write::DeflateEncoder::new(vec![], flate2::Compression::default());
        encoder.write_all(&data)?;
        let encoded = encoder.finish()?;
        let mut stream = f.open_stream("/DocInfo")?;
        stream.set_len(0)?;
        stream.write_all(&encoded)?;
    }
    let d = Document::open(&path)?;
    std::fs::remove_file(&path)?;
    for c in d.chars.values() {
        assert!(c.child("italic").is_some());
        assert_eq!(c.child("strikeout").unwrap().u("style"), 7);
        let sup = c.u("id").is_multiple_of(2);
        assert_eq!(c.child("supscript").is_some(), sup);
        assert_eq!(c.child("subscript").is_some(), !sup);
        assert_eq!(c.child("offset").unwrap().n("hangul"), -20.);
        assert_eq!(
            c.a("symMark"),
            [
                "",
                "DOT_ABOVE",
                "RING_ABOVE",
                "CARON",
                "TILDE",
                "SIDE",
                "COLON",
                ""
            ][c.u("id") % 8]
        );
    }
    Ok(())
}
#[test]
fn hwpx_named_decorations_and_italic_reach_pdf_ops() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let doc = Document::open(&root.join("samples/text-style/decorations.hwpx")).unwrap();
    let mut e = Engine::new(
        &doc,
        std::path::Path::new(if cfg!(windows) {
            "C:/Windows/Fonts"
        } else {
            "/mnt/c/Windows/Fonts"
        }),
    );
    e.render().unwrap();
    assert_eq!(e.pages.len(), 1);
    let ops = &e.pages[0].ops;
    let strokes = |row: usize| {
        ops.iter()
            .filter(|o| match o {
                Op::Path { points, .. } => {
                    points[0].1 > 50. + row as f64 * 40. - 1.
                        && points[0].1 < 50. + row as f64 * 40. + 25.
                }
                Op::Bezier { start, .. } => {
                    start.1 > 50. + row as f64 * 40. - 1. && start.1 < 50. + row as f64 * 40. + 25.
                }
                _ => false,
            })
            .count()
    };
    assert_eq!(strokes(0), 1);
    assert!(strokes(1) > 20);
    assert!(strokes(2) > strokes(1));
    assert_eq!(strokes(3), 2);
    assert_eq!(strokes(4), 1);
    assert_eq!(strokes(5), 2);
    assert!(strokes(6) > 20);
    for row in [7, 10, 11] {
        assert_eq!(strokes(row), 0, "unexpected line on row {row}");
    }
    assert_eq!(strokes(8), 1);
    assert_eq!(strokes(9), 2);
    let italic_fonts: Vec<_> = ops
        .iter()
        .filter_map(|o| {
            if let Op::Text { font, y, .. } = o {
                (*y > 525.).then_some(*font)
            } else {
                None
            }
        })
        .collect();
    assert!(!italic_fonts.is_empty());
    for id in italic_fonts {
        let face = ttf_parser::Face::parse(&e.fonts.list[id].bytes, 0).unwrap();
        assert!(face.is_italic());
    }
}
