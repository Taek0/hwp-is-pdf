use anyhow::{Context, Result};
use flate2::{Compression, write::ZlibEncoder};

use std::{
    collections::BTreeMap,
    io::Write,
    path::{Path, PathBuf},
};

pub(crate) mod navigation;
mod operations;
pub use operations::{Op, Page, PositionedGlyph};
mod font_catalog;
mod font_subset;
mod fonts;
mod glyph_paint;
mod gradients;
mod kerning;
pub use fonts::{Font, Fonts, glyph_id};
fn text_paint_mode(outline: Option<&(f64, Vec<f64>)>, bold: f64, color: [f64; 3]) -> String {
    if let Some((width, dash)) = outline {
        let dash = dash
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        format!(
            "{} {} {} RG {width:.4} w [{dash}] 0 d 1 Tr ",
            color[0], color[1], color[2]
        )
    } else if bold > 0. {
        format!(
            "{} {} {} RG {bold:.4} w 2 Tr ",
            color[0], color[1], color[2]
        )
    } else {
        String::new()
    }
}
fn compress(b: &[u8]) -> Result<Vec<u8>> {
    let mut z = ZlibEncoder::new(Vec::new(), Compression::default());
    z.write_all(b)?;
    Ok(z.finish()?)
}
struct Writer {
    objects: Vec<Vec<u8>>,
}
impl Writer {
    fn add(&mut self, b: impl Into<Vec<u8>>) -> usize {
        self.objects.push(b.into());
        self.objects.len()
    }
    fn stream(&mut self, dict: &str, b: &[u8]) -> Result<usize> {
        let data = compress(b)?;
        let mut out = format!(
            "<< /Length {} /Filter /FlateDecode {dict} >>\nstream\n",
            data.len()
        )
        .into_bytes();
        out.extend(data);
        out.extend(b"\nendstream");
        Ok(self.add(out))
    }
    fn save(&self, path: &Path, root: usize) -> Result<()> {
        let mut b = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
        let mut offsets = vec![0];
        for (i, o) in self.objects.iter().enumerate() {
            offsets.push(b.len());
            b.extend(format!("{} 0 obj\n", i + 1).bytes());
            b.extend(o);
            b.extend(b"\nendobj\n");
        }
        let pos = b.len();
        b.extend(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len()).bytes());
        for o in offsets.iter().skip(1) {
            b.extend(format!("{o:010} 00000 n \n").bytes());
        }
        b.extend(
            format!(
                "trailer\n<< /Size {} /Root {root} 0 R >>\nstartxref\n{pos}\n%%EOF\n",
                offsets.len()
            )
            .bytes(),
        );
        std::fs::write(path, b)?;
        Ok(())
    }
}
pub fn save(
    path: &Path,
    pages: &[Page],
    fonts: &mut Fonts,
    images: &BTreeMap<String, Vec<u8>>,
) -> Result<()> {
    let mut w = Writer {
        objects: Vec::new(),
    };
    let root = w.add(Vec::new());
    let tree = w.add(Vec::new());
    let mut streams = Vec::new();
    let mut image_ids = BTreeMap::new();
    for page in pages {
        let mut content = String::new();
        let mut actual_depth = 0usize;
        let mut outline: Option<(f64, Vec<f64>)> = None;
        let mut outline_stack = Vec::new();
        for op in &page.ops {
            match op {
                Op::ClipPath { points } => {
                    if let Some((x, y)) = points.first() {
                        content.push_str(&format!("{x} {} m\n", page.height - y));
                        for (x, y) in points.iter().skip(1) {
                            content.push_str(&format!("{x} {} l\n", page.height - y));
                        }
                        content.push_str("h W n\n");
                    }
                }
                Op::GlyphPaint {
                    x,
                    y,
                    font,
                    glyph,
                    size,
                    scale,
                    color,
                } => {
                    let face = ttf_parser::Face::parse(&fonts.list[*font].bytes, 0)?;
                    let unit = size / face.units_per_em() as f64;
                    let path = glyph_paint::path(&face, *glyph);
                    let stroke = fonts.ink_padding(*font, *size) * 2.;
                    let pen = if stroke > 0. {
                        format!(
                            "{} {} {} RG {} w\n",
                            color[0],
                            color[1],
                            color[2],
                            stroke / unit
                        )
                    } else {
                        String::new()
                    };
                    let paint = if stroke > 0. { "B" } else { "f" };
                    content.push_str(&format!(
                        "q {} {} {} rg {} 0 0 {unit} {x} {} cm\n{pen}{path}{paint} Q\n",
                        color[0],
                        color[1],
                        color[2],
                        unit * scale,
                        page.height - y
                    ));
                }
                Op::ClipRect { x, y, w: iw, h } => {
                    content.push_str(&format!("{x} {} {iw} {h} re W n\n", page.height - y - h))
                }
                Op::Gradient {
                    x,
                    y,
                    w: iw,
                    h,
                    kind,
                    angle,
                    center,
                    padding,
                    steps,
                    step_center,
                    colors,
                } => {
                    anyhow::ensure!(
                        colors.len() >= 2 && colors.len() <= 256,
                        "Gradient requires 2..256 colors"
                    );
                    anyhow::ensure!(
                        [
                            *x,
                            *y,
                            *iw,
                            *h,
                            *angle,
                            center[0],
                            center[1],
                            *padding,
                            *step_center
                        ]
                        .into_iter()
                        .all(f64::is_finite),
                        "Non-finite gradient geometry"
                    );
                    content.push_str(&format!("q 1 0 0 1 {x} {} cm\n", page.height - y - h));
                    content.push_str(&gradients::vector(
                        kind,
                        *angle,
                        *center,
                        *iw,
                        *h,
                        *padding,
                        *steps,
                        *step_center,
                        colors,
                    ));
                    content.push_str("Q\n");
                }
                Op::TextOutline { width, dash } => outline = Some((*width, dash.clone())),
                Op::Link { .. } | Op::Destination { .. } => {}
                Op::FieldText { .. } => anyhow::bail!("Unresolved PDF field value"),
                Op::ActualTextBegin { text } => {
                    let actual = text
                        .encode_utf16()
                        .map(|u| format!("{u:04X}"))
                        .collect::<String>();
                    content.push_str(&format!("/Span << /ActualText <FEFF{actual}> >> BDC\n"));
                    actual_depth += 1;
                }
                Op::ActualTextEnd => {
                    anyhow::ensure!(actual_depth > 0, "Unbalanced logical text span");
                    actual_depth -= 1;
                    content.push_str("EMC\n");
                }
                Op::Shaped {
                    x,
                    y,
                    text,
                    font,
                    size,
                    scale,
                    color,
                    glyphs,
                } => {
                    let actual = text
                        .encode_utf16()
                        .map(|u| format!("{u:04X}"))
                        .collect::<String>();
                    if actual_depth == 0 {
                        content.push_str(&format!("/Span << /ActualText <FEFF{actual}> >> BDC\n"));
                    }
                    content.push_str(&format!("q {} {} {} rg\n", color[0], color[1], color[2]));
                    let stroke = fonts.ink_padding(*font, *size) * 2.;
                    if stroke > 0. {
                        content.push_str(&format!(
                            "{} {} {} RG {stroke:.4} w\n",
                            color[0], color[1], color[2]
                        ));
                    }
                    let weight = if outline.is_some() {
                        text_paint_mode(outline.as_ref(), stroke, *color)
                    } else if stroke > 0. {
                        "2 Tr ".into()
                    } else {
                        String::new()
                    };
                    for glyph in glyphs {
                        fonts.list[*font]
                            .used
                            .entry(glyph.id)
                            .or_insert_with(|| "\u{fffd}".into());
                        content.push_str(&format!("BT {weight}/F{font} {size:.4} Tf {:.4} Tz 1 0 0 1 {:.4} {:.4} Tm <{:04X}> Tj ET\n",scale*100.,x+glyph.x,page.height-y-glyph.y,glyph.id));
                    }
                    content.push_str("Q\n");
                    if actual_depth == 0 {
                        content.push_str("EMC\n");
                    }
                }
                Op::StrokeDash { pattern } => {
                    let values: Vec<_> = pattern.iter().map(|v| format!("{v:.4}")).collect();
                    content.push_str(&format!("[{}] 0 d\n", values.join(" ")));
                }
                Op::TransformBegin { matrix: m } => {
                    outline_stack.push(outline.clone());
                    content.push_str(&format!(
                        "q {} {} {} {} {} {} cm\n",
                        m[0],
                        -m[1],
                        -m[2],
                        m[3],
                        m[4] + m[2] * page.height,
                        page.height - m[5] - m[3] * page.height
                    ));
                }
                Op::TransformEnd => {
                    outline = outline_stack.pop().unwrap_or(None);
                    content.push_str("Q\n");
                }
                Op::Bezier {
                    start,
                    segments,
                    closed,
                    width,
                    color,
                    fill,
                } => {
                    content.push_str(&format!(
                        "q {} {} {} RG {} w {} {} m\n",
                        color[0],
                        color[1],
                        color[2],
                        width,
                        start.0,
                        page.height - start.1
                    ));
                    for s in segments {
                        content.push_str(&format!(
                            "{} {} {} {} {} {} c\n",
                            s[0],
                            page.height - s[1],
                            s[2],
                            page.height - s[3],
                            s[4],
                            page.height - s[5]
                        ));
                    }
                    if *closed {
                        content.push_str("h\n");
                    }
                    if let Some(f) = fill {
                        let paint = if *width > 0. { "B" } else { "f" };
                        content.push_str(&format!("{} {} {} rg {paint} Q\n", f[0], f[1], f[2]));
                    } else {
                        content.push_str(if *width > 0. { "S Q\n" } else { "n Q\n" });
                    }
                }
                Op::Path {
                    points,
                    closed,
                    width,
                    color,
                    fill,
                } => {
                    if let Some((x, y)) = points.first() {
                        content.push_str(&format!(
                            "q {} {} {} RG {width:.4} w {x:.4} {:.4} m\n",
                            color[0],
                            color[1],
                            color[2],
                            page.height - y
                        ));
                        for (x, y) in points.iter().skip(1) {
                            content.push_str(&format!("{x:.4} {:.4} l\n", page.height - y));
                        }
                        if *closed {
                            content.push_str("h\n")
                        }
                        if let Some(f) = fill {
                            let paint = if *width > 0. { "B" } else { "f" };
                            content.push_str(&format!("{} {} {} rg {paint} Q\n", f[0], f[1], f[2]));
                        } else {
                            content.push_str(if *width > 0. { "S Q\n" } else { "n Q\n" })
                        }
                    }
                }
                Op::Text {
                    x,
                    y,
                    text,
                    font,
                    size,
                    scale,
                    color,
                } => {
                    let hex = fonts.encode(*font, text);
                    let stroke = fonts.ink_padding(*font, *size) * 2.;
                    let weight = text_paint_mode(outline.as_ref(), stroke, *color);
                    content.push_str(&format!("q {} {} {} rg BT {weight}/F{} {size:0.4} Tf {:0.4} Tz 1 0 0 1 {x:0.4} {:0.4} Tm <{hex}> Tj ET Q\n",color[0],color[1],color[2],font,scale*100.,page.height-y));
                }
                Op::Line {
                    x,
                    y,
                    x2,
                    y2,
                    width,
                    color,
                }
                | Op::DecorativeLine {
                    x,
                    y,
                    x2,
                    y2,
                    width,
                    color,
                } => content.push_str(&format!(
                    "q {} {} {} RG {width:0.4} w {x:0.4} {:0.4} m {x2:0.4} {:0.4} l S Q\n",
                    color[0],
                    color[1],
                    color[2],
                    page.height - y,
                    page.height - y2
                )),
                Op::Rect {
                    x,
                    y,
                    w: rw,
                    h,
                    fill,
                } => content.push_str(&format!(
                    "q {} {} {} rg {x:0.4} {:0.4} {rw:0.4} {h:0.4} re f Q\n",
                    fill[0],
                    fill[1],
                    fill[2],
                    page.height - y - h
                )),
                Op::Image { x, y, w: iw, h, id }
                | Op::ImageTile {
                    x, y, w: iw, h, id, ..
                }
                | Op::ImageClip {
                    x, y, w: iw, h, id, ..
                } => {
                    if !image_ids.contains_key(id) {
                        let bytes = images.get(id).context("Missing image payload")?;
                        let decoded = image::load_from_memory(bytes)?;
                        let rgba = decoded.to_rgba8();
                        let mask = if rgba.pixels().any(|p| p[3] != 255) {
                            let alpha = rgba.pixels().map(|p| p[3]).collect::<Vec<_>>();
                            let id=w.stream(&format!("/Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceGray /BitsPerComponent 8",rgba.width(),rgba.height()),&alpha)?;
                            format!(" /SMask {id} 0 R")
                        } else {
                            String::new()
                        };
                        let im = decoded.to_rgb8();
                        let obj=w.stream(&format!("/Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceRGB /BitsPerComponent 8{mask}",im.width(),im.height()),im.as_raw())?;
                        image_ids.insert(id.clone(), (obj, image_ids.len()));
                    }
                    let (_, index) = image_ids[id];
                    if let Op::ImageTile { tile_w, tile_h, .. } = op {
                        anyhow::ensure!(*tile_w > 0. && *tile_h > 0., "Invalid image tile size");
                        let cols = (iw / tile_w).ceil() as usize;
                        let rows = (h / tile_h).ceil() as usize;
                        anyhow::ensure!(
                            cols.checked_mul(rows).is_some_and(|n| n <= 16384),
                            "Excessive image tiles"
                        );
                        content
                            .push_str(&format!("q {x} {} {iw} {h} re W n\n", page.height - y - h));
                        for row in 0..rows {
                            for col in 0..cols {
                                let xx = x + col as f64 * tile_w;
                                let yy = y + row as f64 * tile_h;
                                content.push_str(&format!(
                                    "q {tile_w} 0 0 {tile_h} {xx} {} cm /I{index} Do Q\n",
                                    page.height - yy - tile_h
                                ));
                            }
                        }
                        content.push_str("Q\n");
                        continue;
                    }
                    if let Op::ImageClip { source: s, .. } = op {
                        anyhow::ensure!(
                            s.iter().all(|v| v.is_finite()) && s[2] > s[0] && s[3] > s[1],
                            "Invalid image crop"
                        );
                        let full_w = iw / (s[2] - s[0]);
                        let full_h = h / (s[3] - s[1]);
                        let px = x - s[0] * full_w;
                        let py = y - s[1] * full_h;
                        content.push_str(&format!(
                            "q {} {} {} {} re W n {} 0 0 {} {} {} cm /I{} Do Q\n",
                            x,
                            page.height - y - h,
                            iw,
                            h,
                            full_w,
                            full_h,
                            px,
                            page.height - py - full_h,
                            index
                        ));
                        continue;
                    }
                    content.push_str(&format!(
                        "q {iw:0.4} 0 0 {h:0.4} {x:0.4} {:0.4} cm /I{index} Do Q\n",
                        page.height - y - h
                    ));
                }
            }
        }
        anyhow::ensure!(actual_depth == 0, "Unclosed logical text span");
        streams.push(w.stream("", content.as_bytes())?);
    }
    let mut font_ids = Vec::new();
    for (index, font) in fonts.list.iter().enumerate() {
        let f = ttf_parser::Face::parse(&font.bytes, 0).unwrap();
        let units = f.units_per_em() as f64;
        let scale = 1000. / units;
        let subset = font_subset::subset(&font.bytes, font.used.keys().copied())?;
        let embedded = subset.as_deref().unwrap_or(&font.bytes);
        let file = w.stream(&format!("/Length1 {}", embedded.len()), embedded)?;
        let bb = f.global_bounding_box();
        let name = if subset.is_some() {
            let mut hash = crc32fast::hash(embedded);
            let tag: String = (0..6)
                .map(|_| {
                    let c = (b'A' + (hash % 26) as u8) as char;
                    hash /= 26;
                    c
                })
                .collect();
            format!("{tag}+HwpFont{index}")
        } else {
            format!("HwpFont{index}")
        };
        let desc=w.add(format!("<< /Type /FontDescriptor /FontName /{name} /Flags 4 /FontBBox [{} {} {} {}] /ItalicAngle 0 /Ascent {} /Descent {} /CapHeight {} /StemV 80 /FontFile2 {file} 0 R >>",bb.x_min as f64*scale,bb.y_min as f64*scale,bb.x_max as f64*scale,bb.y_max as f64*scale,f.ascender() as f64*scale,f.descender() as f64*scale,f.capital_height().unwrap_or(f.ascender()) as f64*scale));
        let mut widths = String::new();
        for &gid in font.used.keys() {
            widths.push_str(&format!(
                "{gid} [{}] ",
                f.glyph_hor_advance(ttf_parser::GlyphId(gid))
                    .unwrap_or(1000) as f64
                    * scale
            ));
        }
        let cid=w.add(format!("<< /Type /Font /Subtype /CIDFontType2 /BaseFont /{name} /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor {desc} 0 R /CIDToGIDMap /Identity /DW 1000 /W [{widths}] >>"));
        let mut cm="/CIDInit /ProcSet findresource begin 12 dict begin begincmap /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def /CMapName /HwpUnicode def /CMapType 2 def 1 begincodespacerange <0000> <FFFF> endcodespacerange\n".to_string();
        let pairs: Vec<_> = font.used.iter().collect();
        for chunk in pairs.chunks(100) {
            cm.push_str(&format!("{} beginbfchar\n", chunk.len()));
            for (gid, ch) in chunk {
                let utf: String = ch.encode_utf16().map(|v| format!("{v:04X}")).collect();
                cm.push_str(&format!("<{gid:04X}> <{utf}>\n"));
            }
            cm.push_str("endbfchar\n");
        }
        cm.push_str("endcmap CMapName currentdict /CMap defineresource pop end end");
        let uni = w.stream("", cm.as_bytes())?;
        font_ids.push(w.add(format!("<< /Type /Font /Subtype /Type0 /BaseFont /{name} /Encoding /Identity-H /DescendantFonts [{cid} 0 R] /ToUnicode {uni} 0 R >>")));
    }
    let fr: String = font_ids
        .iter()
        .enumerate()
        .map(|(i, id)| format!("/F{i} {id} 0 R "))
        .collect();
    let ir: String = image_ids
        .values()
        .map(|(id, i)| format!("/I{i} {id} 0 R "))
        .collect();
    let ids: Vec<_> = pages.iter().map(|_| w.add(Vec::new())).collect();
    let nav = navigation::install(&mut w, pages, &ids);
    for (i, (page, stream)) in pages.iter().zip(streams).enumerate() {
        let annotations = if nav.annotations[i].is_empty() {
            String::new()
        } else {
            format!(
                " /Annots [{}]",
                nav.annotations[i]
                    .iter()
                    .map(|id| format!("{id} 0 R "))
                    .collect::<String>()
            )
        };
        w.objects[ids[i]-1]=format!("<< /Type /Page /Parent {tree} 0 R /MediaBox [0 0 {} {}] /Resources << /Font << {fr} >> /XObject << {ir} >> >> /Contents {stream} 0 R{annotations} >>",page.width,page.height).into_bytes();
    }
    let names = nav
        .names
        .map(|id| format!(" /Names {id} 0 R"))
        .unwrap_or_default();
    let outlines = nav
        .outlines
        .map(|id| format!(" /Outlines {id} 0 R"))
        .unwrap_or_default();
    w.objects[root - 1] =
        format!("<< /Type /Catalog /Pages {tree} 0 R{names}{outlines} >>").into_bytes();
    w.objects[tree - 1] = format!(
        "<< /Type /Pages /Count {} /Kids [{}] >>",
        ids.len(),
        ids.iter()
            .map(|id| format!("{id} 0 R "))
            .collect::<String>()
    )
    .into_bytes();
    w.save(path, root)
}
