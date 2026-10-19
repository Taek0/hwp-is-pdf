//! EMF+ only path images nested in dual-mode EMF streams.
use super::*;
use tiny_skia::{FillRule, Mask, PathBuilder, Pixmap, PixmapPaint, Rect, Transform};
type Saved = (crate::graphics::Matrix, Option<Mask>);
fn real(b: &[u8], at: usize) -> Result<f64> {
    let v = f32::from_bits(u32(b, at)?) as f64;
    ensure!(v.is_finite(), "Nonfinite EMF+ coordinate");
    Ok(v)
}
fn transform(b: &[u8]) -> Result<crate::graphics::Matrix> {
    let mut m = [0.; 6];
    for (i, v) in m.iter_mut().enumerate() {
        *v = real(b, i * 4)?;
    }
    Ok(m)
}
pub(super) struct Player {
    world: crate::graphics::Matrix,
    saved: BTreeMap<u32, Saved>,
    clip: Option<Mask>,
    infinite_regions: std::collections::BTreeSet<u8>,
    paths: BTreeMap<u8, tiny_skia::Path>,
    images: BTreeMap<u8, (bool, Vec<u8>)>,
    only: bool,
    depth: usize,
    pub(super) bitmap_fallback_pending: bool,
}
impl Player {
    pub(super) fn new(depth: usize) -> Self {
        Self {
            world: crate::graphics::IDENTITY,
            saved: BTreeMap::new(),
            clip: None,
            infinite_regions: Default::default(),
            paths: BTreeMap::new(),
            images: BTreeMap::new(),
            only: false,
            depth,
            bitmap_fallback_pending: false,
        }
    }
    pub(super) fn comment(
        &mut self,
        b: &[u8],
        canvas: &mut Pixmap,
        scale: (f64, f64),
    ) -> Result<()> {
        if b.get(..4) != Some(b"EMF+") {
            return Ok(());
        }
        let mut at = 4;
        while at + 12 <= b.len() {
            let kind = u16(b, at)?;
            let flags = u16(b, at + 2)?;
            let size = u32(b, at + 4)? as usize;
            let data_size = u32(b, at + 8)? as usize;
            ensure!(
                size >= 12 && size <= b.len() - at && data_size <= size - 12,
                "Invalid EMF+ record"
            );
            let d = &b[at + 12..at + 12 + data_size];
            at += size;
            match kind {
                0x4001 => self.only = flags & 1 == 0,
                0x402a => self.world = transform(d)?,
                0x402b => self.world = crate::graphics::IDENTITY,
                0x402c => {
                    let m = transform(d)?;
                    self.world = if flags & 0x2000 != 0 {
                        crate::graphics::compose(m, self.world)
                    } else {
                        crate::graphics::compose(self.world, m)
                    };
                }
                0x4025 => {
                    ensure!(self.saved.len() < 256, "EMF+ saved-state limit");
                    self.saved
                        .insert(u32(d, 0)?, (self.world, self.clip.clone()));
                }
                0x4026 => {
                    let id = u32(d, 0)?;
                    let (world, clip) = self.saved.get(&id).context("EMF+ restore state")?;
                    self.world = *world;
                    self.clip = clip.clone();
                    self.saved.retain(|k, _| *k < id);
                }
                0x4008 => {
                    ensure!(flags & 0x8000 == 0, "Continued EMF+ object unsupported");
                    let id = flags as u8;
                    match (flags >> 8) & 127 {
                        3 if self.only => {
                            self.paths.insert(id, path(d)?);
                        }
                        4 if self.only => {
                            ensure!(
                                u32(d, 4)? == 0 && u32(d, 8)? == 0x10000003,
                                "Unsupported EMF+ region object"
                            );
                            self.infinite_regions.insert(id);
                        }
                        5 => match u32(d, 4)? {
                            2 => {
                                ensure!(
                                    matches!(u32(d, 8)?, 3..=5),
                                    "Unsupported nested metafile type"
                                );
                                let n = u32(d, 12)? as usize;
                                ensure!(n <= 16 * 1024 * 1024, "Nested EMF+ image too large");
                                self.images.insert(
                                    id,
                                    (
                                        true,
                                        d.get(16..16 + n).context("Truncated nested EMF")?.to_vec(),
                                    ),
                                );
                            }
                            1 => {
                                ensure!(
                                    u32(d, 24)? == 1 && d.len() <= 16 * 1024 * 1024,
                                    "Unsupported raw EMF+ bitmap"
                                );
                                self.images.insert(
                                    id,
                                    (
                                        false,
                                        d.get(28..).context("EMF+ compressed bitmap")?.to_vec(),
                                    ),
                                );
                            }
                            _ => anyhow::bail!("Unsupported EMF+ image type"),
                        },
                        _ => {}
                    }
                }
                0x4014 if self.only => {
                    ensure!(flags & 0x8000 != 0, "EMF+ path brush object unsupported");
                    let color = u32(d, 0)?;
                    let p = self
                        .paths
                        .get(&(flags as u8))
                        .context("Missing EMF+ path")?;
                    let m = self.world;
                    let t = Transform::from_row(
                        (m[0] * scale.0) as f32,
                        (m[1] * scale.1) as f32,
                        (m[2] * scale.0) as f32,
                        (m[3] * scale.1) as f32,
                        (m[4] * scale.0) as f32,
                        (m[5] * scale.1) as f32,
                    );
                    let mut paint = tiny_skia::Paint::default();
                    paint.set_color_rgba8(
                        (color >> 16) as u8,
                        (color >> 8) as u8,
                        color as u8,
                        (color >> 24) as u8,
                    );
                    canvas.fill_path(p, &paint, FillRule::EvenOdd, t, self.clip.as_ref());
                }
                0x401b => {
                    ensure!(
                        self.depth < 8
                            && flags & 0x0800 == 0
                            && u32(d, 4)? == 2
                            && u32(d, 24)? == 3,
                        "Unsupported nested image placement"
                    );
                    let (metafile, bytes) = self
                        .images
                        .get(&(flags as u8))
                        .context("Missing EMF+ nested image")?;
                    let (src, w, h) = if *metafile {
                        let w = (u32(bytes, 16)? as i32 - u32(bytes, 8)? as i32 + 1) as f32;
                        let h = (u32(bytes, 20)? as i32 - u32(bytes, 12)? as i32 + 1) as f32;
                        ensure!(w > 0. && h > 0., "Invalid nested metafile bounds");
                        let png = super::emf::render_nested(bytes, w, h, self.depth + 1)?;
                        (Pixmap::decode_png(&png)?, w, h)
                    } else {
                        let image = image::load_from_memory(bytes)?.to_rgba8();
                        let (w, h) = image.dimensions();
                        ensure!(w as u64 * h as u64 <= 16_000_000, "EMF+ bitmap too large");
                        let mut raw = image.into_raw();
                        for p in raw.as_chunks_mut::<4>().0 {
                            for k in 0..3 {
                                p[k] = ((p[k] as u16 * p[3] as u16 + 127) / 255) as u8;
                            }
                        }
                        self.bitmap_fallback_pending = true;
                        (
                            Pixmap::from_vec(
                                raw,
                                tiny_skia::IntSize::from_wh(w, h).context("EMF+ bitmap size")?,
                            )
                            .context("EMF+ bitmap pixels")?,
                            w as f32,
                            h as f32,
                        )
                    };
                    let (sx, sy, sw, sh) = (real(d, 8)?, real(d, 12)?, real(d, 16)?, real(d, 20)?);
                    ensure!(sw > 0. && sh > 0., "Invalid EMF+ image source rectangle");
                    let mut p = [(0., 0.); 3];
                    for (i, v) in p.iter_mut().enumerate() {
                        let (raw_x, raw_y) = if flags & 0x4000 != 0 {
                            (i16(d, 28 + i * 4)? as f64, i16(d, 30 + i * 4)? as f64)
                        } else {
                            (real(d, 28 + i * 8)?, real(d, 32 + i * 8)?)
                        };
                        let (x, y) = crate::graphics::point(self.world, raw_x, raw_y);
                        *v = (x * scale.0, y * scale.1);
                    }
                    let ax = (p[1].0 - p[0].0) / sw;
                    let ay = (p[1].1 - p[0].1) / sw;
                    let bx = (p[2].0 - p[0].0) / sh;
                    let by = (p[2].1 - p[0].1) / sh;
                    let px = w as f64 / src.width() as f64;
                    let py = h as f64 / src.height() as f64;
                    let t = Transform::from_row(
                        (ax * px) as f32,
                        (ay * px) as f32,
                        (bx * py) as f32,
                        (by * py) as f32,
                        (p[0].0 - ax * sx - bx * sy) as f32,
                        (p[0].1 - ay * sx - by * sy) as f32,
                    );
                    canvas.draw_pixmap(
                        0,
                        0,
                        src.as_ref(),
                        &PixmapPaint {
                            quality: tiny_skia::FilterQuality::Bicubic,
                            ..Default::default()
                        },
                        t,
                        self.clip.as_ref(),
                    );
                }
                0x4030 if self.only => {
                    ensure!(
                        flags & 255 == 2 && (real(d, 0)? - 1.).abs() < 0.00001,
                        "Unsupported EMF+ page unit/scale"
                    );
                }
                0x4032 if self.only => {
                    let mode = (flags >> 8) & 15;
                    ensure!(mode <= 1, "Unsupported EMF+ clip mode");
                    let r = Rect::from_xywh(
                        real(d, 0)? as f32,
                        real(d, 4)? as f32,
                        real(d, 8)? as f32,
                        real(d, 12)? as f32,
                    )
                    .context("EMF+ clip rectangle")?;
                    let m = self.world;
                    let t = Transform::from_row(
                        (m[0] * scale.0) as f32,
                        (m[1] * scale.1) as f32,
                        (m[2] * scale.0) as f32,
                        (m[3] * scale.1) as f32,
                        (m[4] * scale.0) as f32,
                        (m[5] * scale.1) as f32,
                    );
                    let path = PathBuilder::from_rect(r);
                    if mode == 1
                        && let Some(mask) = &mut self.clip
                    {
                        mask.intersect_path(&path, FillRule::Winding, false, t);
                    } else {
                        let mut mask =
                            Mask::new(canvas.width(), canvas.height()).context("EMF+ clip mask")?;
                        mask.fill_path(&path, FillRule::Winding, false, t);
                        self.clip = Some(mask);
                    }
                }
                0x4034 if self.only => {
                    ensure!(
                        self.infinite_regions.contains(&(flags as u8)) && (flags >> 8) & 15 == 0,
                        "Unsupported EMF+ clip region"
                    );
                    self.clip = None;
                }
                0x4033 if self.only => anyhow::bail!("EMF+ path clip unsupported"),
                // Rendering settings and whole-canvas clips; nested paths are bounded by their pixmap.
                0x4004 => self.bitmap_fallback_pending = false,
                0x4002 | 0x401e..=0x4024 | 0x4030 | 0x4032..=0x4034 => {}
                _ if !self.only => {}
                _ => anyhow::bail!("Unsupported EMF+ only drawing {kind:#x}"),
            }
        }
        ensure!(at == b.len(), "Trailing EMF+ comment data");
        Ok(())
    }
}
fn path(d: &[u8]) -> Result<tiny_skia::Path> {
    let n = u32(d, 4)? as usize;
    let flags = u32(d, 8)?;
    ensure!(
        n <= 100000 && flags & 0x8000 == 0,
        "EMF+ path count/relative encoding"
    );
    let compressed = flags & 0x4000 != 0;
    let stride = if compressed { 4 } else { 8 };
    let types = d
        .get(12 + n * stride..12 + n * stride + n)
        .context("EMF+ path types")?;
    let p = (0..n)
        .map(|i| {
            let at = 12 + i * stride;
            if compressed {
                Ok((i16(d, at)? as f32, i16(d, at + 2)? as f32))
            } else {
                Ok((real(d, at)? as f32, real(d, at + 4)? as f32))
            }
        })
        .collect::<Result<Vec<_>>>()?;
    let mut b = PathBuilder::new();
    let mut i = 0;
    while i < n {
        match types[i] & 7 {
            0 => b.move_to(p[i].0, p[i].1),
            1 => b.line_to(p[i].0, p[i].1),
            3 => {
                ensure!(
                    i + 2 < n && types[i + 1] & 7 == 3 && types[i + 2] & 7 == 3,
                    "Incomplete EMF+ cubic"
                );
                b.cubic_to(
                    p[i].0,
                    p[i].1,
                    p[i + 1].0,
                    p[i + 1].1,
                    p[i + 2].0,
                    p[i + 2].1,
                );
                i += 2;
            }
            _ => anyhow::bail!("Unknown EMF+ path point"),
        }
        if types[i] & 128 != 0 {
            b.close();
        }
        i += 1;
    }
    b.finish().context("Empty EMF+ path")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn incomplete_cubic_and_nonfinite_coordinates_are_rejected() {
        let mut d = vec![0; 12];
        d[4..8].copy_from_slice(&2u32.to_le_bytes());
        for v in [0f32, 0., 10., 10.] {
            d.extend(v.to_le_bytes());
        }
        d.extend([0, 3]);
        assert!(path(&d).is_err());
        d[29] = 1;
        assert!(path(&d).is_ok());
        d[12..16].copy_from_slice(&f32::NAN.to_le_bytes());
        assert!(path(&d).is_err());
    }
    #[test]
    fn unknown_restore_and_truncated_records_are_errors() {
        let mut bytes = b"EMF+".to_vec();
        bytes.extend(0x4026u16.to_le_bytes());
        bytes.extend(0u16.to_le_bytes());
        bytes.extend(16u32.to_le_bytes());
        bytes.extend(4u32.to_le_bytes());
        bytes.extend(5u32.to_le_bytes());
        let mut image = Pixmap::new(8, 8).unwrap();
        assert!(
            Player::new(0)
                .comment(&bytes, &mut image, (1., 1.))
                .is_err()
        );
        bytes.pop();
        assert!(
            Player::new(0)
                .comment(&bytes, &mut image, (1., 1.))
                .is_err()
        );
    }
}
