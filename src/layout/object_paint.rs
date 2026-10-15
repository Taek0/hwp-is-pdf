//! Layout responsibility: object paint.
use super::*;

impl<'a> Engine<'a> {
    pub(super) fn object(
        &mut self,
        n: &Node,
        ox: f64,
        line_y: f64,
        base: f64,
        anchor: f64,
        container: f64,
    ) -> Result<()> {
        let pos = n.child("pos").context("Missing object position")?;
        let sz = n.child("sz").context("Missing object size")?;
        let mut w = sz.n("width") / 100.;
        let h = sz.n("height") / 100.;
        let m = margin(n, "outMargin");
        let declared_h = h;
        let mut h = effective_height(n);
        let inline = pos.a("treatAsChar") == "1";
        let mut x;

        let mut y = if inline {
            x = anchor + m[0];
            let baseline = if n.name == "equation" {
                n.n("baseLine") / 100.
            } else {
                0.85
            };
            base - declared_h * baseline + m[2]
        } else {
            let hr = pos.a("horzRelTo");
            let vr = pos.a("vertRelTo");
            let extent = if hr == "PAPER" {
                self.pw
            } else if hr == "PAGE" {
                self.pw - self.left - self.right
            } else {
                container
            };
            x = (if hr == "PAPER" {
                0.
            } else if hr == "PAGE" {
                self.left
            } else {
                ox
            }) + pos.signed("horzOffset") / 100.
                + match pos.a("horzAlign") {
                    "CENTER" => (extent - w) / 2.,
                    "RIGHT" => extent - w,
                    _ => 0.,
                };
            let extent = if vr == "PAPER" {
                self.ph
            } else {
                self.ph - self.top - self.bottom
            };
            let origin = if vr == "PAPER" {
                0.
            } else if vr == "PAGE" {
                self.top
            } else {
                line_y
            };
            object_position::vertical(
                origin,
                extent,
                h,
                vr,
                pos.a("vertAlign"),
                pos.signed("vertOffset") / 100.,
            )
        };
        if n.a("reflowCaptions") == "1" {
            x += caption_extent(n)[0];
        }
        // Saved HWPX floating tables position the outside margin box at the
        // anchor. Rebuilt tables already incorporate this spacing in flow.
        if !inline
            && n.name == "tbl"
            && n.a("binaryHwp") != "1"
            && n.a("reflowTable") != "1"
            && n.a("reflowCaptions") != "1"
        {
            x += m[0];
            y += m[2];
        }
        if !inline
            && n.child("caption").is_some()
            && n.a("binaryHwp") == "1"
            && n.a("reflowCaptions") != "1"
        {
            let e = caption_extent(n);
            x += m[0] + e[0];
            y += m[2] + e[2];
            // Position the combined caption/object box inside its containing column.
            x = x
                .max(ox + e[0])
                .min((ox + container - w - e[1]).max(ox + e[0]));
        }
        let fit_start = self.report.bounds.len();
        let fit = if n.n("reflowScale") > 0. && n.n("reflowScale") < 1. {
            let factor = n.n("reflowScale");
            let matrix = [factor, 0., 0., factor, x * (1. - factor), y * (1. - factor)];
            self.current.ops.push(Op::TransformBegin { matrix });
            Some(matrix)
        } else if self.depth == 0
            && n.name != "tbl"
            && n.child("caption").is_none()
            && n.a("textWrap") != "BEHIND_TEXT"
            && (inline || pos.a("vertRelTo") == "PARA")
            && (h > self.ph - self.top - self.bottom || w > self.pw - self.left - self.right)
        {
            let target_y = y.max(self.top).min(self.ph - self.bottom - 1.);
            let factor = ((self.ph - self.bottom - target_y) / h.max(0.1))
                .min(container / w.max(0.1))
                .min(1.);
            let target_x = x.max(ox).min(ox + container - w * factor);
            let matrix = [
                factor,
                0.,
                0.,
                factor,
                target_x - factor * x,
                target_y - factor * y,
            ];
            self.current.ops.push(Op::TransformBegin { matrix });
            self.flow_floor = Some((self.pages.len(), target_y + h * factor, false));
            self.report.warnings.insert(format!(
                "Oversized {} {} fitted proportionally to printable area at {:.1}%",
                n.name,
                n.a("id"),
                factor * 100.
            ));
            Some(matrix)
        } else {
            None
        };
        let mut opaque_pieces = None;
        if n.name == "pic"
            && n.a("alphaParts").is_empty()
            && n.a("textWrap") != "BEHIND_TEXT"
            && let Some(img) = n.child("img")
            && let Some(bytes) = self.doc.binaries.get(img.a("binaryItemIDRef"))
            && let Some(rects) = crate::image_layout::alpha_rects(bytes, n)?
        {
            let angle = n.child("rotationInfo").map(|a| a.n("angle")).unwrap_or(0.);
            let flip = n.child("flip");
            let flipped = flip.is_some_and(|f| f.a("horizontal") == "1" || f.a("vertical") == "1");
            let native = n
                .child("curSz")
                .filter(|s| s.n("width") > 0. && s.n("height") > 0. && (angle != 0. || flipped));
            let nw = native.map(|s| s.n("width") / 100.).unwrap_or(w);
            let nh = native.map(|s| s.n("height") / 100.).unwrap_or(h);
            let matrix = crate::graphics::rotation(
                x,
                y,
                nw,
                nh,
                angle,
                flip.is_some_and(|f| f.a("horizontal") == "1"),
                flip.is_some_and(|f| f.a("vertical") == "1"),
            );
            opaque_pieces = Some(
                rects
                    .iter()
                    .map(|r| {
                        crate::graphics::rect(
                            matrix,
                            [x + r[0] * nw, y + r[1] * nh, x + r[2] * nw, y + r[3] * nh],
                        )
                    })
                    .collect::<Vec<_>>(),
            );
        }
        self.report.bounds.push(Bounds {
            page: self.pages.len() + 1,
            kind: if n.name == "pic" && (!n.a("alphaParts").is_empty() || opaque_pieces.is_some()) {
                "pic-alpha".into()
            } else if n.a("textWrap") == "BEHIND_TEXT" && n.name == "pic" {
                "watermark".into()
            } else {
                n.name.clone()
            },
            id: n.a("id").into(),
            x,
            y,
            w,
            h,
        });
        if !n.a("alphaParts").is_empty() {
            let parts: Vec<Vec<(f64, f64)>> = serde_json::from_str(n.a("alphaParts"))?;
            for (i, part) in parts.iter().enumerate() {
                let x0 = part.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
                let x1 = part.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
                let y0 = part.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
                let y1 = part.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
                self.report.bounds.push(Bounds {
                    page: self.pages.len() + 1,
                    kind: "pic".into(),
                    id: format!("{}-alpha-{i}", n.a("id")),
                    x: ox + x0,
                    y: line_y + y0,
                    w: x1 - x0,
                    h: y1 - y0,
                });
            }
        }
        if let Some(parts) = opaque_pieces {
            for (i, b) in parts.into_iter().enumerate() {
                self.report.bounds.push(Bounds {
                    page: self.pages.len() + 1,
                    kind: "pic".into(),
                    id: format!("{}-alpha-{i}", n.a("id")),
                    x: b[0],
                    y: b[1],
                    w: b[2] - b[0],
                    h: b[3] - b[1],
                });
            }
        }
        let behind = n.a("textWrap") == "BEHIND_TEXT";
        let paint_start = self.current.ops.len();
        let bound_start = self.report.bounds.len();
        let angle = n.child("rotationInfo").map(|r| r.n("angle")).unwrap_or(0.);
        let flip = n.child("flip");
        let flip_x = flip.is_some_and(|f| f.a("horizontal") == "1");
        let flip_y = flip.is_some_and(|f| f.a("vertical") == "1");
        let transform = if angle.rem_euclid(360.).abs() > 0.001 || flip_x || flip_y {
            if let Some(size) = n
                .child("curSz")
                .filter(|s| s.n("width") > 0. && s.n("height") > 0.)
            {
                w = size.n("width") / 100.;
                h = size.n("height") / 100.;
            }
            let m = crate::graphics::rotation(x, y, w, h, angle, flip_x, flip_y);
            self.current.ops.push(Op::TransformBegin { matrix: m });
            Some(m)
        } else {
            None
        };
        let mut captions_handled = false;
        match n.name.as_str() {
            "chart" => {
                let (ops, warning) = crate::chart::draw(n, x, y, w, h, &mut self.fonts)?;
                self.current.ops.extend(ops);
                self.report.charts += 1;
                if let Some(w) = warning {
                    self.report.warnings.insert(w);
                }
                if n.a("axisEvidence") == "Inconclusive" {
                    self.report.warnings.insert("Chart series orientation uses writer convention; axis evidence is inconclusive".into());
                }
            }
            "group" => {
                anyhow::ensure!(self.depth < 64, "Excessive nested group layout");
                let fitted = n.n("reflowScale") > 0. && n.n("reflowScale") < 1.;
                if fitted {
                    self.depth += 1;
                }
                let result = (|| -> Result<()> {
                    for child in n.children.iter().filter(|c| c.child("pos").is_some()) {
                        self.object(child, x, y, y, x, w)?;
                    }
                    Ok(())
                })();
                if fitted {
                    self.depth -= 1;
                }
                result?;
            }
            "polygon" | "curve" | "arc" => self.path_shape(n, x, y, w, h)?,
            "equation" => {
                self.paint_equation(n, x, y, w, h)?;
            }
            "pic" => {
                let img = n.child("img").context("Missing image reference")?;
                let id = img.a("binaryItemIDRef").to_string();
                if self.doc.binaries.contains_key(&id) {
                    let crop = n
                        .child("imgClip")
                        .filter(|c| c.n("right") > c.n("left") && c.n("bottom") > c.n("top"));
                    let source = if let Some(c) = crop {
                        let dim = n.child("imgDim");
                        let (dw, dh) =
                            if dim.is_some_and(|d| d.n("dimwidth") > 0. && d.n("dimheight") > 0.) {
                                (dim.unwrap().n("dimwidth"), dim.unwrap().n("dimheight"))
                            } else {
                                let image = image::load_from_memory(&self.doc.binaries[&id])?;
                                (image.width() as f64 * 75., image.height() as f64 * 75.)
                            };
                        let mut s = [
                            c.n("left") / dw,
                            c.n("top") / dh,
                            c.n("right") / dw,
                            c.n("bottom") / dh,
                        ];
                        // Old records quantize the 96-DPI dimensions; only absorb
                        // small overshoot in inferred full-image edges, never crop loss.
                        if dim.is_none_or(|d| d.n("dimwidth") <= 0. || d.n("dimheight") <= 0.) {
                            for i in [2, 3] {
                                if s[i] > 1. && s[i] < 1.01 {
                                    s[i] = 1.;
                                }
                            }
                        }
                        Some(s)
                    } else {
                        None
                    };
                    if let Some(s) = source.filter(|s| {
                        s[0].abs() > 0.000001
                            || s[1].abs() > 0.000001
                            || (s[2] - 1.).abs() > 0.000001
                            || (s[3] - 1.).abs() > 0.000001
                    }) {
                        if n.child("imgDim")
                            .is_none_or(|d| d.n("dimwidth") <= 0. || d.n("dimheight") <= 0.)
                        {
                            self.report.warnings.insert("Legacy picture crop uses 96-DPI pixel-to-HWPUNIT fallback (imgDim absent)".into());
                        }
                        self.current.ops.push(Op::ImageClip {
                            x,
                            y,
                            w,
                            h,
                            id,
                            source: s,
                        });
                    } else {
                        self.current.ops.push(Op::Image { x, y, w, h, id });
                    }
                    self.report.pictures += 1
                } else {
                    bail!("Missing binary {id}")
                }
            }
            "tbl" => {
                let previous_page = self.pages.len();
                if self.depth == 0 {
                    self.split_caption_handled = false;
                }
                self.table(n, x, y, w, h)?;
                captions_handled = self.depth == 0 && self.split_caption_handled;
                if self.pages.len() > previous_page && n.child("caption").is_some() {
                    y = self.top;
                    h = self.last_table_end - self.top;
                }
                if self.depth == 0
                    && pos.a("vertRelTo") == "PARA"
                    && pos.a("vertAlign") != "BOTTOM"
                    && (self.pages.len() > previous_page || self.last_table_end > y + h + 0.5)
                {
                    self.flow_floor = Some((
                        self.pages.len(),
                        self.last_table_end
                            + if captions_handled {
                                0.
                            } else {
                                caption_extent(n)[3]
                            },
                        self.pages.len() > previous_page,
                    ));
                }
                self.report.tables += 1;
            }
            "line" => {
                let s = n.child("lineShape");
                self.current.ops.push(Op::Line {
                    x,
                    y,
                    x2: x + w,
                    y2: y + h,
                    width: val(s, "width").max(0.1),
                    color: color(s.map(|n| n.a("color")).unwrap_or("#000000")),
                });
            }
            "ellipse" => {
                let s = n.child("lineShape");
                let stroke = color(s.map(|n| n.a("color")).unwrap_or("#000000"));
                for i in 0..64 {
                    let a = i as f64 * std::f64::consts::TAU / 64.;
                    let b = (i + 1) as f64 * std::f64::consts::TAU / 64.;
                    self.current.ops.push(Op::Line {
                        x: x + w / 2. + w / 2. * a.cos(),
                        y: y + h / 2. + h / 2. * a.sin(),
                        x2: x + w / 2. + w / 2. * b.cos(),
                        y2: y + h / 2. + h / 2. * b.sin(),
                        width: val(s, "width").max(0.1),
                        color: stroke,
                    });
                }
            }
            "rect" => {
                self.paint_rectangle(n, x, y, w, h);
                if let Some(draw) = n.child("drawText")
                    && let Some(list) = draw.child("subList")
                {
                    let tm = margin(draw, "textMargin");
                    self.local(list, x + tm[0], y + tm[2], w - tm[0] - tm[1])?;
                }
            }
            _ => {}
        }
        if let Some(m) = transform {
            self.current.ops.push(Op::TransformEnd);
            for b in &mut self.report.bounds[bound_start..] {
                let r = crate::graphics::rect(m, [b.x, b.y, b.x + b.w, b.y + b.h]);
                b.x = r[0];
                b.y = r[1];
                b.w = r[2] - r[0];
                b.h = r[3] - r[1];
            }
        }
        if !captions_handled {
            self.paint_captions(n, x, y, w, h)?;
            if n.name == "tbl" && n.a("reflowCaptions") == "1" {
                self.last_table_end = self.last_table_end.max(y + side_caption_height(n));
            }
        }
        if let Some(matrix) = fit {
            self.current.ops.push(Op::TransformEnd);
            for b in &mut self.report.bounds[fit_start..] {
                let r = crate::graphics::rect(matrix, [b.x, b.y, b.x + b.w, b.y + b.h]);
                b.x = r[0];
                b.y = r[1];
                b.w = r[2] - r[0];
                b.h = r[3] - r[1];
            }
        }
        if behind {
            let paint = self.current.ops.drain(paint_start..).collect::<Vec<_>>();
            self.current.ops.splice(0..0, paint);
        }
        Ok(())
    }
    pub(super) fn box_lines(&mut self, x: f64, y: f64, w: f64, h: f64, width: f64, c: [f64; 3]) {
        for (a, b, aa, bb) in [
            (x, y, x + w, y),
            (x, y + h, x + w, y + h),
            (x, y, x, y + h),
            (x + w, y, x + w, y + h),
        ] {
            self.current.ops.push(Op::Line {
                x: a,
                y: b,
                x2: aa,
                y2: bb,
                width,
                color: c,
            });
        }
    }
    pub(super) fn border(&mut self, id: &str, x: f64, y: f64, w: f64, h: f64) {
        if let Some(b) = self.doc.borders.get(id) {
            if let Some(fill) = b.find("winBrush")
                && fill.a("faceColor") != "none"
            {
                self.current.ops.push(Op::Rect {
                    x,
                    y,
                    w,
                    h,
                    fill: color(fill.a("faceColor")),
                });
            }
            for (name, (a, bb, c, d)) in [
                ("leftBorder", (x, y, x, y + h)),
                ("rightBorder", (x + w, y, x + w, y + h)),
                ("topBorder", (x, y, x + w, y)),
                ("bottomBorder", (x, y + h, x + w, y + h)),
            ] {
                if let Some(s) = b.child(name)
                    && s.a("type") != "NONE"
                {
                    let mm = s
                        .a("width")
                        .split_whitespace()
                        .next()
                        .unwrap_or("0.1")
                        .parse::<f64>()
                        .unwrap_or(0.1);
                    self.current.ops.push(Op::Line {
                        x: a,
                        y: bb,
                        x2: c,
                        y2: d,
                        width: mm * 72. / 25.4,
                        color: color(s.a("color")),
                    });
                }
            }
        }
    }
}
