//! Deterministic pixel oracle for native PDF rotation and image clipping.
use hwp_engine::{
    graphics,
    pdf::{self, Fonts, Op, Page},
};
use std::{collections::BTreeMap, io::Cursor, path::Path};
fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|s| s == "verify") {
        let im = image::open(&args[1])?.to_rgb8();
        for (x, y, c) in [
            (45, 65, [255, 0, 0]),
            (185, 65, [0, 255, 0]),
            (235, 115, [0, 255, 0]),
            (325, 65, [0, 0, 255]),
            (375, 65, [255, 0, 0]),
            (325, 115, [255, 255, 0]),
            (375, 115, [0, 255, 0]),
        ] {
            anyhow::ensure!(
                im.get_pixel(x, y).0 == c,
                "Unexpected pixel at {x},{y}: {:?}",
                im.get_pixel(x, y)
            );
        }
        println!("7 PDF raster color probes passed (72 DPI)");
        return Ok(());
    }
    let im = image::RgbImage::from_fn(100, 100, |x, y| {
        image::Rgb(match (x < 50, y < 50) {
            (true, true) => [255, 0, 0],
            (false, true) => [0, 255, 0],
            (true, false) => [0, 0, 255],
            _ => [255, 255, 0],
        })
    });
    let mut data = Cursor::new(Vec::new());
    im.write_to(&mut data, image::ImageFormat::Png)?;
    let mut f = Fonts::new(Path::new(if cfg!(windows) {
        "C:/Windows/Fonts"
    } else {
        "/mnt/c/Windows/Fonts"
    }));
    let font = f.get("arial.ttf")?;
    let mut ops = Vec::new();
    for (x, s) in [
        (20., "Original"),
        (160., "Crop: top-right"),
        (300., "Rotate: 90 degrees"),
    ] {
        ops.push(Op::Text {
            x,
            y: 25.,
            text: s.into(),
            font,
            size: 11.,
            scale: 1.,
            color: [0.; 3],
        });
    }
    ops.push(Op::Image {
        x: 20.,
        y: 40.,
        w: 100.,
        h: 100.,
        id: "grid".into(),
    });
    ops.push(Op::ImageClip {
        x: 160.,
        y: 40.,
        w: 100.,
        h: 100.,
        id: "grid".into(),
        source: [0.5, 0., 1., 0.5],
    });
    ops.push(Op::TransformBegin {
        matrix: graphics::rotation(300., 40., 100., 100., 90., false, false),
    });
    ops.push(Op::Image {
        x: 300.,
        y: 40.,
        w: 100.,
        h: 100.,
        id: "grid".into(),
    });
    ops.push(Op::TransformEnd);
    let p = [
        (20., 230.),
        (90., 175.),
        (160., 230.),
        (230., 190.),
        (300., 230.),
        (400., 180.),
    ];
    ops.push(Op::Bezier {
        start: p[0],
        segments: graphics::curve(&p, &[1, 1, 1, 1, 1]),
        closed: false,
        width: 2.,
        color: [0.1, 0.2, 0.8],
        fill: None,
    });
    pdf::save(
        Path::new(&args[0]),
        &[Page {
            width: 440.,
            height: 260.,
            ops,
        }],
        &mut f,
        &BTreeMap::from([("grid".into(), data.into_inner())]),
    )?;
    Ok(())
}
