//! Native grouped shapes store cumulative matrices in the root group's frame.
//! Axis-aligned geometry can be resolved without scaling the text-box font.
use super::*;
pub(super) fn axis_box(data: &[u8]) -> Result<Option<[f64; 4]>> {
    let count = u16(data, 50)? as usize;
    ensure!(count < 128, "Excessive group matrices");
    let read = |offset| -> Result<crate::graphics::Matrix> {
        let mut a = [0.; 6];
        for (i, v) in a.iter_mut().enumerate() {
            *v = f64::from_le_bytes(bytes(data, offset + i * 8, 8)?.try_into()?);
        }
        ensure!(
            a.iter().all(|v| v.is_finite()),
            "Nonfinite native group matrix"
        );
        Ok([a[0], a[3], a[1], a[4], a[2], a[5]])
    };
    let mut matrix = read(52)?;
    for i in 0..count {
        matrix = crate::graphics::compose(matrix, read(100 + i * 96)?);
        matrix = crate::graphics::compose(matrix, read(148 + i * 96)?);
    }
    if matrix[1].abs() > 1e-8 || matrix[2].abs() > 1e-8 || matrix[0] <= 0. || matrix[3] <= 0. {
        return Ok(None);
    }
    let rect = crate::graphics::rect(
        matrix,
        [0., 0., u32(data, 20)? as f64, u32(data, 24)? as f64],
    );
    ensure!(
        rect.iter()
            .all(|v| v.is_finite() && v.abs() < i32::MAX as f64),
        "Native group coordinate overflow"
    );
    Ok(Some([
        rect[0],
        rect[1],
        rect[2] - rect[0],
        rect[3] - rect[1],
    ]))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scale_translation_is_part_of_geometry_but_not_text_size() {
        let mut d = vec![0; 196];
        for (offset, value) in [(20, 1000u32), (24, 2000)] {
            d[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        d[50..52].copy_from_slice(&1u16.to_le_bytes());
        for (offset, a) in [
            (52, [1., 0., 900., 0., 1., 300.]),
            (100, [0.5, 0., 100., 0., 0.25, 0.]),
            (148, [1., 0., 0., 0., 1., 0.]),
        ] {
            for (i, v) in a.into_iter().enumerate() {
                d[offset + i * 8..offset + (i + 1) * 8].copy_from_slice(&f64::to_le_bytes(v));
            }
        }
        assert_eq!(axis_box(&d).unwrap(), Some([1000., 300., 500., 500.]));
        assert!(axis_box(&d[..100]).is_err());
    }
}
