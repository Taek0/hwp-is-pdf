//! Bounded software playback for WMF compound paths and destination-dependent ROPs.
//! No OS graphics API or executable OLE server is invoked.
use super::*;
use tiny_skia::{FillRule, Mask, Paint, PathBuilder, Pixmap, PixmapPaint, Rect, Stroke, Transform};

#[derive(Clone)]
pub(super) enum Brush {
    Solid([u8; 3]),
    Empty,
    Pattern(Pixmap),
}
#[derive(Clone)]
pub(super) enum Object {
    Brush(Brush),
    Pen(u16, f32, [u8; 3]),
    Font,
}
#[derive(Clone)]
struct State {
    origin: (f32, f32),
    extent: (f32, f32),
    brush: Brush,
    pen: Option<(u16, f32, [u8; 3])>,
    clip: Option<Rect>,
    clip_empty: bool,
    alternate: bool,
}
pub(super) fn rgb(v: u32) -> [u8; 3] {
    [v as u8, (v >> 8) as u8, (v >> 16) as u8]
}
pub(super) fn paint(c: [u8; 3]) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color_rgba8(c[0], c[1], c[2], 255);
    p
}
pub(super) fn dib(bytes: &[u8]) -> Result<Pixmap> {
    let mut record = vec![0; 22];
    record[..4].copy_from_slice(&0x00cc0020u32.to_le_bytes());
    record.extend(bytes);
    let image = image::load_from_memory(&raster::bitmap(&record)?)?.to_rgba8();
    let size = tiny_skia::IntSize::from_wh(image.width(), image.height())
        .context("WMF bitmap dimensions")?;
    Pixmap::from_vec(image.into_raw(), size).context("WMF bitmap pixels")
}
fn put(objects: &mut Vec<Option<Object>>, value: Object) -> Result<()> {
    if let Some(slot) = objects.iter_mut().find(|v| v.is_none()) {
        *slot = Some(value);
    } else {
        ensure!(objects.len() < 4096, "Too many WMF objects");
        objects.push(Some(value));
    }
    Ok(())
}
pub(super) fn needed(rs: &[Record<'_>]) -> bool {
    rs.iter()
        .any(|r| matches!(r.kind, 0x538 | 0x940 | 0xb41 | 0x142))
}
pub(super) fn render(rs: &[Record<'_>]) -> Result<Vec<u8>> {
    let window = rs
        .iter()
        .find(|r| r.kind == 0x20c)
        .context("Complex WMF window extent")?;
    let wx = i16(window.data, 2)? as f32;
    let wy = i16(window.data, 0)? as f32;
    ensure!(wx != 0. && wy != 0., "Zero WMF window extent");
    if let Some(bytes) = super::emf::embedded(rs)? {
        return super::emf::render(&bytes, wx, wy);
    }
    let width = (wx.abs() * 3.).ceil().clamp(1., 4096.) as u32;
    let height = (wy.abs() * 3.).ceil().clamp(1., 4096.) as u32;
    ensure!(
        u64::from(width) * u64::from(height) <= 16_000_000,
        "WMF software canvas too large"
    );
    let mut canvas = Pixmap::new(width, height).context("WMF software canvas")?;
    canvas.fill(tiny_skia::Color::WHITE);
    let mut s = State {
        origin: (0., 0.),
        extent: (wx, wy),
        brush: Brush::Solid([255; 3]),
        pen: Some((0, 1., [0; 3])),
        clip: None,
        clip_empty: false,
        alternate: true,
    };
    let mut stack = Vec::new();
    let mut objects = Vec::new();
    for r in rs {
        let d = r.data;
        let (sx, sy) = (width as f32 / s.extent.0, height as f32 / s.extent.1);
        let point = |x: f32, y: f32| ((x - s.origin.0) * sx, (y - s.origin.1) * sy);
        let rectangle = |b: usize, right: usize, top: usize, left: usize| -> Result<Rect> {
            let (x, y) = point(i16(d, left)? as f32, i16(d, top)? as f32);
            let (x2, y2) = point(i16(d, right)? as f32, i16(d, b)? as f32);
            Rect::from_ltrb(x.min(x2), y.min(y2), x.max(x2), y.max(y2))
                .context("Empty WMF rectangle")
        };
        let mask = if s.clip_empty {
            Some(Mask::new(width, height).unwrap())
        } else {
            s.clip.map(|rect| {
                let mut m = Mask::new(width, height).unwrap();
                m.fill_path(
                    &PathBuilder::from_rect(rect),
                    FillRule::Winding,
                    false,
                    Transform::identity(),
                );
                m
            })
        };
        match r.kind {
            0 => break,
            0x103 => ensure!(
                matches!(i16(d, 0)?, 7 | 8),
                "Unsupported complex WMF map mode"
            ),
            0x20b => s.origin = (i16(d, 2)? as f32, i16(d, 0)? as f32),
            0x20c => {
                s.extent = (i16(d, 2)? as f32, i16(d, 0)? as f32);
                ensure!(s.extent.0 != 0. && s.extent.1 != 0., "Zero WMF extent");
            }
            0x1e => {
                ensure!(stack.len() < 256, "Excessive saved WMF states");
                stack.push(s.clone());
            }
            0x127 => {
                let level = i16(d, 0)? as isize;
                let at = if level < 0 {
                    stack.len() as isize + level
                } else {
                    level - 1
                };
                ensure!(
                    at >= 0 && (at as usize) < stack.len(),
                    "Invalid WMF restore level"
                );
                s = stack[at as usize].clone();
                stack.truncate(at as usize);
            }
            0x12c => {
                ensure!(u16(d, 0)? == 0, "Nonempty WMF clip region unsupported");
                s.clip = None;
                s.clip_empty = false;
            }
            0x416 => {
                let rect = rectangle(0, 2, 4, 6)?;
                s.clip = match s.clip {
                    Some(old) => old.intersect(&rect),
                    None => Some(rect),
                };
                s.clip_empty |= s.clip.is_none();
            }
            0x106 => {
                let mode = u16(d, 0)?;
                ensure!(matches!(mode, 1 | 2), "WMF fill mode");
                s.alternate = mode == 1;
            }
            0x2fc => {
                let brush = match u16(d, 0)? {
                    0 => Brush::Solid(rgb(u32(d, 2)?)),
                    1 => Brush::Empty,
                    _ => anyhow::bail!("Unsupported complex WMF brush"),
                };
                put(&mut objects, Object::Brush(brush))?;
            }
            0x142 => {
                ensure!(u16(d, 2)? == 0, "WMF pattern palette");
                put(
                    &mut objects,
                    Object::Brush(Brush::Pattern(dib(d.get(4..).context("WMF pattern")?)?)),
                )?;
            }
            0x2fa => put(
                &mut objects,
                Object::Pen(
                    u16(d, 0)?,
                    i16(d, 2)?.unsigned_abs() as f32,
                    rgb(u32(d, 6)?),
                ),
            )?,
            0x2fb => put(&mut objects, Object::Font)?,
            0x12d => {
                let i = u16(d, 0)? as usize;
                if i & 0x8000 != 0 {
                    match i & 0x7fff {
                        0 => s.brush = Brush::Solid([255; 3]),
                        4 => s.brush = Brush::Solid([0; 3]),
                        5 => s.brush = Brush::Empty,
                        7 => s.pen = Some((0, 1., [0; 3])),
                        8 => s.pen = None,
                        _ => anyhow::bail!("Unsupported WMF stock object"),
                    }
                } else {
                    match objects
                        .get(i)
                        .and_then(Option::as_ref)
                        .context("WMF object index")?
                    {
                        Object::Brush(b) => s.brush = b.clone(),
                        Object::Pen(style, w, c) => {
                            ensure!(*style <= 6, "Complex WMF pen style");
                            s.pen = if *style == 5 {
                                None
                            } else {
                                Some((*style, *w, *c))
                            };
                        }
                        Object::Font => {}
                    }
                }
            }
            0x1f0 => {
                *objects
                    .get_mut(u16(d, 0)? as usize)
                    .context("WMF deleted object")? = None;
            }
            0x324 | 0x325 | 0x538 => {
                let count = u16(d, 0)? as usize;
                let (counts, mut at) = if r.kind == 0x538 {
                    let counts = (0..count)
                        .map(|i| u16(d, 2 + i * 2).map(usize::from))
                        .collect::<Result<Vec<_>>>()?;
                    (counts, 2 + count * 2)
                } else {
                    (vec![count], 2)
                };
                ensure!(
                    counts.iter().sum::<usize>() <= 100000,
                    "Excessive WMF polygon points"
                );
                let mut path = PathBuilder::new();
                for count in counts {
                    for i in 0..count {
                        let (x, y) = point(i16(d, at)? as f32, i16(d, at + 2)? as f32);
                        at += 4;
                        if i == 0 {
                            path.move_to(x, y);
                        } else {
                            path.line_to(x, y);
                        }
                    }
                    if r.kind != 0x325 {
                        path.close();
                    }
                }
                if let Some(path) = path.finish() {
                    if r.kind != 0x325 {
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
                            Brush::Pattern(_) => {
                                anyhow::bail!("Pattern-filled WMF path unsupported")
                            }
                        }
                    }
                    if let Some((style, w, c)) = s.pen {
                        ensure!(
                            matches!(style, 0 | 6),
                            "Dashed complex WMF path unsupported"
                        );
                        canvas.stroke_path(
                            &path,
                            &paint(c),
                            &Stroke {
                                width: (w.max(1.) * sx.abs()).max(1.),
                                ..Default::default()
                            },
                            Transform::identity(),
                            mask.as_ref(),
                        );
                    }
                }
            }
            0xb41 => {
                ensure!(
                    u32(d, 0)? == 0x00cc0020 && i16(d, 8)? == 0 && i16(d, 10)? == 0,
                    "Unsupported WMF bitmap crop/ROP"
                );
                let src = dib(d.get(20..).context("WMF bitmap")?)?;
                ensure!(
                    u16(d, 4)? as u32 == src.height() && u16(d, 6)? as u32 == src.width(),
                    "Partial WMF bitmap source"
                );
                let (x, y) = point(i16(d, 18)? as f32, i16(d, 16)? as f32);
                let (w, h) = (i16(d, 14)? as f32 * sx, i16(d, 12)? as f32 * sy);
                canvas.draw_pixmap(
                    0,
                    0,
                    src.as_ref(),
                    &PixmapPaint {
                        quality: tiny_skia::FilterQuality::Bicubic,
                        ..Default::default()
                    },
                    Transform::from_row(
                        w / src.width() as f32,
                        0.,
                        0.,
                        h / src.height() as f32,
                        x,
                        y,
                    ),
                    mask.as_ref(),
                );
            }
            0x940 => {
                ensure!(d.len() == 18, "WMF DIBBITBLT source form unsupported");
                let rop = ((u32(d, 0)? >> 16) & 255) as u8;
                ensure!(
                    matches!(rop, 0x5a | 0xa0 | 0xaa),
                    "Unsupported WMF destination ROP"
                );
                if rop == 0xaa {
                    continue;
                }
                let (x, y) = point(i16(d, 16)? as f32, i16(d, 14)? as f32);
                let (w, h) = (i16(d, 12)? as f32 * sx, i16(d, 10)? as f32 * sy);
                for yy in y.floor().max(0.) as u32..(y + h).ceil().clamp(0., height as f32) as u32 {
                    for xx in
                        x.floor().max(0.) as u32..(x + w).ceil().clamp(0., width as f32) as u32
                    {
                        if let Some(m) = &mask
                            && m.data()[(yy * width + xx) as usize] == 0
                        {
                            continue;
                        }
                        let c = match &s.brush {
                            Brush::Solid(c) => *c,
                            Brush::Empty => continue,
                            Brush::Pattern(p) => {
                                let px = ((xx as f32 / sx.abs()) as u32) % p.width();
                                let py = ((yy as f32 / sy.abs()) as u32) % p.height();
                                let at = ((py * p.width() + px) * 4) as usize;
                                [p.data()[at], p.data()[at + 1], p.data()[at + 2]]
                            }
                        };
                        let at = ((yy * width + xx) * 4) as usize;
                        for (k, v) in c.iter().enumerate() {
                            if rop == 0x5a {
                                canvas.data_mut()[at + k] ^= *v;
                            } else {
                                canvas.data_mut()[at + k] &= *v;
                            }
                        }
                    }
                }
            }
            0x104 => ensure!(u16(d, 0)? == 13, "Complex WMF pen ROP"),
            // Text state is retained only as object slots: this renderer rejects text drawing.
            0x626 | 0x201 | 0x209 | 0x102 | 0x107 | 0x12e | 0x214 => {}
            _ => anyhow::bail!("Unsupported complex WMF record {:#x}", r.kind),
        }
    }
    Ok(canvas.encode_png()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(items: &[(u16, Vec<u16>)]) -> Vec<u8> {
        let mut b = vec![0; 18];
        b[0] = 1;
        b[2] = 9;
        b[5] = 3;
        for (kind, words) in items {
            b.extend(((3 + words.len()) as u32).to_le_bytes());
            b.extend(kind.to_le_bytes());
            for w in words {
                b.extend(w.to_le_bytes());
            }
        }
        b.extend(3u32.to_le_bytes());
        b.extend(0u16.to_le_bytes());
        let len = b.len() as u32 / 2;
        b[6..10].copy_from_slice(&len.to_le_bytes());
        b
    }
    #[test]
    fn compound_alternate_fill_preserves_holes() {
        let b = fixture(&[
            (0x20c, vec![20, 20]),
            (0x2fc, vec![0, 255, 0, 0]),
            (0x12d, vec![0]),
            (0x12d, vec![0x8008]),
            (
                0x538,
                vec![
                    2, 4, 4, 2, 2, 18, 2, 18, 18, 2, 18, 7, 7, 13, 7, 13, 13, 7, 13,
                ],
            ),
        ]);
        let png = render(&records(&b).unwrap()).unwrap();
        let im = image::load_from_memory(&png).unwrap().to_rgb8();
        assert_eq!(im.get_pixel(10, 10).0, [255, 0, 0]);
        assert_eq!(im.get_pixel(30, 30).0, [255, 255, 255]);
    }
    #[test]
    fn destination_xor_is_reversible_and_restore_is_bounded() {
        let operation = (0x940, vec![0x49, 0x5a, 0, 0, 0, 20, 20, 0, 0]);
        let b = fixture(&[(0x20c, vec![20, 20]), operation.clone(), operation]);
        let im = image::load_from_memory(&render(&records(&b).unwrap()).unwrap())
            .unwrap()
            .to_rgb8();
        assert!(im.pixels().all(|p| p.0 == [255; 3]));
        let bad = fixture(&[(0x20c, vec![20, 20]), (0x127, vec![0xffff])]);
        assert!(render(&records(&bad).unwrap()).is_err());
    }
}
