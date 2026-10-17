use super::paint::State;
use super::*;
use crate::pdf::{Fonts, Op};
pub(super) fn draw(d: &[u8], s: &mut State, fonts: &mut Fonts, out: &mut Vec<Op>) -> Result<()> {
    let len = u16(d, 4)? as usize;
    let options = u16(d, 6)?;
    ensure!(options == 0, "Clipped/opaque WMF text unsupported");
    let raw = d.get(8..8 + len).context("Truncated WMF text")?;
    let dx_at = 8 + len.div_ceil(2) * 2;
    let dx = if d.len() >= dx_at + len * 2 {
        Some(&d[dx_at..dx_at + len * 2])
    } else {
        None
    };
    let font = fonts
        .family(&s.font.face, s.font.weight >= 600, s.font.italic)?
        .or(fonts.family("Arial", s.font.weight >= 600, s.font.italic)?)
        .context("Missing WMF font")?;
    let mut size = s.font.height.unsigned_abs() as f64 * s.sy().abs();
    ensure!(size > 0. && size < 2000., "Invalid WMF font size");
    let face = ttf_parser::Face::parse(&fonts.list[font].bytes, 0)?;
    if s.font.height > 0 {
        size *= face.units_per_em() as f64 / (face.ascender() as f64 - face.descender() as f64);
    }
    let ratio = (s.sx() / s.sy()).abs();
    let at = if s.align & 1 != 0 {
        s.at
    } else {
        (i16(d, 2)? as f64, i16(d, 0)? as f64)
    };
    let (mut x, mut base) = s.xy(at.0, at.1);
    if s.align & 24 == 0 {
        base += face.ascender() as f64 / face.units_per_em() as f64 * size;
    } else if s.align & 24 == 8 {
        base += face.descender() as f64 / face.units_per_em() as f64 * size;
    }
    let mut chars = vec![];
    let mut i = 0;
    while i < raw.len() {
        let count = if s.font.charset == 129 && raw[i] >= 0x81 && i + 1 < raw.len() {
            2
        } else {
            1
        };
        let encoding = if s.font.charset == 129 {
            encoding_rs::EUC_KR
        } else {
            encoding_rs::WINDOWS_1252
        };
        let (value, _, bad) = encoding.decode(&raw[i..i + count]);
        ensure!(!bad, "Invalid WMF text encoding");
        let advance = if let Some(dx) = dx {
            (i..i + count)
                .map(|j| i16(dx, j * 2).map(|v| v as f64 * s.sx()))
                .sum::<Result<f64>>()?
        } else {
            fonts.width(font, &value, size) * ratio
        };
        chars.push((value.into_owned(), advance));
        i += count;
    }
    let width: f64 = chars.iter().map(|(_, w)| w).sum();
    match s.align & 6 {
        2 => x -= width,
        6 => x -= width / 2.,
        _ => {}
    }
    for (text, advance) in chars {
        out.push(Op::Text {
            x,
            y: base,
            text,
            font,
            size,
            scale: ratio,
            color: s.color,
        });
        x += advance;
    }
    if s.align & 1 != 0 {
        s.at.0 += width / s.sx();
    }
    Ok(())
}
