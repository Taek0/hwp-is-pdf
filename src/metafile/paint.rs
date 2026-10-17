use super::*;
use crate::pdf::{Fonts, Op};

#[derive(Clone)]
pub(super) struct Font {
    pub height: i16,
    pub weight: i16,
    pub italic: bool,
    pub charset: u8,
    pub face: String,
}
#[derive(Clone)]
enum Object {
    Pen(u16, f64, [f64; 3]),
    Brush(u16, [f64; 3]),
    Font(Font),
}
pub(super) struct State {
    pub org: (f64, f64),
    pub extent: (f64, f64),
    pub target: [f64; 4],
    pen: Option<(f64, [f64; 3])>,
    brush: Option<[f64; 3]>,
    pub font: Font,
    pub color: [f64; 3],
    pub align: u16,
    pub at: (f64, f64),
}
impl State {
    pub fn xy(&self, x: f64, y: f64) -> (f64, f64) {
        (
            self.target[0] + (x - self.org.0) * self.target[2] / self.extent.0,
            self.target[1] + (y - self.org.1) * self.target[3] / self.extent.1,
        )
    }
    pub fn sx(&self) -> f64 {
        self.target[2] / self.extent.0
    }
    pub fn sy(&self) -> f64 {
        self.target[3] / self.extent.1
    }
}
fn color(v: u32) -> [f64; 3] {
    [
        (v & 255) as f64 / 255.,
        ((v >> 8) & 255) as f64 / 255.,
        ((v >> 16) & 255) as f64 / 255.,
    ]
}
fn put(objects: &mut Vec<Option<Object>>, value: Object) -> Result<()> {
    if let Some(slot) = objects.iter_mut().find(|s| s.is_none()) {
        *slot = Some(value);
    } else {
        ensure!(objects.len() < 4096, "Too many WMF objects");
        objects.push(Some(value));
    }
    Ok(())
}
pub(crate) fn draw(bytes: &[u8], id: &str, target: [f64; 4], fonts: &mut Fonts) -> Result<Vec<Op>> {
    let mut s = State {
        org: (0., 0.),
        extent: (1., 1.),
        target,
        pen: Some((1., [0.; 3])),
        brush: Some([1.; 3]),
        font: Font {
            height: -12,
            weight: 400,
            italic: false,
            charset: 0,
            face: "Arial".into(),
        },
        color: [0.; 3],
        align: 0,
        at: (0., 0.),
    };
    let mut objects = vec![];
    let mut out = vec![];
    for r in records(bytes)? {
        let d = r.data;
        match r.kind {
            0 => break,
            0x20b => s.org = (i16(d, 2)? as f64, i16(d, 0)? as f64),
            0x20c => {
                s.extent = (i16(d, 2)? as f64, i16(d, 0)? as f64);
                ensure!(
                    s.extent.0 != 0. && s.extent.1 != 0.,
                    "Zero WMF window extent"
                );
            }
            0x2fa => put(
                &mut objects,
                Object::Pen(
                    u16(d, 0)?,
                    i16(d, 2)?.unsigned_abs() as f64,
                    color(u32(d, 6)?),
                ),
            )?,
            0x2fc => put(&mut objects, Object::Brush(u16(d, 0)?, color(u32(d, 2)?)))?,
            0x2fb => {
                ensure!(d.len() >= 18, "Truncated WMF font");
                ensure!(
                    i16(d, 4)? == 0 && i16(d, 6)? == 0,
                    "Rotated WMF font unsupported"
                );
                let raw = &d[18..];
                let end = raw.iter().position(|b| *b == 0).unwrap_or(raw.len());
                let face = encoding_rs::EUC_KR.decode(&raw[..end]).0.into_owned();
                put(
                    &mut objects,
                    Object::Font(Font {
                        height: i16(d, 0)?,
                        weight: i16(d, 8)?,
                        italic: d[10] != 0,
                        charset: d[13],
                        face,
                    }),
                )?;
            }
            0x12d => {
                let index = u16(d, 0)? as usize;
                if index & 0x8000 != 0 {
                    match index & 0x7fff {
                        0 => s.brush = Some([1.; 3]),
                        4 => s.brush = Some([0.; 3]),
                        5 => s.brush = None,
                        7 => s.pen = Some((1., [0.; 3])),
                        8 => s.pen = None,
                        _ => anyhow::bail!("Unsupported WMF stock object"),
                    }
                } else {
                    match objects
                        .get(index)
                        .and_then(|o| o.as_ref())
                        .context("Invalid WMF object reference")?
                    {
                        Object::Pen(style, width, c) => {
                            ensure!(matches!(style, 0 | 5), "Unsupported WMF pen style");
                            s.pen = if *style == 5 {
                                None
                            } else {
                                Some((*width, *c))
                            };
                        }
                        Object::Brush(style, c) => {
                            ensure!(matches!(style, 0 | 1), "Unsupported WMF brush style");
                            s.brush = if *style == 1 { None } else { Some(*c) };
                        }
                        Object::Font(f) => s.font = f.clone(),
                    }
                }
            }
            0x1f0 => {
                let i = u16(d, 0)? as usize;
                *objects.get_mut(i).context("Invalid WMF deleted object")? = None;
            }
            0x209 => s.color = color(u32(d, 0)?),
            0x12e => s.align = u16(d, 0)?,
            0x214 => s.at = (i16(d, 2)? as f64, i16(d, 0)? as f64),
            0x213 => {
                let next = (i16(d, 2)? as f64, i16(d, 0)? as f64);
                if let Some((width, c)) = s.pen {
                    let (a, b) = s.xy(s.at.0, s.at.1);
                    let (c1, d1) = s.xy(next.0, next.1);
                    out.push(Op::Line {
                        x: a,
                        y: b,
                        x2: c1,
                        y2: d1,
                        width: (width * s.sx().abs()).max(0.1),
                        color: c,
                    });
                }
                s.at = next;
            }
            0x324 | 0x325 => {
                let count = u16(d, 0)? as usize;
                ensure!(
                    count <= 32768 && d.len() >= 2 + 4 * count,
                    "Invalid WMF polygon"
                );
                let mut points = vec![];
                for i in 0..count {
                    points.push(s.xy(i16(d, 2 + 4 * i)? as f64, i16(d, 4 + 4 * i)? as f64));
                }
                let fill = if r.kind == 0x324 { s.brush } else { None };
                if s.pen.is_some() || fill.is_some() {
                    out.push(Op::Path {
                        points,
                        closed: r.kind == 0x324,
                        width: s.pen.map(|p| (p.0 * s.sx().abs()).max(0.1)).unwrap_or(0.),
                        color: s.pen.map(|p| p.1).unwrap_or(fill.unwrap_or([0.; 3])),
                        fill,
                    });
                }
            }
            0x41b => {
                let (x, y) = s.xy(i16(d, 6)? as f64, i16(d, 4)? as f64);
                let (right, bottom) = s.xy(i16(d, 2)? as f64, i16(d, 0)? as f64);
                if let Some(fill) = s.brush {
                    out.push(Op::Rect {
                        x: x.min(right),
                        y: y.min(bottom),
                        w: (right - x).abs(),
                        h: (bottom - y).abs(),
                        fill,
                    });
                }
                if let Some((width, color)) = s.pen {
                    out.push(Op::Path {
                        points: vec![(x, y), (right, y), (right, bottom), (x, bottom)],
                        closed: true,
                        width: (width * s.sx().abs()).max(0.1),
                        color,
                        fill: None,
                    });
                }
            }
            0xf43 => {
                ensure!(
                    u32(d, 0)? == 0x00cc0020 && i16(d, 10)? == 0 && i16(d, 12)? == 0,
                    "Unsupported WMF raster crop"
                );
                let (x, y) = s.xy(i16(d, 20)? as f64, i16(d, 18)? as f64);
                let w = i16(d, 16)? as f64 * s.sx();
                let h = i16(d, 14)? as f64 * s.sy();
                ensure!(w > 0. && h > 0., "Mirrored WMF raster unsupported");
                out.push(Op::Image {
                    x,
                    y,
                    w,
                    h,
                    id: format!("{id}-wmf-{}", r.offset),
                });
            }
            0xa32 => super::text::draw(d, &mut s, fonts, &mut out)?,
            0x104 => ensure!(u16(d, 0)? == 13, "Unsupported WMF raster pen operation"),
            // Playback hints and application comments have no drawing commands.
            0x626 | 0x106 | 0x107 | 0x102 => {}
            _ => anyhow::bail!("Unsupported WMF drawing record 0x{:04x}", r.kind),
        }
    }
    Ok(out)
}
