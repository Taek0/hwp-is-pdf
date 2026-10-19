//! Non-text glyph silhouettes for shadows, preserving one searchable text copy.
use std::fmt::Write;
#[derive(Default)]
struct Outline {
    path: String,
    at: (f32, f32),
}
impl ttf_parser::OutlineBuilder for Outline {
    fn move_to(&mut self, x: f32, y: f32) {
        writeln!(self.path, "{x} {y} m").unwrap();
        self.at = (x, y);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        writeln!(self.path, "{x} {y} l").unwrap();
        self.at = (x, y);
    }
    fn quad_to(&mut self, x: f32, y: f32, xx: f32, yy: f32) {
        let a = (
            self.at.0 + 2. / 3. * (x - self.at.0),
            self.at.1 + 2. / 3. * (y - self.at.1),
        );
        let b = (xx + 2. / 3. * (x - xx), yy + 2. / 3. * (y - yy));
        self.curve_to(a.0, a.1, b.0, b.1, xx, yy);
    }
    fn curve_to(&mut self, x: f32, y: f32, xx: f32, yy: f32, xxx: f32, yyy: f32) {
        writeln!(self.path, "{x} {y} {xx} {yy} {xxx} {yyy} c").unwrap();
        self.at = (xxx, yyy);
    }
    fn close(&mut self) {
        self.path.push_str("h\n");
    }
}
pub(super) fn path(face: &ttf_parser::Face<'_>, glyph: u16) -> String {
    let mut outline = Outline::default();
    face.outline_glyph(ttf_parser::GlyphId(glyph), &mut outline);
    outline.path
}
