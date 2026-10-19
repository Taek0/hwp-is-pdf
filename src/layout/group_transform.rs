//! Render normalized HWPX group children in their affine coordinate frames.
use super::*;
impl Engine<'_> {
    pub(super) fn hwpx_group(&mut self, n: &Node, x: f64, y: f64, w: f64, h: f64) -> Result<()> {
        anyhow::ensure!(self.depth < 64, "Excessive group nesting");
        let (gw, gh) = (n.n("groupWidth") / 100., n.n("groupHeight") / 100.);
        anyhow::ensure!(gw > 0. && gh > 0., "Missing group coordinate frame");
        let outer = [w / gw, 0., 0., h / gh, x, y];
        for child in n.children.iter().filter(|c| !c.a("groupMatrix").is_empty()) {
            let matrix: crate::graphics::Matrix = serde_json::from_str(child.a("groupMatrix"))?;
            let matrix = crate::graphics::compose(outer, matrix);
            let begin = self.report.bounds.len();
            self.current.ops.push(Op::TransformBegin { matrix });
            self.depth += 1;
            let result = self.object(child, 0., 0., 0., 0., val(child.child("sz"), "width"));
            self.depth -= 1;
            self.current.ops.push(Op::TransformEnd);
            result?;
            for b in &mut self.report.bounds[begin..] {
                let r = crate::graphics::rect(matrix, [b.x, b.y, b.x + b.w, b.y + b.h]);
                b.x = r[0];
                b.y = r[1];
                b.w = r[2] - r[0];
                b.h = r[3] - r[1];
            }
        }
        Ok(())
    }
}
