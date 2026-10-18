//! Searchable outline text; no duplicate text or rasterized glyphs.
use super::*;
#[derive(Clone)]
pub(super) struct Outline {
    width: f64,
    dash: Vec<f64>,
}
pub(super) fn read(style: &Node, size: f64) -> Option<Outline> {
    let kind = style.child("outline")?.a("type");
    let width = size * if kind == "BOLD" { 0.065 } else { 0.025 };
    let dash: &[f64] = match kind {
        "SOLID" | "BOLD" => &[],
        "DOT" => &[1., 2.],
        "DASH" => &[6., 3.],
        "DASH_DOT" => &[6., 2., 1., 2.],
        "DASH_DOT_DOT" => &[6., 2., 1., 2., 1., 2.],
        _ => return None,
    };
    Some(Outline {
        width,
        dash: dash.iter().map(|v| v * width.max(0.2)).collect(),
    })
}
pub(super) fn begin(ops: &mut Vec<Op>, outline: &Option<Outline>) {
    if let Some(outline) = outline {
        ops.push(Op::TransformBegin {
            matrix: crate::graphics::IDENTITY,
        });
        ops.push(Op::TextOutline {
            width: outline.width,
            dash: outline.dash.clone(),
        });
    }
}
pub(super) fn end(ops: &mut Vec<Op>, outline: &Option<Outline>) {
    if outline.is_some() {
        ops.push(Op::TransformEnd);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn outline_none_is_inactive_and_bold_is_wider() {
        let style = |s: &str| {
            crate::model::xml(format!("<charPr><outline type=\"{s}\"/></charPr>").as_bytes())
                .unwrap()
        };
        assert!(read(&style("NONE"), 20.).is_none());
        assert!(
            read(&style("BOLD"), 20.).unwrap().width > read(&style("SOLID"), 20.).unwrap().width
        );
        assert_eq!(read(&style("DASH_DOT"), 20.).unwrap().dash.len(), 4);
    }
}
