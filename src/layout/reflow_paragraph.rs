//! Layout responsibility: reflow paragraph.
use super::*;

impl<'a> Engine<'a> {
    pub(super) fn reflow_list(
        &mut self,
        list: &mut Node,
        width: f64,
        nesting: usize,
    ) -> Result<f64> {
        let mut y = 0.;
        for child in &mut list.children {
            if child.name != "p" {
                continue;
            }
            let (mut p, ls, s) = self.reflow_paragraph(child, width, nesting)?;
            y += s.before;
            for seg in &mut node_mut(&mut p, "linesegarray").children {
                seg.attrs
                    .insert("vertpos".into(), (seg.n("vertpos") + y * 100.).to_string());
            }
            y += ls.iter().map(|l| l.height).sum::<f64>() + s.after;
            *child = p;
        }
        Ok(y)
    }
    pub(super) fn reflow_paragraph(
        &mut self,
        source: &Node,
        width: f64,
        nesting: usize,
    ) -> Result<(Node, Vec<crate::reflow::Line>, crate::reflow::Settings)> {
        anyhow::ensure!(nesting < 64, "Excessive nested reflow");
        let style = self.doc.paras.get(source.a("paraPrIDRef"));
        anyhow::ensure!(
            style.is_none_or(|s| s.a("textDir") != "RTL"),
            "RTL paragraph reflow is not supported"
        );
        let mut settings = crate::reflow::settings(style);
        let continuation = source.u("reflowStart");
        if continuation > 0 {
            settings.before = 0.;
            settings.indent = 0.;
            settings.page = false;
        }
        let numbered = style
            .and_then(|s| s.child("heading"))
            .is_some_and(|h| matches!(h.a("type"), "NUMBER" | "BULLET"));
        let mut p = source.clone();
        let number = self.paragraph_number(source, true)?;
        anyhow::ensure!(
            !numbered || number.is_some(),
            "Missing numbering/bullet definition for paragraph {}",
            source.a("id")
        );
        if let Some((_, _, reserved)) = &number {
            let anchor = (settings.left + settings.indent).max(0.);
            p.attrs.insert("reflowNumberX".into(), anchor.to_string());
            settings.left = settings.left.max(anchor + reserved);
            settings.indent = 0.;
        }
        p.attrs.insert("reflowed".into(), "1".into());
        let source_items = Self::items(&p);
        let objects: Vec<_> = source_items
            .iter()
            .filter_map(|i| {
                if let ItemKind::Object(n) = &i.kind {
                    Some((i.pos, n))
                } else {
                    None
                }
            })
            .collect();
        if !self.general_wrap
            && nesting == 0
            && number.is_none()
            && !objects.is_empty()
            && objects.len() <= 2
        {
            let sides: BTreeSet<_> = objects
                .iter()
                .map(|(_, o)| o.child("pos").map(|p| p.a("horzAlign")).unwrap_or(""))
                .collect();
            let eligible = sides.len() == objects.len()
                && objects.iter().all(|(_, object)| {
                    matches!(object.name.as_str(), "pic" | "rect")
                        && object.a("textWrap") == "SQUARE"
                        && object.child("caption").is_none()
                        && object.child("pos").is_some_and(|p| {
                            p.a("treatAsChar") != "1"
                                && p.a("vertRelTo") == "PARA"
                                && matches!(p.a("horzRelTo"), "PARA" | "COLUMN")
                                && matches!(p.a("horzAlign"), "LEFT" | "RIGHT")
                                && matches!(p.a("vertAlign"), "" | "TOP")
                                && p.signed("horzOffset") == 0.
                                && p.signed("vertOffset") == 0.
                        })
                })
                && source_items
                    .iter()
                    .filter(|i| i.pos < objects.last().unwrap().0)
                    .all(|i| matches!(i.kind, ItemKind::Control(_) | ItemKind::Object(_)));
            if eligible {
                for run in p.children.iter_mut().filter(|r| r.name == "run") {
                    for n in &mut run.children {
                        if objects
                            .iter()
                            .any(|(_, object)| n.a("id") == object.a("id") && n.name == object.name)
                        {
                            n.attrs.insert("reflowFloat".into(), "1".into());
                        }
                    }
                }
            }
        }
        if self.general_wrap && nesting == 0 && number.is_none() && !objects.is_empty() {
            anyhow::ensure!(
                objects.len() <= 32,
                "Too many anchored objects in a paragraph"
            );
            let eligible = Self::general_objects(&source_items);
            if eligible {
                for run in p.children.iter_mut().filter(|r| r.name == "run") {
                    for n in &mut run.children {
                        if objects
                            .iter()
                            .any(|(_, o)| o.a("id") == n.a("id") && o.name == n.name)
                        {
                            n.attrs.insert("reflowFloat".into(), "1".into());
                            n.attrs.insert("reflowGeneral".into(), "1".into());
                        }
                    }
                }
            }
        }
        self.reflow_children(&mut p, width, nesting + 1)?;
        if self.general_wrap && nesting == 0 && number.is_none() {
            self.prepare_reflow_anchors(&mut p, width, &mut settings)?;
        }
        fn normalize(n: &mut Node) {
            if n.name == "lineBreak" {
                n.name = "#text".into();
                n.text = "\n".into();
                n.children.clear();
            }
            if n.name == "tab" && n.n("width") <= 0. {
                n.attrs.insert("width".into(), "2400".into());
            }
            for c in &mut n.children {
                normalize(c)
            }
        }
        normalize(&mut p);
        let mut exclusions: Vec<(f64, f64, f64)> = if source.a("floatCarryRegions").is_empty() {
            Vec::new()
        } else {
            serde_json::from_str(source.a("floatCarryRegions"))?
        };
        let inherited = !exclusions.is_empty();
        let (mut occupied_left, mut occupied_right) = (0f64, 0f64);
        for object in Self::items(&p).iter().filter_map(|i| match &i.kind {
            ItemKind::Object(n) if n.a("reflowFloat") == "1" => Some(n),
            _ => None,
        }) {
            let m = margin(object, "outMargin").map(|v| v.max(0.));
            let occupied = val(object.child("sz"), "width") + m[0] + m[1] + 1.;
            if object
                .child("pos")
                .is_some_and(|p| p.a("horzAlign") == "RIGHT")
            {
                occupied_right = occupied_right.max(occupied);
            } else {
                occupied_left = occupied_left.max(occupied);
            }
        }
        let side_space = width - occupied_left - occupied_right - settings.left - settings.right;
        for run in p.children.iter_mut().filter(|r| r.name == "run") {
            for n in run
                .children
                .iter_mut()
                .filter(|n| n.a("reflowFloat") == "1" && n.a("reflowGeneral") != "1")
            {
                let m = margin(n, "outMargin").map(|v| v.max(0.));
                let occupied = val(n.child("sz"), "width") + m[0] + m[1] + 1.;
                let height = effective_height(n) + m[2] + m[3];
                if side_space <= 24. {
                    // A full-width float has no usable side. Clear below it,
                    // retaining the established block layout instead of failing.
                    n.attrs.remove("reflowFloat");
                    n.attrs.insert("textWrap".into(), "TOP_AND_BOTTOM".into());
                    let pos = node_mut(n, "pos");
                    for (key, value) in [
                        ("horzAlign", "LEFT"),
                        ("horzOffset", "0"),
                        ("vertOffset", "0"),
                    ] {
                        pos.attrs.insert(key.into(), value.into());
                    }
                    continue;
                }
                let right = n.child("pos").is_some_and(|p| p.a("horzAlign") == "RIGHT");
                let pos = node_mut(n, "pos");
                pos.attrs.insert(
                    "horzOffset".into(),
                    (if right { -m[1] } else { m[0] } * 100.).to_string(),
                );
                pos.attrs
                    .insert("vertOffset".into(), (m[2] * 100.).to_string());
                exclusions.push(if right {
                    (0., occupied, height)
                } else {
                    (occupied, 0., height)
                });
            }
        }
        if !exclusions.is_empty() {
            let height = exclusions.iter().map(|(_, _, h)| *h).fold(0f64, f64::max);
            p.attrs.insert(
                if inherited {
                    "reflowFloatCarry"
                } else {
                    "reflowFloatHeight"
                }
                .into(),
                height.to_string(),
            );
            p.attrs.insert(
                "reflowFloatRegions".into(),
                serde_json::to_string(&exclusions)?,
            );
        }
        let mut general: Vec<crate::wrap_region::Region> = if source.a("generalCarry").is_empty() {
            Vec::new()
        } else {
            serde_json::from_str(source.a("generalCarry"))?
        };
        let extra = p.n("anchorBeforeExtra");
        for region in &mut general {
            region.shift_y(-extra);
        }
        let own_general = p
            .children_named("run")
            .flat_map(|r| &r.children)
            .any(|n| n.a("reflowGeneral") == "1");
        for run in p.children.iter_mut().filter(|r| r.name == "run") {
            for object in run
                .children
                .iter_mut()
                .filter(|n| n.a("reflowGeneral") == "1")
            {
                let region = self.position_region(object, width, &general)?;
                let region = if object.a("pageAbsolute") == "1"
                    && (self.active_columns > 1 || object.a("textWrap") == "TOP_AND_BOTTOM")
                {
                    let b = region.bounds();
                    crate::wrap_region::Region {
                        points: vec![(0., b[1]), (width, b[1]), (width, b[3]), (0., b[3])],
                        parts: None,
                        margin: [0.; 4],
                        flow: String::new(),
                    }
                } else {
                    region
                };
                general.push(region);
            }
        }
        if own_general
            && p.n("pageFloatFloor") == 0.
            && general.iter().map(|r| r.bounds()[3]).fold(0f64, f64::max) + settings.before
                > self.ph - self.top - self.bottom - 16.
        {
            let mut fallback = source.clone();
            fallback.attrs.remove("generalCarry");
            for run in fallback.children.iter_mut().filter(|r| r.name == "run") {
                for n in run.children.iter_mut().filter(|n| n.child("pos").is_some()) {
                    n.attrs.insert("textWrap".into(), "TOP_AND_BOTTOM".into());
                    n.attrs.remove("reflowFloat");
                    n.attrs.remove("reflowGeneral");
                }
            }
            self.report.warnings.insert(
                "Oversized connected objects placed as sequential blocks to preserve all content"
                    .into(),
            );
            return self.reflow_paragraph(&fallback, width, nesting);
        }
        if p.n("pageFloatFloor") > 0. {
            let floor = self.top
                + settings.before
                + general.iter().map(|r| r.bounds()[3]).fold(0f64, f64::max);
            p.attrs.insert(
                "pageFloatFloor".into(),
                floor.min(self.ph - self.bottom).to_string(),
            );
        }
        if !general.is_empty() {
            let height = general.iter().map(|r| r.bounds()[3]).fold(0f64, f64::max);
            p.attrs.insert(
                if own_general {
                    "reflowFloatHeight"
                } else {
                    "reflowFloatCarry"
                }
                .into(),
                height.to_string(),
            );
            p.attrs
                .insert("generalRegions".into(), serde_json::to_string(&general)?);
        }
        let mut grouped = self.paragraph_units(&p)?;
        // The label is not part of the UTF-16 text stream, but its ink must fit
        grouped.retain(|u| u.pos >= continuation);
        // the first line, including empty list items.
        if let Some((_, g, _)) = &number {
            if grouped.is_empty() {
                grouped.push(crate::reflow::Unit::default());
            }
            grouped[0].ascent = grouped[0].ascent.max(g.ascent);
            grouped[0].descent = grouped[0].descent.max(g.descent);
            grouped[0].em = grouped[0].em.max(g.size);
        }
        let wrapped = if !general.is_empty() {
            crate::reflow::wrap_regions(&grouped, width, &settings, &general)?
        } else if !exclusions.is_empty() {
            if source.a("floatFlow") == "1" {
                crate::reflow::wrap_flow(&grouped, width, &settings, &exclusions)?
            } else {
                crate::reflow::wrap_around(&grouped, width, &settings, exclusions.first().copied())?
            }
        } else {
            crate::reflow::wrap(&grouped, width, &settings)?
        };
        anyhow::ensure!(
            number.is_none() || !wrapped.first().is_some_and(|l| l.block),
            "List labels on leading block objects are not yet supported"
        );
        let mut segments = Node {
            name: "linesegarray".into(),
            ..Default::default()
        };
        let mut yy = 0.;
        for l in &wrapped {
            let mut n = Node {
                name: "lineseg".into(),
                ..Default::default()
            };
            for (key, value) in [
                ("textpos", l.pos.to_string()),
                ("vertpos", (yy * 100.).to_string()),
                ("vertsize", (l.height * 100.).to_string()),
                ("textheight", (l.height * 100.).to_string()),
                ("baseline", (l.baseline * 100.).to_string()),
                ("spacing", "0".into()),
                ("horzpos", (l.left * 100.).to_string()),
                ("horzsize", (l.width * 100.).to_string()),
                ("hardBreak", usize::from(l.hard).to_string()),
            ] {
                n.attrs.insert(key.into(), value);
            }
            segments.children.push(n);
            yy += l.height;
        }
        p.children.retain(|n| n.name != "linesegarray");
        p.children.push(segments);
        Ok((p, wrapped, settings))
    }
}
