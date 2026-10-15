//! TrueType loading, collection extraction and glyph metrics.
use super::*;
pub struct Font {
    pub bytes: Vec<u8>,
    pub name: String,
    pub used: BTreeMap<u16, char>,
    pub synthetic_bold: bool,
}
pub struct Fonts {
    pub list: Vec<Font>,
    pub root: PathBuf,
}
/// Unicode cmap first; Windows Symbol cmap (platform 3 / encoding 0) is not
/// searched by ttf-parser's Unicode-only Face::glyph_index convenience method.
pub fn glyph_id(face: &ttf_parser::Face<'_>, c: char) -> Option<ttf_parser::GlyphId> {
    face.glyph_index(c).or_else(|| {
        face.tables()
            .cmap?
            .subtables
            .into_iter()
            .filter(|t| t.platform_id == ttf_parser::PlatformId::Windows && t.encoding_id == 0)
            .find_map(|t| {
                t.glyph_index(c as u32).or_else(|| {
                    if (c as u32) <= 0xff {
                        t.glyph_index(0xf000 + c as u32)
                    } else {
                        None
                    }
                })
            })
    })
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
        let (file, index) = name
            .rsplit_once('#')
            .and_then(|(f, i)| i.parse::<u32>().ok().map(|i| (f, i)))
            .unwrap_or((name, 0));
        let bytes = std::fs::read(self.root.join(file))
            .with_context(|| format!("Font {name} missing in {}", self.root.display()))?;
        let bytes = if bytes.starts_with(b"ttcf") {
            extract_ttc_face(&bytes, index)?
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
            synthetic_bold: false,
        });
        Ok(id)
    }
    pub fn width(&self, id: usize, text: &str, size: f64) -> f64 {
        let f = ttf_parser::Face::parse(&self.list[id].bytes, 0).unwrap();
        text.chars()
            .map(|c| {
                glyph_id(&f, c)
                    .and_then(|g| f.glyph_hor_advance(g))
                    .unwrap_or(f.units_per_em()) as f64
                    / f.units_per_em() as f64
                    * size
            })
            .sum()
    }
    pub fn has(&self, id: usize, c: char) -> bool {
        glyph_id(
            &ttf_parser::Face::parse(&self.list[id].bytes, 0).unwrap(),
            c,
        )
        .is_some()
    }
    pub(super) fn encode(&mut self, id: usize, text: &str) -> String {
        let font = &mut self.list[id];
        let f = ttf_parser::Face::parse(&font.bytes, 0).unwrap();
        let mut result = String::new();
        for c in text.chars() {
            let gid = glyph_id(&f, c).map(|v| v.0).unwrap_or(0);
            font.used.insert(gid, c);
            result.push_str(&format!("{gid:04X}"));
        }
        result
    }
}
// PDF FontFile2 embeds one sfnt, not a TTC collection. Rebase table offsets
// and recompute checkSumAdjustment while leaving glyph data unmodified.
fn extract_ttc_face(src: &[u8], index: u32) -> Result<Vec<u8>> {
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
    anyhow::ensure!(index < be32(src, 8)?, "TTC face index outside collection");
    let offset = be32(src, 12 + 4 * index as usize)? as usize;
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
