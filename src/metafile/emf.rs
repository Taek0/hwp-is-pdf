//! Non-executing EMF path playback for complete WMFC comment payloads.
use super::complex::{Brush, Object, dib, paint, rgb};
use super::*;
use tiny_skia::{FillRule, Mask, PathBuilder, Pixmap, PixmapPaint, Rect, Stroke, Transform};

pub(super) fn embedded(rs: &[Record<'_>]) -> Result<Option<Vec<u8>>> {
    let parts: Vec<_> = rs
        .iter()
        .filter(|r| r.kind == 0x626 && r.data.get(4..8) == Some(b"WMFC"))
        .collect();
    if parts.is_empty() {
        return Ok(None);
    }
    let total = u32(parts[0].data, 34)? as usize;
    ensure!(
        total <= 64 * 1024 * 1024 && u32(parts[0].data, 22)? as usize == parts.len(),
        "Invalid WMFC chunk count/size"
    );
    let mut out = Vec::with_capacity(total);
    for part in parts {
        let count = u32(part.data, 26)? as usize;
        ensure!(
            u32(part.data, 34)? as usize == total && out.len() + count <= total,
            "Inconsistent WMFC chunks"
        );
        out.extend(
            part.data
                .get(38..38 + count)
                .context("Truncated WMFC chunk")?,
        );
    }
    ensure!(
        out.len() == total && u32(&out, 0)? == 1 && out.get(40..44) == Some(b" EMF"),
        "Invalid embedded EMF"
    );
    Ok(Some(out))
}
fn signed(b: &[u8], at: usize) -> Result<f64> {
    Ok(u32(b, at)? as i32 as f64)
}
fn matrix(b: &[u8]) -> Result<crate::graphics::Matrix> {
    let mut m = [0.; 6];
    for (i, v) in m.iter_mut().enumerate() {
        *v = f32::from_bits(u32(b, 8 + i * 4)?) as f64;
    }
    ensure!(m.iter().all(|v| v.is_finite()), "Nonfinite EMF transform");
    Ok(m)
}
#[derive(Clone)]
struct State {
    world: crate::graphics::Matrix,
    viewport: (f64, f64),
    brush: Brush,
    pen: Option<(u16, f32, [u8; 3])>,
    clip: Option<Rect>,
    empty: bool,
    alternate: bool,
    at: (f32, f32),
}
impl State {
    fn xy(&self, x: f64, y: f64, scale: (f64, f64)) -> (f32, f32) {
        let (x, y) = crate::graphics::point(self.world, x, y);
        (
            ((x + self.viewport.0) * scale.0) as f32,
            ((y + self.viewport.1) * scale.1) as f32,
        )
    }
    fn mask(&self, w: u32, h: u32) -> Option<Mask> {
        if self.empty {
            return Mask::new(w, h);
        }
        self.clip.map(|r| {
            let mut m = Mask::new(w, h).unwrap();
            m.fill_path(
                &PathBuilder::from_rect(r),
                FillRule::Winding,
                false,
                Transform::identity(),
            );
            m
        })
    }
}
fn draw(
    canvas: &mut Pixmap,
    path: tiny_skia::Path,
    s: &State,
    scale: (f64, f64),
    fill: bool,
    stroke: bool,
) -> Result<()> {
    let mask = s.mask(canvas.width(), canvas.height());
    if fill {
        match &s.brush {
            Brush::Solid(c) => canvas.fill_path(
                &path,
                &paint(*c),
                if s.alternate {
                    FillRule::EvenOdd
                } else {
                    FillRule::Winding
                },
                Transform::identity(),
                mask.as_ref(),
            ),
            Brush::Empty => {}
            Brush::Pattern(_) => anyhow::bail!("EMF pattern path fill unsupported"),
        }
    }
    if stroke && let Some((style, w, c)) = s.pen {
        ensure!(matches!(style & 15, 0 | 6), "EMF dashed path unsupported");
        let line_width = if w == 0. {
            scale.0.abs() as f32
        } else {
            (w as f64 * (s.world[0] * scale.0).hypot(s.world[1] * scale.1)) as f32
        };
        canvas.stroke_path(
            &path,
            &paint(c),
            &Stroke {
                width: line_width.max(0.2),
                ..Default::default()
            },
            Transform::identity(),
            mask.as_ref(),
        );
    }
    Ok(())
}
fn bitmap(d: &[u8], bmi: usize, bmi_len: usize, bits: usize, bits_len: usize) -> Result<Pixmap> {
    let mut raw = d
        .get(bmi..bmi + bmi_len)
        .context("EMF DIB header")?
        .to_vec();
    raw.extend(d.get(bits..bits + bits_len).context("EMF DIB pixels")?);
    dib(&raw)
}
pub(super) fn render(bytes: &[u8], wx: f32, wy: f32) -> Result<Vec<u8>> {
    render_impl(bytes, wx, wy, 0, true)
}
pub(super) fn render_nested(bytes: &[u8], wx: f32, wy: f32, depth: usize) -> Result<Vec<u8>> {
    render_impl(bytes, wx, wy, depth, false)
}
fn render_impl(bytes: &[u8], wx: f32, wy: f32, depth: usize, opaque: bool) -> Result<Vec<u8>> {
    let width = (wx.abs() * 3.).ceil().clamp(1., 4096.) as u32;
    let height = (wy.abs() * 3.).ceil().clamp(1., 4096.) as u32;
    ensure!(
        width as u64 * height as u64 <= 16_000_000,
        "EMF canvas too large"
    );
    let scale = (width as f64 / wx as f64, height as f64 / wy as f64);
    let mut canvas = Pixmap::new(width, height).context("EMF canvas")?;
    if opaque {
        canvas.fill(tiny_skia::Color::WHITE);
    }
    let mut plus = super::emf_plus::Player::new(depth);
    let mut s = State {
        world: crate::graphics::IDENTITY,
        viewport: (0., 0.),
        brush: Brush::Solid([255; 3]),
        pen: Some((0, 1., [0; 3])),
        clip: None,
        empty: false,
        alternate: true,
        at: (0., 0.),
    };
    let mut objects: Vec<Option<Object>> = vec![None; 4096];
    let mut saved = Vec::new();
    let mut path = PathBuilder::new();
    let mut building = false;
    let mut at = 0;
    let mut count = 0;
    let mut eof = false;
    while at + 8 <= bytes.len() {
        let k = u32(bytes, at)?;
        let len = u32(bytes, at + 4)? as usize;
        ensure!(
            len >= 8 && len.is_multiple_of(4) && len <= bytes.len() - at && count < 100000,
            "Invalid EMF record"
        );
        let d = &bytes[at..at + len];
        at += len;
        count += 1;
        let xy = |x: usize, y: usize| -> Result<(f32, f32)> {
            Ok(s.xy(signed(d, x)?, signed(d, y)?, scale))
        };
        match k {
            1 => {}
            14 => {
                eof = true;
                break;
            }
            13 => s.viewport = (signed(d, 8)?, signed(d, 12)?),
            35 => s.world = matrix(d)?,
            36 => {
                let m = matrix(d)?;
                s.world = match u32(d, 32)? {
                    1 => crate::graphics::IDENTITY,
                    2 => crate::graphics::compose(s.world, m),
                    3 => crate::graphics::compose(m, s.world),
                    4 => m,
                    _ => anyhow::bail!("EMF transform mode"),
                };
            }
            33 => {
                ensure!(saved.len() < 256, "Excessive EMF saved states");
                saved.push(s.clone());
            }
            34 => {
                let level = signed(d, 8)? as isize;
                let index = if level < 0 {
                    saved.len() as isize + level
                } else {
                    level - 1
                };
                ensure!(
                    index >= 0 && (index as usize) < saved.len(),
                    "Invalid EMF restore"
                );
                s = saved[index as usize].clone();
                saved.truncate(index as usize);
            }
            39 | 82 | 94 | 95 => {
                let index = u32(d, 8)? as usize;
                ensure!(index > 0 && index < objects.len(), "EMF object index");
                objects[index] = Some(match k {
                    39 => Object::Brush(match u32(d, 12)? {
                        0 => Brush::Solid(rgb(u32(d, 16)?)),
                        1 => Brush::Empty,
                        _ => anyhow::bail!("EMF brush style"),
                    }),
                    82 => Object::Font,
                    94 => Object::Brush(Brush::Pattern(bitmap(
                        d,
                        u32(d, 16)? as usize,
                        u32(d, 20)? as usize,
                        u32(d, 24)? as usize,
                        u32(d, 28)? as usize,
                    )?)),
                    _ => {
                        ensure!(
                            u32(d, 36)? == 0 && u32(d, 48)? == 0,
                            "EMF patterned/custom pen"
                        );
                        Object::Pen(
                            (u32(d, 28)? & 0xf) as u16,
                            u32(d, 32)? as f32,
                            rgb(u32(d, 40)?),
                        )
                    }
                });
            }
            37 => {
                let i = u32(d, 8)?;
                if i & 0x80000000 != 0 {
                    match i & 0x7fffffff {
                        0 => s.brush = Brush::Solid([255; 3]),
                        4 => s.brush = Brush::Solid([0; 3]),
                        5 => s.brush = Brush::Empty,
                        7 => s.pen = Some((0, 1., [0; 3])),
                        8 => s.pen = None,
                        10..=17 => {}
                        _ => anyhow::bail!("EMF stock object"),
                    }
                } else {
                    match objects
                        .get(i as usize)
                        .and_then(Option::as_ref)
                        .context("Missing EMF object")?
                    {
                        Object::Brush(b) => s.brush = b.clone(),
                        Object::Pen(style, w, c) => {
                            s.pen = if *style == 5 {
                                None
                            } else {
                                Some((*style, *w, *c))
                            }
                        }
                        Object::Font => {}
                    }
                }
            }
            40 => {
                *objects
                    .get_mut(u32(d, 8)? as usize)
                    .context("EMF delete handle")? = None;
            }
            19 => {
                let mode = u32(d, 8)?;
                ensure!(matches!(mode, 1 | 2), "EMF polygon fill mode");
                s.alternate = mode == 1;
            }
            59 => {
                path = PathBuilder::new();
                building = true;
            }
            60 => building = false,
            61 => path.close(),
            27 => {
                let p = xy(8, 12)?;
                path.move_to(p.0, p.1);
                s.at = p;
            }
            54 => {
                let p = xy(8, 12)?;
                if !building {
                    path = PathBuilder::new();
                    path.move_to(s.at.0, s.at.1);
                }
                path.line_to(p.0, p.1);
                s.at = p;
                if !building && let Some(p) = std::mem::take(&mut path).finish() {
                    draw(&mut canvas, p, &s, scale, false, true)?;
                }
            }
            62..=64 => {
                if let Some(p) = std::mem::take(&mut path).finish() {
                    draw(&mut canvas, p, &s, scale, k != 63, k != 62)?;
                }
            }
            85..=89 => {
                let n = u32(d, 24)? as usize;
                ensure!(n <= 100000 && 28 + n * 4 <= d.len(), "EMF points");
                let points = (0..n)
                    .map(|i| {
                        Ok(s.xy(
                            i16(d, 28 + i * 4)? as f64,
                            i16(d, 30 + i * 4)? as f64,
                            scale,
                        ))
                    })
                    .collect::<Result<Vec<_>>>()?;
                let standalone = matches!(k, 85..=87);
                let mut offset = 0;
                if standalone {
                    if !building {
                        path = PathBuilder::new();
                    }
                    if let Some(p) = points.first() {
                        path.move_to(p.0, p.1);
                        offset = 1;
                    }
                }
                if matches!(k, 85 | 88) {
                    ensure!(
                        (points.len() - offset).is_multiple_of(3),
                        "EMF cubic points"
                    );
                    for p in points[offset..].as_chunks::<3>().0 {
                        path.cubic_to(p[0].0, p[0].1, p[1].0, p[1].1, p[2].0, p[2].1);
                    }
                } else {
                    for p in &points[offset..] {
                        path.line_to(p.0, p.1);
                    }
                }
                if k == 86 {
                    path.close();
                }
                if let Some(p) = points.last() {
                    s.at = *p;
                }
                if standalone
                    && !building
                    && let Some(p) = std::mem::take(&mut path).finish()
                {
                    draw(&mut canvas, p, &s, scale, k == 86, true)?;
                }
            }
            91 => {
                let polygons = u32(d, 24)? as usize;
                let points = u32(d, 28)? as usize;
                ensure!(points <= 100000 && polygons <= 100000, "EMF polygon count");
                let mut pos = 32 + polygons * 4;
                let mut consumed = 0;
                let mut p = PathBuilder::new();
                for i in 0..polygons {
                    let n = u32(d, 32 + i * 4)? as usize;
                    ensure!(consumed + n <= points, "EMF polygon lengths");
                    for j in 0..n {
                        let (x, y) = s.xy(i16(d, pos)? as f64, i16(d, pos + 2)? as f64, scale);
                        pos += 4;
                        if j == 0 {
                            p.move_to(x, y);
                        } else {
                            p.line_to(x, y);
                        }
                    }
                    p.close();
                    consumed += n;
                }
                ensure!(consumed == points, "EMF polygon point total");
                if let Some(p) = p.finish() {
                    draw(&mut canvas, p, &s, scale, true, true)?;
                }
            }
            30 => {
                let (x, y) = xy(8, 12)?;
                let (r, b) = xy(16, 20)?;
                let rect = Rect::from_ltrb(x.min(r), y.min(b), x.max(r), y.max(b))
                    .context("EMF clip rectangle")?;
                s.clip = match s.clip {
                    Some(old) => old.intersect(&rect),
                    None => Some(rect),
                };
                s.empty |= s.clip.is_none();
            }
            75 => {
                ensure!(u32(d, 12)? == 5, "EMF clip combine mode");
                if u32(d, 8)? == 0 {
                    s.clip = None;
                    s.empty = false;
                } else {
                    ensure!(u32(d, 24)? == 1, "Multi-rectangle EMF clip");
                    s.clip = Rect::from_ltrb(
                        (signed(d, 48)? * scale.0) as f32,
                        (signed(d, 52)? * scale.1) as f32,
                        (signed(d, 56)? * scale.0) as f32,
                        (signed(d, 60)? * scale.1) as f32,
                    );
                    s.empty = s.clip.is_none();
                }
            }
            76 => {
                let rop = ((u32(d, 40)? >> 16) & 255) as u8;
                ensure!(
                    u32(d, 84)? == 0 && u32(d, 92)? == 0 && matches!(rop, 0x5a | 0xa0 | 0xaa),
                    "EMF source bitmap transfer unsupported"
                );
                if rop == 0xaa {
                    continue;
                }
                let (x, y) = xy(24, 28)?;
                let (r, b) = s.xy(
                    signed(d, 24)? + signed(d, 32)?,
                    signed(d, 28)? + signed(d, 36)?,
                    scale,
                );
                let mask = s.mask(width, height);
                for yy in y.min(b).floor().max(0.) as u32..y.max(b).ceil().min(height as f32) as u32
                {
                    for xx in
                        x.min(r).floor().max(0.) as u32..x.max(r).ceil().min(width as f32) as u32
                    {
                        if mask
                            .as_ref()
                            .is_some_and(|m| m.data()[(yy * width + xx) as usize] == 0)
                        {
                            continue;
                        }
                        let color = match &s.brush {
                            Brush::Empty => continue,
                            Brush::Solid(c) => *c,
                            Brush::Pattern(p) => {
                                let px = ((xx as f64 / scale.0.abs()) as u32) % p.width();
                                let py = ((yy as f64 / scale.1.abs()) as u32) % p.height();
                                let a = ((py * p.width() + px) * 4) as usize;
                                [p.data()[a], p.data()[a + 1], p.data()[a + 2]]
                            }
                        };
                        let a = ((yy * width + xx) * 4) as usize;
                        for (i, c) in color.iter().enumerate() {
                            if rop == 0x5a {
                                canvas.data_mut()[a + i] ^= *c;
                            } else {
                                canvas.data_mut()[a + i] &= *c;
                            }
                        }
                    }
                }
            }
            81 => {
                if plus.bitmap_fallback_pending {
                    plus.bitmap_fallback_pending = false;
                    continue;
                }
                ensure!(
                    u32(d, 68)? == 0x00cc0020
                        && u32(d, 64)? == 0
                        && signed(d, 32)? == 0.
                        && signed(d, 36)? == 0.,
                    "EMF bitmap ROP/crop"
                );
                let src = bitmap(
                    d,
                    u32(d, 48)? as usize,
                    u32(d, 52)? as usize,
                    u32(d, 56)? as usize,
                    u32(d, 60)? as usize,
                )?;
                ensure!(
                    u32(d, 40)? == src.width() && u32(d, 44)? == src.height(),
                    "EMF partial bitmap"
                );
                let (x, y) = xy(24, 28)?;
                let (r, b) = s.xy(
                    signed(d, 24)? + signed(d, 72)?,
                    signed(d, 28)? + signed(d, 76)?,
                    scale,
                );
                let mask = s.mask(width, height);
                canvas.draw_pixmap(
                    0,
                    0,
                    src.as_ref(),
                    &PixmapPaint {
                        quality: tiny_skia::FilterQuality::Bicubic,
                        ..Default::default()
                    },
                    Transform::from_row(
                        (r - x) / src.width() as f32,
                        0.,
                        0.,
                        (b - y) / src.height() as f32,
                        x,
                        y,
                    ),
                    mask.as_ref(),
                );
            }
            20 => ensure!(u32(d, 8)? == 13, "EMF pen ROP"),
            // Font/text/color-management state has no drawing in these outline streams.
            70 => plus.comment(d.get(12..).context("EMF comment")?, &mut canvas, scale)?,
            18 | 21 | 22 | 24 | 25 | 28 | 58 | 98 | 115 => {}
            _ => anyhow::bail!("Unsupported embedded EMF record {k}"),
        }
    }
    ensure!(eof && at == bytes.len(), "Incomplete EMF stream");
    Ok(canvas.encode_png()?)
}
