use anyhow::{Context, Result};
use flate2::{Compression, write::ZlibEncoder};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize)]
pub enum Op {
    Path {
        points: Vec<(f64, f64)>,
        closed: bool,
        width: f64,
        color: [f64; 3],
        fill: Option<[f64; 3]>,
    },
    Text {
        x: f64,
        y: f64,
        text: String,
        font: usize,
        size: f64,
        scale: f64,
        color: [f64; 3],
    },
    Line {
        x: f64,
        y: f64,
        x2: f64,
        y2: f64,
        width: f64,
        color: [f64; 3],
    },
    Rect {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        fill: [f64; 3],
    },
    Image {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        id: String,
    },
}
#[derive(Default, Serialize)]
pub struct Page {
    pub width: f64,
    pub height: f64,
    pub ops: Vec<Op>,
}
pub struct Font {
    pub bytes: Vec<u8>,
    pub name: String,
    pub used: BTreeMap<u16, char>,
}
pub struct Fonts {
    pub list: Vec<Font>,
    pub root: PathBuf,
}
impl Fonts {
    pub fn new(root: &Path) -> Self {
        Self {
            list: Vec::new(),
            root: root.into(),
        }
    }
    pub fn get(&mut self, name: &str) -> Result<usize> {
        if let Some(i) = self.list.iter().position(|f| f.name == name) {
            return Ok(i);
        }
        let bytes = std::fs::read(self.root.join(name))
            .with_context(|| format!("Font {name} missing in {}", self.root.display()))?;
        let bytes = if bytes.starts_with(b"ttcf") {
            extract_ttc_face(&bytes)?
        } else {
            bytes
        };
        ttf_parser::Face::parse(&bytes, 0)
            .map_err(|_| anyhow::anyhow!("Invalid TrueType font {name}"))?;
        let id = self.list.len();
        self.list.push(Font {
            bytes,
            name: name.into(),
            used: BTreeMap::new(),
        });
        Ok(id)
    }
    pub fn width(&self, id: usize, text: &str, size: f64) -> f64 {
        let f = ttf_parser::Face::parse(&self.list[id].bytes, 0).unwrap();
        text.chars()
            .map(|c| {
                f.glyph_index(c)
                    .and_then(|g| f.glyph_hor_advance(g))
                    .unwrap_or(f.units_per_em()) as f64
                    / f.units_per_em() as f64
                    * size
            })
            .sum()
    }
    pub fn has(&self, id: usize, c: char) -> bool {
        ttf_parser::Face::parse(&self.list[id].bytes, 0)
            .unwrap()
            .glyph_index(c)
            .is_some()
    }
    fn encode(&mut self, id: usize, text: &str) -> String {
        let font = &mut self.list[id];
        let f = ttf_parser::Face::parse(&font.bytes, 0).unwrap();
        let mut result = String::new();
        for c in text.chars() {
            let gid = f.glyph_index(c).map(|v| v.0).unwrap_or(0);
            font.used.insert(gid, c);
            result.push_str(&format!("{gid:04X}"));
        }
        result
    }
}
// PDF FontFile2 embeds one sfnt, not a TTC collection. Rebase table offsets
// and recompute checkSumAdjustment while leaving glyph data unmodified.
fn extract_ttc_face(src: &[u8]) -> Result<Vec<u8>> {
    fn be16(b: &[u8], o: usize) -> Result<u16> {
        Ok(u16::from_be_bytes(
            b.get(o..o + 2).context("Truncated TTC")?.try_into()?,
        ))
    }
    fn be32(b: &[u8], o: usize) -> Result<u32> {
        Ok(u32::from_be_bytes(
            b.get(o..o + 4).context("Truncated TTC")?.try_into()?,
        ))
    }
    let offset = be32(src, 12)? as usize;
    let count = be16(src, offset + 4)? as usize;
    anyhow::ensure!(count < 4096, "Invalid TTC table count");
    let mut out = src
        .get(offset..offset + 12 + count * 16)
        .context("TTC directory")?
        .to_vec();
    let mut head = None;
    for i in 0..count {
        let dir = 12 + i * 16;
        let from = be32(src, offset + dir + 8)? as usize;
        let len = be32(src, offset + dir + 12)? as usize;
        let to = out.len();
        let bytes = src
            .get(from..from + len)
            .context("TTC table outside file")?;
        out.extend_from_slice(bytes);
        while out.len() % 4 != 0 {
            out.push(0)
        }
        out[dir + 8..dir + 12].copy_from_slice(&(to as u32).to_be_bytes());
        if &src[offset + dir..offset + dir + 4] == b"head" {
            anyhow::ensure!(len >= 12, "Short head table");
            head = Some(to);
            out[to + 8..to + 12].fill(0);
        }
    }
    let sum = out
        .as_chunks::<4>()
        .0
        .iter()
        .fold(0u32, |sum, b| sum.wrapping_add(u32::from_be_bytes(*b)));
    if let Some(head) = head {
        out[head + 8..head + 12].copy_from_slice(&0xb1b0afbau32.wrapping_sub(sum).to_be_bytes());
    }
    Ok(out)
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
        for op in &page.ops {
            match op {
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
                            content.push_str(&format!("{} {} {} rg B Q\n", f[0], f[1], f[2]));
                        } else {
                            content.push_str("S Q\n")
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
                    content.push_str(&format!("q {} {} {} rg BT /F{} {size:0.4} Tf {:0.4} Tz 1 0 0 1 {x:0.4} {:0.4} Tm <{hex}> Tj ET Q\n",color[0],color[1],color[2],font,scale*100.,page.height-y));
                }
                Op::Line {
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
                Op::Image { x, y, w: iw, h, id } => {
                    if !image_ids.contains_key(id) {
                        let bytes = images.get(id).context("Missing image payload")?;
                        let im = image::load_from_memory(bytes)?.to_rgb8();
                        let obj=w.stream(&format!("/Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceRGB /BitsPerComponent 8",im.width(),im.height()),im.as_raw())?;
                        image_ids.insert(id.clone(), (obj, image_ids.len()));
                    }
                    let (_, index) = image_ids[id];
                    content.push_str(&format!(
                        "q {iw:0.4} 0 0 {h:0.4} {x:0.4} {:0.4} cm /I{index} Do Q\n",
                        page.height - y - h
                    ));
                }
            }
        }
        streams.push(w.stream("", content.as_bytes())?);
    }
    let mut font_ids = Vec::new();
    for (index, font) in fonts.list.iter().enumerate() {
        let f = ttf_parser::Face::parse(&font.bytes, 0).unwrap();
        let units = f.units_per_em() as f64;
        let scale = 1000. / units;
        let file = w.stream(&format!("/Length1 {}", font.bytes.len()), &font.bytes)?;
        let bb = f.global_bounding_box();
        let name = format!("HwpFont{index}");
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
                let mut buf = [0; 2];
                let utf: String = ch
                    .encode_utf16(&mut buf)
                    .iter()
                    .map(|v| format!("{v:04X}"))
                    .collect();
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
    let mut ids = Vec::new();
    for (page, stream) in pages.iter().zip(streams) {
        ids.push(w.add(format!("<< /Type /Page /Parent {tree} 0 R /MediaBox [0 0 {} {}] /Resources << /Font << {fr} >> /XObject << {ir} >> >> /Contents {stream} 0 R >>",page.width,page.height)));
    }
    w.objects[root - 1] = format!("<< /Type /Catalog /Pages {tree} 0 R >>").into_bytes();
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
