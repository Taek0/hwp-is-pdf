use hwp_engine::{
    layout::Engine,
    model::{Document, xml},
    pdf::Op,
};
use std::path::Path;
fn fonts() -> &'static Path {
    Path::new(if cfg!(windows) {
        "C:/Windows/Fonts"
    } else {
        "/mnt/c/Windows/Fonts"
    })
}
fn doc(content: &str, direction: &str) -> Document {
    let mut d = Document::open(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/reflow/contour-single.hwpx"),
    )
    .unwrap();
    d.masters.clear();
    d.sections=vec![xml(format!(r#"<sec><p id="1" paraPrIDRef="0"><run charPrIDRef="0"><secPr textDirection="{direction}"><pagePr width="40000" height="30000"><margin left="4000" right="4000" top="4000" bottom="4000"/></pagePr></secPr>{content}</run></p></sec>"#).as_bytes()).unwrap()];
    d
}
#[test]
fn shadows_are_non_text_ink_and_vertical_flow_preserves_paper_and_source() {
    let mut d = doc("<t>ABC 가나다</t>", "VERTICALALL");
    d.chars.get_mut("0").unwrap().children.push(
        xml(br##"<shadow type="CONTINUOUS" color="#777777" offsetX="20" offsetY="20"/>"##).unwrap(),
    );
    let original = d.sections.clone();
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    assert_eq!(d.sections, original);
    assert_eq!(e.pages.len(), 1);
    assert_eq!((e.pages[0].width, e.pages[0].height), (400., 300.));
    assert!(
        e.pages[0]
            .ops
            .iter()
            .any(|o| matches!(o, Op::GlyphPaint { .. }))
    );
    let text: String = e.pages[0]
        .ops
        .iter()
        .filter_map(|o| {
            if let Op::Text { text, .. } = o {
                Some(text.as_str())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(text, "ABC 가나다");
    let mut nesting = 0;
    for op in &e.pages[0].ops {
        match op {
            Op::TransformBegin { .. } => nesting += 1,
            Op::TransformEnd => {
                nesting -= 1;
                assert!(nesting >= 0);
            }
            _ => {}
        }
    }
    assert_eq!(nesting, 0);
}
#[test]
fn page_parity_changes_number_without_inserting_body_sheets() {
    for (kind, expected) in [("EVEN", "2"), ("ODD", "1")] {
        let d = doc(
            &format!(
                r#"<ctrl><pageNumCtrl pageStartsOn="{kind}"/></ctrl><ctrl><pageNumPos position="5" format="0"/></ctrl><t>BODY</t>"#
            ),
            "HORIZONTAL",
        );
        let mut e = Engine::new(&d, fonts());
        e.render().unwrap();
        assert_eq!(e.pages.len(), 1);
        assert!(
            e.pages[0]
                .ops
                .iter()
                .any(|o| matches!(o,Op::Text{text,..} if text==expected))
        );
    }
}
#[test]
fn checkbox_caption_is_searchable_once_and_checked_ink_is_present() {
    let d = doc(
        r##"<checkBtn caption="Choice" value="CHECKED" printable="1" foreColor="#000000" backStyle="TRANSPARENT"><formCharPr charPrIDRef="0"/><sz width="6000" height="1700"/><pos treatAsChar="1"/></checkBtn>"##,
        "HORIZONTAL",
    );
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    assert!(
        e.pages[0]
            .ops
            .iter()
            .any(|o| matches!(o,Op::Path{points,closed:false,..} if points.len()==3))
            || e.pages[0]
                .ops
                .iter()
                .filter(|o| matches!(o, Op::GlyphPaint { .. }))
                .count()
                == 6
    );
    let count = e.pages[0]
        .ops
        .iter()
        .filter(|o| matches!(o,Op::Text{text,..} if text=="Choice"))
        .count();
    assert_eq!(count, 1);
}

#[test]
fn relation_scale_and_column_matrix_do_not_print_control_keywords() {
    let mut f = hwp_engine::pdf::Fonts::new(fonts());
    for (script, expected) in [("bold50", "50"), ("rmAOB", "AOB"), ("rmcm", "cm")] {
        let b = hwp_engine::math::layout(script, 18., &mut f).unwrap();
        let text: String = b
            .ops
            .iter()
            .filter_map(|o| {
                if let Op::Text { text, .. } = o {
                    Some(text.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(text, expected);
    }
    for script in [
        "A REL <-> {+2} {-5} B",
        "A BUILDREL <-> {+2} B",
        "matrix {lcol {a#bb} rcol {ccc#d}}",
    ] {
        let b = hwp_engine::math::layout(script, 18., &mut f).unwrap();
        let text: String = b
            .ops
            .iter()
            .filter_map(|o| {
                if let Op::Text { text, .. } = o {
                    Some(text.as_str())
                } else {
                    None
                }
            })
            .collect();
        for keyword in ["REL", "BUILDREL", "lcol", "rcol"] {
            assert!(!text.contains(keyword));
        }
        assert!(b.width > 0. && b.ascent > 0.);
    }
    let b = hwp_engine::math::layout("scale 70 {a} + scale 120 {b}", 18., &mut f).unwrap();
    assert!(
        b.ops
            .iter()
            .any(|o| matches!(o,Op::Text{text,size,..} if text=="a" && (*size-12.6).abs()<0.001))
    );
    assert!(
        b.ops
            .iter()
            .any(|o| matches!(o,Op::Text{text,size,..} if text=="b" && (*size-21.6).abs()<0.001))
    );
}

#[test]
fn only_unmodified_click_here_prompts_are_nonprinting() {
    let d = doc(
        r#"<ctrl><fieldBegin type="CLICK_HERE" modified="0"/></ctrl><t>PROMPT</t><ctrl><fieldEnd/></ctrl><t>END</t><ctrl><fieldBegin type="CLICK_HERE" modified="1"/></ctrl><t>USER</t><ctrl><fieldEnd/></ctrl>"#,
        "HORIZONTAL",
    );
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    let text: String = e.pages[0]
        .ops
        .iter()
        .filter_map(|o| {
            if let Op::Text { text, .. } = o {
                Some(text.as_str())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(text, "ENDUSER");
}

#[test]
fn proportional_text_baseline_does_not_include_half_the_interline_gap() {
    let d = doc("<t>ABC</t>", "HORIZONTAL");
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    assert!(
        e.pages[0]
            .ops
            .iter()
            .any(|o| matches!(o,Op::Text{text,y,..} if text=="A" && (*y-50.2).abs()<0.001))
    );
}

#[test]
fn vertical_upright_hangul_keeps_height_when_horizontal_ratio_is_compressed() {
    let mut d = doc("<t>가나다라마바사</t>", "VERTICAL");
    let style = d.chars.get_mut("0").unwrap();
    style
        .children
        .push(xml(br#"<ratio hangul="75" latin="75"/>"#).unwrap());
    style.children.push(xml(b"<italic/>").unwrap());
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    let q = hwp_engine::qa::inspect(&e.pages, &e.fonts, &e.report.bounds);
    assert!(q.text_overlaps.is_empty(), "{:?}", q.text_overlaps);
}

#[test]
fn left_right_parentheses_enclose_the_fraction_height() {
    let mut f = hwp_engine::pdf::Fonts::new(fonts());
    let inner = hwp_engine::math::layout("{a} over {b}", 18., &mut f).unwrap();
    let outer = hwp_engine::math::layout("LEFT ( {a} over {b} RIGHT )", 18., &mut f).unwrap();
    assert!(
        outer.width > inner.width && outer.ascent >= inner.ascent && outer.descent >= inner.descent
    );
    assert_eq!(
        outer
            .ops
            .iter()
            .filter(|o| matches!(o, Op::Text { text, size, .. } if (text=="(" || text==")") && *size>18.))
            .count(),
        2
    );
}

#[test]
fn vertical_all_keeps_latin_upright_but_rotates_parentheses() {
    let d = doc("<t>(A가)</t>", "VERTICALALL");
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    let mut matrix = hwp_engine::graphics::IDENTITY;
    let mut stack = Vec::new();
    let mut checked = 0;
    for op in &e.pages[0].ops {
        match op {
            Op::TransformBegin { matrix: m } => {
                stack.push(matrix);
                matrix = hwp_engine::graphics::compose(matrix, *m);
            }
            Op::TransformEnd => matrix = stack.pop().unwrap(),
            Op::Text { text, .. } if text == "(" => {
                assert!(matrix[0].abs() < 1e-9 && (matrix[1] - 1.).abs() < 1e-9);
                checked += 1;
            }
            Op::Text { text, .. } if text == "A" => {
                assert!((matrix[0] - 1.).abs() < 1e-9 && matrix[1].abs() < 1e-9);
                checked += 1;
            }
            _ => {}
        }
    }
    assert_eq!(checked, 2);
}

#[test]
fn ellipse_gradient_is_clipped_and_does_not_clip_following_text() {
    let d = doc(
        r##"<ellipse><sz width="10000" height="4000"/><pos treatAsChar="1"/><lineShape style="NONE"/><fillBrush><gradation type="RADIAL" angle="0" centerX="50" centerY="50"><color value="#000000"/><color value="#FFFFFF"/></gradation></fillBrush></ellipse><t>AFTER</t>"##,
        "HORIZONTAL",
    );
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    let ops = &e.pages[0].ops;
    assert!(
        ops.iter()
            .any(|o| matches!(o,Op::ClipPath{points} if points.len()==64))
    );
    let gradient = ops
        .iter()
        .position(|o| matches!(o, Op::Gradient { .. }))
        .unwrap();
    let text = ops
        .iter()
        .position(|o| matches!(o,Op::Text{text,..} if text=="A"))
        .unwrap();
    assert!(
        ops[gradient + 1..text]
            .iter()
            .any(|o| matches!(o, Op::TransformEnd))
    );
}

#[test]
fn vertical_character_offset_moves_across_the_column_not_down_the_run() {
    let mut d = doc("<t>가</t>", "VERTICAL");
    let mut style = d.chars["0"].clone();
    style
        .children
        .push(xml(br#"<offset hangul="20" latin="20"/>"#).unwrap());
    d.chars.insert("shifted".into(), style);
    d.sections[0].children[0]
        .children
        .push(xml("<run charPrIDRef=\"shifted\"><t>가</t></run>".as_bytes()).unwrap());
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    let mut matrix = hwp_engine::graphics::IDENTITY;
    let mut stack = Vec::new();
    let mut points = Vec::new();
    for op in &e.pages[0].ops {
        match op {
            Op::TransformBegin { matrix: m } => {
                stack.push(matrix);
                matrix = hwp_engine::graphics::compose(matrix, *m);
            }
            Op::TransformEnd => matrix = stack.pop().unwrap(),
            Op::Text { text, x, y, .. } if text == "가" => {
                points.push(hwp_engine::graphics::point(matrix, *x, *y))
            }
            _ => {}
        }
    }
    assert_eq!(points.len(), 2);
    assert!((points[1].0 - points[0].0 - 2.4).abs() < 0.001);
    assert!((points[1].1 - points[0].1 - 12.).abs() < 0.001);
}

#[test]
fn page_image_fill_is_painted_once_behind_text_and_gutter_offsets_body() {
    let mut d = doc("<t>ABC</t>", "HORIZONTAL");
    let sec = &mut d.sections[0].children[0].children[0].children[0];
    let page = sec
        .children
        .iter_mut()
        .find(|n| n.name == "pagePr")
        .unwrap();
    page.children
        .iter_mut()
        .find(|n| n.name == "margin")
        .unwrap()
        .attrs
        .insert("gutter".into(), "5500".into());
    sec.children.push(xml(br#"<pageBorderFill type="BOTH" borderFillIDRef="background" fillArea="PAPER" textBorder="CONTENT"/>"#).unwrap());
    d.borders.insert("background".into(),xml(br#"<borderFill><fillBrush><imgBrush mode="TOTAL"><img binaryItemIDRef="bg"/></imgBrush></fillBrush></borderFill>"#).unwrap());
    let mut png = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
        2,
        2,
        image::Rgb([230, 240, 250]),
    ))
    .write_to(&mut png, image::ImageFormat::Png)
    .unwrap();
    d.binaries.insert("bg".into(), png.into_inner());
    let mut e = Engine::new(&d, fonts());
    e.render().unwrap();
    let ops = &e.pages[0].ops;
    assert_eq!(
        ops.iter()
            .filter(|o| matches!(o,Op::Image{id,..} if id=="bg"))
            .count(),
        1
    );
    let image = ops
        .iter()
        .position(|o| matches!(o, Op::Image { .. }))
        .unwrap();
    let text = ops
        .iter()
        .position(|o| matches!(o,Op::Text{text,x,..} if text=="A" && (*x-95.).abs()<0.001))
        .unwrap();
    assert!(image < text);
}

#[test]
fn foreground_paper_objects_do_not_consume_body_lines_and_paint_after_text() {
    let d = doc(
        r##"<t>ANCHOR</t><rect id="overlay" textWrap="IN_FRONT_OF_TEXT"><sz width="5000" height="2000"/><pos treatAsChar="0" horzRelTo="PAPER" vertRelTo="PAPER" horzAlign="LEFT" vertAlign="TOP" horzOffset="18000" vertOffset="11000"/><lineShape style="NONE"/><fillBrush><winBrush faceColor="#FF0000"/></fillBrush></rect>"##,
        "HORIZONTAL",
    );
    let mut e = Engine::new(&d, fonts());
    e.render_reflow().unwrap();
    assert_eq!(e.pages.len(), 1);
    let ops = &e.pages[0].ops;
    let text = ops
        .iter()
        .position(|o| matches!(o,Op::Text{text,..} if text=="A"))
        .unwrap();
    let fill=ops.iter().position(|o|matches!(o,Op::Rect{x,y,w,h,fill} if (*x-180.).abs()<0.01 && (*y-110.).abs()<0.01 && *w==50. && *h==20. && *fill==[1.,0.,0.])).unwrap();
    assert!(fill > text);
}
#[test]
fn cacheless_fraction_rebuilds_baseline_without_shrinking_to_stale_descent() {
    let d = doc(
        r#"<equation baseUnit="1800" baseLine="85"><sz width="30000" height="5000"/><pos treatAsChar="1"/><script>{a+b} over {c+d}</script></equation>"#,
        "HORIZONTAL",
    );
    let mut e = Engine::new(&d, fonts());
    e.render_reflow().unwrap();
    let sizes: Vec<_> = e.pages[0]
        .ops
        .iter()
        .filter_map(|o| {
            if let Op::Text { size, .. } = o {
                Some(*size)
            } else {
                None
            }
        })
        .collect();
    assert!(!sizes.is_empty());
    assert!(sizes.iter().all(|s| *s >= 15.));
}
