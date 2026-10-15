//! Shared PDF drawing operations, including shaped glyphs and logical text spans.
use serde::Serialize;
#[derive(Clone, Debug, Serialize)]
pub struct PositionedGlyph {
    pub id: u16,
    pub x: f64,
    pub y: f64,
}
#[derive(Clone, Debug, Serialize)]
pub enum Op {
    ActualTextBegin {
        text: String,
    },
    ActualTextEnd,
    Shaped {
        x: f64,
        y: f64,
        text: String,
        font: usize,
        size: f64,
        scale: f64,
        color: [f64; 3],
        glyphs: Vec<PositionedGlyph>,
    },
    TransformBegin {
        matrix: crate::graphics::Matrix,
    },
    TransformEnd,
    Bezier {
        start: (f64, f64),
        segments: Vec<[f64; 6]>,
        closed: bool,
        width: f64,
        color: [f64; 3],
        fill: Option<[f64; 3]>,
    },
    ImageClip {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        id: String,
        source: [f64; 4],
    },
    Path {
        points: Vec<(f64, f64)>,
        closed: bool,
        width: f64,
        color: [f64; 3],
        fill: Option<[f64; 3]>,
    },
    Text {
        x: f64,
        y: f64,
        text: String,
        font: usize,
        size: f64,
        scale: f64,
        color: [f64; 3],
    },
    Line {
        x: f64,
        y: f64,
        x2: f64,
        y2: f64,
        width: f64,
        color: [f64; 3],
    },
    Rect {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        fill: [f64; 3],
    },
    Image {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        id: String,
    },
}
#[derive(Default, Serialize)]
pub struct Page {
    pub width: f64,
    pub height: f64,
    pub ops: Vec<Op>,
}
