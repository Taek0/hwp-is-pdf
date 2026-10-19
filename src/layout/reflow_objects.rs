//! Layout responsibility: reflow objects.
use super::*;

impl<'a> Engine<'a> {
    pub(super) fn reflow_children(
        &mut self,
        p: &mut Node,
        width: f64,
        nesting: usize,
    ) -> Result<()> {
        anyhow::ensure!(nesting < 64, "Excessive nested reflow");
        let missing_cache = lines(p).is_empty();
        for run in p.children.iter_mut().filter(|r| r.name == "run") {
            for n in &mut run.children {
                match n.name.as_str() {
                    "equation" => {
                        if missing_cache {
                            n.attrs.insert("measureEquation".into(), "1".into());
                        }
                        self.prepare_equation(n, width)?;
                    }
                    "tbl" | "pic" | "rect" | "ellipse" | "polygon" | "curve" | "group" | "line"
                    | "arc" | "chart" | "checkBtn" => {
                        let behind = Self::nonflow_object(n);
                        anyhow::ensure!(
                            !behind || (n.name != "tbl" && n.child("caption").is_none()),
                            "Background tables/captions require separate placement"
                        );
                        let width = if behind {
                            val(n.child("sz"), "width").max(1.)
                        } else {
                            self.caption_body_width(n, width)?
                        };
                        let block = !behind
                            && (n.name == "tbl"
                                || n.child("pos").is_none_or(|p| p.a("treatAsChar") != "1"));
                        if n.name == "tbl" {
                            self.reflow_table_contents(n, width, nesting)?;
                        } else {
                            if n.name == "group" {
                                let mut para = Node {
                                    name: "p".into(),
                                    ..Default::default()
                                };
                                let mut run = Node {
                                    name: "run".into(),
                                    ..Default::default()
                                };
                                for child in n.children.iter().filter(|c| c.child("pos").is_some())
                                {
                                    let mut child = child.clone();
                                    child.attrs.insert("reflowFloat".into(), "1".into());
                                    run.children.push(child);
                                }
                                para.children.push(run);
                                self.reflow_children(
                                    &mut para,
                                    val(n.child("sz"), "width"),
                                    nesting + 1,
                                )?;
                                let mut prepared = para.children.remove(0).children.into_iter();
                                for child in
                                    n.children.iter_mut().filter(|c| c.child("pos").is_some())
                                {
                                    *child = prepared.next().unwrap();
                                }
                            }
                            let old_w = val(n.child("sz"), "width");
                            let ratio = if n.name == "group" || behind {
                                1.
                            } else {
                                let horizontal = (width / old_w.max(0.1)).min(1.);
                                if matches!(n.name.as_str(), "line" | "arc" | "chart") {
                                    horizontal.min(
                                        (self.ph - self.top - self.bottom - 24.)
                                            / effective_height(n).max(1.),
                                    )
                                } else {
                                    horizontal
                                }
                            };
                            for key in ["sz", "curSz"] {
                                if let Some(sz) = n.children.iter_mut().find(|s| s.name == key) {
                                    for dimension in ["width", "height"] {
                                        let value = sz.n(dimension) * ratio;
                                        sz.attrs.insert(dimension.into(), value.to_string());
                                    }
                                }
                            }
                            if n.name == "rect" {
                                let angle = crate::graphics::source_angle(n);
                                let native_w = if angle != 0. {
                                    val(n.child("curSz"), "width")
                                } else {
                                    val(n.child("sz"), "width")
                                };
                                let native_h = if angle != 0. {
                                    val(n.child("curSz"), "height")
                                } else {
                                    val(n.child("sz"), "height")
                                };
                                let mut height = native_h;
                                if let Some(draw) =
                                    n.children.iter_mut().find(|n| n.name == "drawText")
                                {
                                    let m = margin(draw, "textMargin");
                                    if let Some(list) =
                                        draw.children.iter_mut().find(|n| n.name == "subList")
                                    {
                                        height = height.max(
                                            self.reflow_list(
                                                list,
                                                native_w - m[0] - m[1],
                                                nesting + 1,
                                            )? + m[2]
                                                + m[3],
                                        );
                                    }
                                }
                                if angle != 0. {
                                    let rad = angle.to_radians();
                                    node_mut(n, "curSz")
                                        .attrs
                                        .insert("height".into(), (height * 100.).to_string());
                                    let sz = node_mut(n, "sz");
                                    sz.attrs.insert(
                                        "width".into(),
                                        ((native_w * rad.cos().abs() + height * rad.sin().abs())
                                            * 100.)
                                            .to_string(),
                                    );
                                    sz.attrs.insert(
                                        "height".into(),
                                        ((native_w * rad.sin().abs() + height * rad.cos().abs())
                                            * 100.)
                                            .to_string(),
                                    );
                                } else {
                                    node_mut(n, "sz")
                                        .attrs
                                        .insert("height".into(), (height * 100.).to_string());
                                }
                            }
                        }
                        if block && n.a("reflowFloat") != "1" {
                            n.attrs.insert("textWrap".into(), "TOP_AND_BOTTOM".into());
                            let pos = node_mut(n, "pos");
                            for (k, v) in [
                                ("treatAsChar", "0"),
                                ("horzRelTo", "PARA"),
                                ("vertRelTo", "PARA"),
                                ("horzAlign", "LEFT"),
                                ("vertAlign", "TOP"),
                                ("horzOffset", "0"),
                                ("vertOffset", "0"),
                            ] {
                                pos.attrs.insert(k.into(), v.into());
                            }
                        }
                        self.prepare_captions(n, nesting)?;
                    }
                    "ctrl" => {
                        for control in &mut n.children {
                            if control.name == "endNote" {
                                control.attrs.insert("reflowEndnote".into(), "1".into());
                            }
                            if control.name == "footNote" {
                                let (_, mut w) =
                                    self.footnote_area(self.column_x(self.reflow_column));
                                if self
                                    .note_spec(false)
                                    .and_then(|s| s.child("placement"))
                                    .is_none_or(|p| p.a("place") == "EACH_COLUMN")
                                {
                                    w = self.column_widths.iter().copied().fold(w, f64::min);
                                }
                                if let Some(list) =
                                    control.children.iter_mut().find(|n| n.name == "subList")
                                {
                                    self.reflow_list(list, w, nesting + 1)?;
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
    pub(super) fn nonflow_object(n: &Node) -> bool {
        n.a("textWrap") == "BEHIND_TEXT"
            || (n.a("textWrap") == "IN_FRONT_OF_TEXT"
                && n.child("pos").is_some_and(|p| p.a("treatAsChar") != "1"))
    }
    pub(super) fn general_objects(items: &[Item]) -> bool {
        let objects: Vec<_> = items
            .iter()
            .filter_map(|i| match &i.kind {
                ItemKind::Object(n) if !Self::nonflow_object(n) => Some((i.pos, n)),
                _ => None,
            })
            .collect();
        objects.iter().all(|(_, n)| {
            matches!(
                n.name.as_str(),
                "pic" | "rect" | "ellipse" | "polygon" | "curve" | "group"
            ) && (n.a("textWrap") == "SQUARE"
                || (n.a("textWrap") == "TOP_AND_BOTTOM"
                    && n.child("pos").is_some_and(|p| {
                        matches!(p.a("horzRelTo"), "PAGE" | "PAPER")
                            || matches!(p.a("vertRelTo"), "PAGE" | "PAPER")
                    })))
                && n.child("caption").is_none()
                && n.child("pos").is_some_and(|p| {
                    p.a("treatAsChar") != "1"
                        && matches!(p.a("horzRelTo"), "PARA" | "COLUMN" | "PAGE" | "PAPER")
                        && matches!(p.a("vertRelTo"), "PARA" | "PAGE" | "PAPER")
                        && (matches!(p.a("horzRelTo"), "PAGE" | "PAPER")
                            || matches!(p.a("vertRelTo"), "PAGE" | "PAPER")
                            || matches!(p.a("vertAlign"), "" | "TOP"))
                })
        })
    }
    pub(super) fn position_region(
        &mut self,
        n: &mut Node,
        width: f64,
        existing: &[crate::wrap_region::Region],
    ) -> Result<crate::wrap_region::Region> {
        let outer_margin = margin(n, "outMargin").map(|v| v.max(0.));
        let angle = crate::graphics::source_angle(n);
        let mut w = val(n.child("sz"), "width");
        let mut h = effective_height(n);
        let native = n
            .child("curSz")
            .filter(|s| s.n("width") > 0. && s.n("height") > 0.);
        let nw = native.map(|s| s.n("width") / 100.).unwrap_or(w);
        let nh = native.map(|s| s.n("height") / 100.).unwrap_or(h);
        if angle != 0. {
            node_mut(n, "curSz")
                .attrs
                .insert("width".into(), (nw * 100.).to_string());
            node_mut(n, "curSz")
                .attrs
                .insert("height".into(), (nh * 100.).to_string());
            let m = crate::graphics::rotation(0., 0., nw, nh, angle, false, false);
            let b = crate::graphics::rect(m, [0., 0., nw, nh]);
            w = b[2] - b[0];
            h = b[3] - b[1];
            node_mut(n, "sz")
                .attrs
                .insert("width".into(), (w * 100.).to_string());
            node_mut(n, "sz")
                .attrs
                .insert("height".into(), (h * 100.).to_string());
        }
        let absolute = n.a("pageAbsolute") == "1";
        let frame_width = if absolute { self.pw } else { width };
        let offset = n
            .child("pos")
            .map(|p| p.signed("vertOffset") / 100.)
            .unwrap_or(0.)
            + outer_margin[2];
        let max_height = if n.n("reflowMaxHeight") > 0. {
            n.n("reflowMaxHeight")
        } else {
            self.ph - self.top - self.bottom - 32.
        };
        let scale = if absolute {
            n.a("reflowScale")
                .parse::<f64>()
                .unwrap_or(1.)
                .clamp(0.001, 1.)
        } else if n.a("groupMember") == "1" {
            1.
        } else {
            (frame_width / w.max(0.01))
                .min((max_height - offset.max(0.)).max(24.) / h.max(0.01))
                .min(1.)
        };
        if scale < 1. {
            n.attrs.insert("reflowScale".into(), scale.to_string());
            self.report.warnings.insert(format!(
                "Float {} fitted to usable area at {:.1}%",
                n.a("id"),
                scale * 100.
            ));
            w *= scale;
            h *= scale;
        }
        let pos = n.child("pos").unwrap();
        let allow_overlap = pos.a("allowOverlap") == "1";
        let mut x = pos.signed("horzOffset") / 100.
            + match pos.a("horzAlign") {
                "CENTER" => (width - w) / 2.,
                "RIGHT" => width - w - outer_margin[1],
                _ => outer_margin[0],
            };
        let mut y = pos.signed("vertOffset") / 100. + outer_margin[2];
        anyhow::ensure!(
            y >= 0. || n.a("reflowNegativeAllowed") == "1",
            "Negative vertical float anchors require saved-layout conversion"
        );
        anyhow::ensure!(
            w <= frame_width + 0.01,
            "Rotated float is wider than its frame"
        );
        let bounded = if absolute {
            x.clamp(-self.left, (self.pw - self.left - w).max(-self.left))
        } else {
            x.clamp(0., (width - w).max(0.))
        };
        if (bounded - x).abs() > 0.01 {
            self.report
                .warnings
                .insert("Float horizontal position clamped to printable column".into());
            x = bounded;
        }
        let mut points = if self.contour_wrap && n.name == "ellipse" {
            (0..128)
                .map(|i| {
                    let a = i as f64 * std::f64::consts::TAU / 128.;
                    (nw / 2. + nw / 2. * a.cos(), nh / 2. + nh / 2. * a.sin())
                })
                .collect::<Vec<_>>()
        } else if self.contour_wrap && matches!(n.name.as_str(), "polygon" | "curve") {
            let ow = if n.n("originalWidth") > 0. {
                n.n("originalWidth") / 100.
            } else {
                let original = val(n.child("orgSz"), "width");
                if original > 0. { original } else { nw }
            };
            let oh = if n.n("originalHeight") > 0. {
                n.n("originalHeight") / 100.
            } else {
                let original = val(n.child("orgSz"), "height");
                if original > 0. { original } else { nh }
            };
            let mut points = n
                .children
                .iter()
                .filter(|p| matches!(p.name.as_str(), "pt" | "point"))
                .map(|p| {
                    (
                        p.n("x") / 100. * nw / ow.max(0.01),
                        p.n("y") / 100. * nh / oh.max(0.01),
                    )
                })
                .collect::<Vec<_>>();
            if n.name == "curve" {
                let mut kinds: Vec<_> = n.children_named("segment").map(|s| s.u("type")).collect();
                if points.is_empty() {
                    for seg in n.children_named("seg") {
                        if points.is_empty() {
                            points.push((
                                seg.n("x1") / 100. * nw / ow.max(0.01),
                                seg.n("y1") / 100. * nh / oh.max(0.01),
                            ));
                        }
                        points.push((
                            seg.n("x2") / 100. * nw / ow.max(0.01),
                            seg.n("y2") / 100. * nh / oh.max(0.01),
                        ));
                        kinds.push(usize::from(seg.a("type") == "CURVE"));
                    }
                }
                anyhow::ensure!(
                    points.len() >= 2 && points.len() <= 1024,
                    "Unsupported curve contour"
                );
                points = crate::graphics::flatten_cubics(
                    points[0],
                    &crate::graphics::curve(&points, &kinds),
                );
            }
            anyhow::ensure!(
                points.len() >= 3 && points.len() <= 65536,
                "Unsupported polygon contour"
            );
            points
        } else {
            vec![(0., 0.), (nw, 0.), (nw, nh), (0., nh)]
        };
        if self.contour_wrap && n.name != "pic" {
            let flip = n.child("flip");
            let m = crate::graphics::rotation(
                0.,
                0.,
                nw,
                nh,
                angle,
                flip.is_some_and(|f| f.a("horizontal") == "1"),
                flip.is_some_and(|f| f.a("vertical") == "1"),
            );
            for p in &mut points {
                *p = crate::graphics::point(m, p.0, p.1);
                p.0 *= scale;
                p.1 *= scale;
            }
        } else {
            points = vec![(0., 0.), (w, 0.), (w, h), (0., h)];
        }
        for p in &mut points {
            p.0 += x;
            p.1 += y;
        }
        let clearance = 1. + val(n.child("lineShape"), "width") / 2.;
        let mut region = crate::wrap_region::Region {
            parts: None,
            points,
            margin: outer_margin.map(|m| m + clearance),
            flow: n.a("textFlow").into(),
        };
        if self.contour_wrap
            && n.name == "pic"
            && let Some(img) = n.child("img")
            && let Some(bytes) = self.doc.binaries.get(img.a("binaryItemIDRef"))
            && let Some(rects) = crate::image_layout::alpha_rects(bytes, n)?
        {
            let flip = n.child("flip");
            let matrix = crate::graphics::rotation(
                0.,
                0.,
                nw,
                nh,
                angle,
                flip.is_some_and(|f| f.a("horizontal") == "1"),
                flip.is_some_and(|f| f.a("vertical") == "1"),
            );
            region.parts = Some(
                rects
                    .iter()
                    .map(|r| {
                        let b = crate::graphics::rect(
                            matrix,
                            [r[0] * nw, r[1] * nh, r[2] * nw, r[3] * nh],
                        );
                        vec![
                            (x + b[0] * scale, y + b[1] * scale),
                            (x + b[2] * scale, y + b[1] * scale),
                            (x + b[2] * scale, y + b[3] * scale),
                            (x + b[0] * scale, y + b[3] * scale),
                        ]
                    })
                    .collect(),
            );
        }
        if self.contour_wrap && n.name == "group" {
            let flip = n.child("flip");
            let matrix = crate::graphics::rotation(
                0.,
                0.,
                nw,
                nh,
                angle,
                flip.is_some_and(|f| f.a("horizontal") == "1"),
                flip.is_some_and(|f| f.a("vertical") == "1"),
            );
            let mut parts = Vec::new();
            for child in n.children.iter_mut().filter(|c| c.child("pos").is_some()) {
                let cp = child.child("pos").unwrap();
                anyhow::ensure!(
                    cp.a("treatAsChar") != "1"
                        && matches!(cp.a("vertAlign"), "" | "TOP")
                        && !matches!(cp.a("vertRelTo"), "PAPER" | "PAGE"),
                    "Group contour requires relative top anchors"
                );
                let original_margin = child.child("outMargin").cloned();
                let child_margin = margin(child, "outMargin").map(|v| v.max(0.));
                let mut prepared = child.clone();
                prepared.children.retain(|c| c.name != "outMargin");
                prepared.attrs.insert("groupMember".into(), "1".into());
                let sub = self.position_region(&mut prepared, nw, &[])?;
                for part in sub
                    .parts
                    .clone()
                    .unwrap_or_else(|| vec![sub.points.clone()])
                {
                    let a =
                        part.iter().map(|p| p.0).fold(f64::INFINITY, f64::min) - child_margin[0];
                    let b = part.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max)
                        + child_margin[1];
                    let c =
                        part.iter().map(|p| p.1).fold(f64::INFINITY, f64::min) - child_margin[2];
                    let d = part.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max)
                        + child_margin[3];
                    let r = crate::graphics::rect(matrix, [a, c, b, d]);
                    parts.push(vec![
                        (x + r[0] * scale, y + r[1] * scale),
                        (x + r[2] * scale, y + r[1] * scale),
                        (x + r[2] * scale, y + r[3] * scale),
                        (x + r[0] * scale, y + r[3] * scale),
                    ]);
                }
                if let Some(m) = original_margin {
                    prepared.children.push(m);
                }
                *child = prepared;
            }
            anyhow::ensure!(
                !parts.is_empty(),
                "Group contour has no normalized child shapes"
            );
            region.parts = Some(parts);
        }
        if !allow_overlap {
            for _ in 0..=existing.len() {
                let a = region.bounds();
                let below = existing
                    .iter()
                    .map(|r| r.bounds())
                    .filter(|b| a[0] < b[2] && a[2] > b[0] && a[1] < b[3] && a[3] > b[1])
                    .map(|b| b[3] + region.margin[2])
                    .fold(y, f64::max);
                if below <= y + 0.01 {
                    break;
                }
                region.shift_y(below - y);
                y = below;
                self.report.warnings.insert("Overlapping float anchors separated vertically because allowOverlap is disabled".into());
            }
        }
        if n.name == "pic"
            && scale == 1.
            && let Some(parts) = &region.parts
        {
            n.attrs
                .insert("alphaParts".into(), serde_json::to_string(parts)?);
        }
        let pos = node_mut(n, "pos");
        for (k, v) in [
            ("horzAlign", "LEFT"),
            ("horzRelTo", "PARA"),
            ("vertRelTo", "PARA"),
            ("vertAlign", "TOP"),
        ] {
            pos.attrs.insert(k.into(), v.into());
        }
        pos.attrs
            .insert("horzOffset".into(), (x * 100.).to_string());
        pos.attrs
            .insert("vertOffset".into(), (y * 100.).to_string());
        Ok(region)
    }
}
