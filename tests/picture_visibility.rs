use hwp_engine::{
    layout::Engine,
    model::{Document, xml},
    pdf::Op,
};
use std::{io::Cursor, path::Path};

fn fonts() -> &'static Path {
    Path::new(if cfg!(windows) {
        "C:/Windows/Fonts"
    } else {
        "/mnt/c/Windows/Fonts"
    })
}
fn picture_document(horizontal: &str, vertical: &str, source_format: &str) -> Document {
    let mut d = Document::open(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/reflow/contour-single.hwpx"),
    )
    .unwrap();
    let mut png = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(2, 2, image::Rgb([40, 80, 120])))
        .write_to(&mut png, image::ImageFormat::Png)
        .unwrap();
    d.binaries.insert("probe".into(), png.into_inner());
    d.masters.clear();
    d.sections = vec![xml(format!(r#"<sec><p paraPrIDRef="0"><run charPrIDRef="0"><secPr><pagePr width="40000" height="40000"><margin left="5000" right="5000" top="5000" bottom="5000"/></pagePr></secPr><pic binaryHwp="1" textWrap="BEHIND_TEXT" id="probe"><sz width="2000" height="3000"/><pos treatAsChar="0" horzRelTo="PAPER" vertRelTo="PAPER" horzAlign="{horizontal}" vertAlign="{vertical}"/><outMargin left="300" right="700" top="500" bottom="900"/><img binaryItemIDRef="probe" sourceFormat="{source_format}"/></pic><t>BODY</t></run><linesegarray><lineseg textpos="0" vertpos="0" vertsize="1200" baseline="1000" horzsize="30000"/></linesegarray></p></sec>"#).as_bytes()).unwrap()];
    d
}
#[test]
fn saved_picture_alignment_uses_the_whole_outside_box() {
    for (ha, va, x, y) in [
        ("LEFT", "TOP", 3., 5.),
        ("CENTER", "CENTER", 188., 183.),
        ("RIGHT", "BOTTOM", 373., 361.),
    ] {
        let d = picture_document(ha, va, "PNG");
        let original = d.sections.clone();
        let mut e = Engine::new(&d, fonts());
        e.render().unwrap();
        let b = e.report.bounds.iter().find(|b| b.id == "probe").unwrap();
        assert!(
            (b.x - x).abs() < 1e-8 && (b.y - y).abs() < 1e-8,
            "{ha}/{va}: {},{}",
            b.x,
            b.y
        );
        assert_eq!((b.w, b.h), (20., 30.));
        assert_eq!(d.sections, original);
    }
}
#[test]
fn pcx_visibility_is_opt_in_and_does_not_change_text_or_source() {
    for format in ["PCX", "PNG"] {
        let d = picture_document("LEFT", "TOP", format);
        let original = d.sections.clone();
        let original_bytes = d.binaries.clone();
        let mut normal = Engine::new(&d, fonts());
        normal.render().unwrap();
        let mut compatible = Engine::new(&d, fonts());
        compatible.hancom_hide_pcx = true;
        compatible.render().unwrap();
        assert_eq!(normal.report.pictures, 1);
        assert_eq!(compatible.report.pictures, usize::from(format != "PCX"));
        assert_eq!(normal.pages.len(), compatible.pages.len());
        let text = |e: &Engine<'_>| {
            e.pages
                .iter()
                .flat_map(|p| &p.ops)
                .filter_map(|op| {
                    if let Op::Text { text, x, y, .. } = op {
                        Some((text.clone(), *x, *y))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(text(&normal), text(&compatible));
        assert_eq!(d.sections, original);
        assert_eq!(d.binaries, original_bytes);
        assert_eq!(
            compatible
                .report
                .warnings
                .iter()
                .any(|w| w.contains("omitted by explicit")),
            format == "PCX"
        );
    }
}
#[test]
fn original_form_marks_only_legacy_logo_and_matches_watermark_anchor() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("복학원서.hwp");
    if !path.exists() {
        return;
    }
    let d = Document::open(&path).unwrap();
    let mut images = Vec::new();
    for s in &d.sections {
        s.all("img", &mut images);
    }
    assert!(
        images
            .iter()
            .any(|i| i.a("binaryItemIDRef") == "image1" && i.a("sourceFormat") == "PCX")
    );
    assert!(
        images
            .iter()
            .any(|i| i.a("binaryItemIDRef").starts_with("image2") && i.a("sourceFormat") != "PCX")
    );
    let mut e = Engine::new(&d, fonts());
    e.hancom_hide_pcx = true;
    e.render().unwrap();
    assert_eq!(e.report.pictures, 1);
    let b = e
        .report
        .bounds
        .iter()
        .find(|b| b.kind == "watermark")
        .unwrap();
    assert!((b.x - 111.8).abs() < 0.001 && (b.y - 211.2).abs() < 0.001);
}
