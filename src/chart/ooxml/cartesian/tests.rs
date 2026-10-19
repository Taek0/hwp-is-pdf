use super::*;
#[test]
fn percent_stacks_cover_exactly_one_axis_range_per_category() {
    let path = std::path::Path::new(if cfg!(windows) {
        "C:/Windows/Fonts"
    } else {
        "/mnt/c/Windows/Fonts"
    });
    if !path.exists() {
        return;
    }
    let mut series = String::new();
    for (i, values) in [[1., 3.], [3., 1.]].iter().enumerate() {
        series += &format!(
            r#"<ser><idx val="{i}"/><order val="{i}"/><tx><v>S{i}</v></tx><cat><strLit><pt idx="0"><v>A</v></pt><pt idx="1"><v>B</v></pt></strLit></cat><val><numLit><pt idx="0"><v>{}</v></pt><pt idx="1"><v>{}</v></pt></numLit></val></ser>"#,
            values[0], values[1]
        );
    }
    let root=crate::model::xml(format!(r#"<chartSpace><chart><plotArea><barChart><barDir val="col"/><grouping val="percentStacked"/>{series}</barChart><catAx/><valAx/></plotArea></chart></chartSpace>"#).as_bytes()).unwrap();
    let mut fonts = Fonts::new(path);
    let ops = draw(&root, 0., 0., 400., 240., &mut fonts).unwrap();
    let (mut height, mut inside) = (0., false);
    let mut bars = Vec::new();
    for op in ops {
        match op {
            Op::ClipRect { h, .. } => {
                height = h;
                inside = true;
            }
            Op::TransformEnd => inside = false,
            Op::Rect { h, .. } if inside => bars.push(h),
            _ => {}
        }
    }
    assert_eq!(bars.len(), 4);
    assert!((bars[0] / height - 0.25).abs() < 1e-9 && (bars[1] / height - 0.75).abs() < 1e-9);
    assert!((bars[0] + bars[1] - height).abs() < 1e-9 && (bars[2] + bars[3] - height).abs() < 1e-9);
}
