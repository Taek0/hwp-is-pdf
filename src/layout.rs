use crate::{
    math,
    model::{Document, Node, text},
    pdf::{Fonts, Op, Page},
};
use anyhow::{Context, Result, bail};
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

fn val(n: Option<&Node>, key: &str) -> f64 {
    n.map(|n| n.n(key)).unwrap_or(0.) / 100.
}
fn node_mut<'a>(n: &'a mut Node, name: &str) -> &'a mut Node {
    let index = if let Some(i) = n.children.iter().position(|c| c.name == name) {
        i
    } else {
        n.children.push(Node {
            name: name.into(),
            ..Default::default()
        });
        n.children.len() - 1
    };
    &mut n.children[index]
}
fn color(s: &str) -> [f64; 3] {
    let v = u32::from_str_radix(s.trim_start_matches('#'), 16).unwrap_or(0);
    [
        ((v >> 16) & 255) as f64 / 255.,
        ((v >> 8) & 255) as f64 / 255.,
        (v & 255) as f64 / 255.,
    ]
}
fn margin(n: &Node, kind: &str) -> [f64; 4] {
    let c = n.child(kind);
    [
        val(c, "left"),
        val(c, "right"),
        val(c, "top"),
        val(c, "bottom"),
    ]
}
fn lines(p: &Node) -> Vec<&Node> {
    p.child("linesegarray")
        .map(|n| n.children_named("lineseg").collect())
        .unwrap_or_default()
}
fn paragraph_columns(p: &Node) -> Option<&Node> {
    p.children_named("run")
        .flat_map(|r| r.children_named("ctrl"))
        .find_map(|c| c.child("colPr"))
}
fn section_columns(s: &Node) -> Option<&Node> {
    s.children_named("p").find_map(paragraph_columns)
}
fn content_height(list: &Node) -> f64 {
    flowing_lines(list)
        .iter()
        .map(|(p, i, y)| y + lines(p)[*i].n("vertsize") / 100.)
        .fold(0., f64::max)
}
fn flowing_lines(list: &Node) -> Vec<(&Node, usize, f64)> {
    let mut out = Vec::new();
    let mut shift = 0.;
    let mut last = None;
    let mut end = 0.;
    let mut previous_text = false;
    for p in list.children_named("p") {
        let has_text = !text(p).trim().is_empty();
        for (i, l) in lines(p).iter().enumerate() {
            let y = l.n("vertpos") / 100.;
            if last
                .is_some_and(|last| y < last || (i == 0 && y == last && has_text && previous_text))
            {
                shift = end - y;
            }
            let actual = y + shift;
            out.push((p, i, actual));
            end = actual + (l.n("vertsize") + l.n("spacing")) / 100.;
            last = Some(y);
        }
        previous_text = has_text;
    }
    out
}
fn effective_height(n: &Node) -> f64 {
    let declared = val(n.child("sz"), "height");
    if n.name == "rect" {
        let text = n
            .child("drawText")
            .and_then(|d| {
                d.child("subList").map(|l| {
                    let m = margin(d, "textMargin");
                    content_height(l) + m[2] + m[3]
                })
            })
            .unwrap_or(0.);
        let current = if n
            .child("rotationInfo")
            .is_some_and(|r| r.n("angle").rem_euclid(360.).abs() > 0.001)
        {
            declared
        } else {
            val(n.child("curSz"), "height")
        };
        declared.max(current).max(text)
    } else {
        declared
    }
}
fn caption_extent(n: &Node) -> [f64; 4] {
    let mut e = [0f64; 4];
    for c in n.children_named("caption") {
        let Some(list) = c.child("subList") else {
            continue;
        };
        let side = match c.a("side") {
            "LEFT" => 0,
            "RIGHT" => 1,
            "TOP" => 2,
            _ => 3,
        };
        let size = if side < 2 {
            c.n("width") / 100.
        } else {
            content_height(list)
        };
        e[side] = e[side].max(size + c.n("gap") / 100.);
    }
    e
}

#[cfg(test)]
mod regression_tests {
    use super::*;
    use crate::model::xml;
    #[test]
    fn stale_textbox_height_uses_current_geometry_and_content() {
        let n=xml(br#"<rect><sz height="3819"/><curSz height="6866"/><drawText><subList><p><linesegarray><lineseg vertpos="5175" vertsize="1125"/></linesegarray></p></subList><textMargin top="283" bottom="283"/></drawText></rect>"#).unwrap();
        assert!((effective_height(&n) - 68.66).abs() < 0.0001);
        assert!((effective_height(&n) - val(n.child("sz"), "height") - 30.47).abs() < 0.0001);
    }
    #[test]
    fn negative_nested_table_origin_contributes_to_cell_height() {
        let n=xml(br#"<subList><p><run><tbl><sz height="7311"/><pos treatAsChar="0" vertRelTo="PARA" vertOffset="4294966449"/></tbl></run><linesegarray><lineseg vertpos="0" vertsize="1000"/></linesegarray></p></subList>"#).unwrap();
        let (a, b) = floating_extent(&n);
        assert!((a + 8.47).abs() < 0.0001);
        assert!((b - a - 73.11).abs() < 0.0001);
    }
    #[test]
    fn inline_object_and_tab_keep_hwp_character_positions() {
        let n=xml("<p><run charPrIDRef=\"1\"><t>가</t><equation/><t>나<tab width=\"2000\"/>다</t></run></p>".as_bytes()).unwrap();
        let its = Engine::items(&n);
        assert_eq!(
            its.iter().map(|i| i.pos).collect::<Vec<_>>(),
            vec![0, 1, 9, 10, 18]
        );
    }
}
fn floating_extent(list: &Node) -> (f64, f64) {
    let mut top = 0f64;
    let mut bottom = content_height(list);
    for p in list.children_named("p") {
        let py = lines(p)
            .first()
            .map(|l| l.n("vertpos") / 100.)
            .unwrap_or(0.);
        for run in p.children_named("run") {
            for n in &run.children {
                if let Some(pos) = n.child("pos")
                    && pos.a("treatAsChar") == "0"
                    && pos.a("vertRelTo") == "PARA"
                {
                    let y = py + pos.signed("vertOffset") / 100.;
                    top = top.min(y);
                    bottom = bottom.max(y + effective_height(n));
                }
            }
        }
    }
    (top, bottom)
}
#[derive(Clone)]
enum ItemKind {
    Char(char),
    Tab(f64),
    Object(Node),
    Control(Node),
}
#[derive(Clone)]
struct Item {
    pos: usize,
    style: String,
    kind: ItemKind,
}
#[derive(Clone)]
struct Glyph {
    font: usize,
    size: f64,
    scale: f64,
    width: f64,
    color: [f64; 3],
    ascent: f64,
    descent: f64,
    ink_left: f64,
    ink_right: f64,
    shift: f64,
    shear: f64,
    raw_ascent: f64,
    raw_descent: f64,
}
type ContinuedNoteRows = BTreeMap<
    i64,
    (
        f64,
        f64,
        std::collections::VecDeque<(Node, usize, f64, String)>,
    ),
>;
#[derive(Serialize)]
pub struct Bounds {
    pub page: usize,
    pub kind: String,
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}
#[derive(Default, Serialize)]
pub struct Report {
    pub charts: usize,
    pub pages: usize,
    pub equations: usize,
    pub pictures: usize,
    pub tables: usize,
    pub warnings: BTreeSet<String>,
    pub bounds: Vec<Bounds>,
}
struct TableHeaderPaint {
    y: f64,
    height: f64,
    end: f64,
    ops: Vec<Op>,
}
struct PlannedTableNote {
    node: Node,
    x: f64,
    width: f64,
    y: f64,
    top: f64,
    op_index: usize,
}
pub struct Engine<'a> {
    pub doc: &'a Document,
    pub fonts: Fonts,
    pub pages: Vec<Page>,
    pub report: Report,
    pw: f64,
    ph: f64,
    left: f64,
    right: f64,
    top: f64,
    bottom: f64,
    header: f64,
    footer: f64,
    current: Page,
    notes: Vec<Node>,
    footnotes: Vec<(Node, f64, f64)>,
    continued_notes: ContinuedNoteRows,
    draining_notes: bool,
    capture_table_headers: bool,
    table_headers: Vec<TableHeaderPaint>,
    table_note_anchors: Vec<PlannedTableNote>,
    split_caption_handled: bool,
    depth: usize,
    glyph_cache: BTreeMap<(char, String), Glyph>,
    active_section: usize,
    page_base: i64,
    numbering: BTreeMap<(String, usize), usize>,
    labels: BTreeMap<(usize, String), String>,
    note_rendering: bool,
    active_columns: usize,
    active_gap: f64,
    last_table_end: f64,
    reflow_table_column: Option<usize>,
    contour_wrap: bool,
    general_wrap: bool,
    flow_floor: Option<(usize, f64, bool)>,
}
mod notes;
mod object_paint;
mod page_decorations;
mod reflow;
mod reflow_objects;
mod reflow_paragraph;
mod reflow_table;
mod table_layout;
mod table_pagination;
mod text_paint;
mod text_style;

impl<'a> Engine<'a> {
    pub fn new(doc: &'a Document, fonts: &Path) -> Self {
        Self {
            doc,
            fonts: Fonts::new(fonts),
            pages: Vec::new(),
            report: Report::default(),
            pw: 0.,
            ph: 0.,
            left: 0.,
            right: 0.,
            top: 0.,
            bottom: 0.,
            header: 0.,
            footer: 0.,
            current: Page::default(),
            notes: Vec::new(),
            footnotes: Vec::new(),
            continued_notes: BTreeMap::new(),
            draining_notes: false,
            capture_table_headers: false,
            table_headers: Vec::new(),
            table_note_anchors: Vec::new(),
            split_caption_handled: false,
            depth: 0,
            glyph_cache: BTreeMap::new(),
            active_section: 0,
            page_base: 1,
            numbering: BTreeMap::new(),
            labels: BTreeMap::new(),
            note_rendering: false,
            active_columns: 1,
            active_gap: 0.,
            last_table_end: 0.,
            reflow_table_column: None,
            contour_wrap: false,
            general_wrap: false,
            flow_floor: None,
        }
    }
    pub fn render(&mut self) -> Result<()> {
        self.render_mode(false)
    }
    pub fn render_reflow_text(&mut self) -> Result<()> {
        self.render_mode(true)
    }
    pub fn render_contours(&mut self) -> Result<()> {
        self.contour_wrap = true;
        self.report.warnings.insert("Opt-in geometric contour wrapping with source flow direction, alpha masks, cubic outlines and group components; complex contours use conservative bounds".into());
        self.render_mode(true)
    }
    pub fn render_reflow(&mut self) -> Result<()> {
        self.render_mode(true)
    }
    fn render_mode(&mut self, force_reflow: bool) -> Result<()> {
        let mut reflow_used = false;
        let mut note_start = (0., 0usize, 0., 0.);
        for (section_index, section) in self.doc.sections.iter().enumerate() {
            self.active_section = section_index;
            if let Some(number) = section.find("newNumber").filter(|n| n.a("type") == "0") {
                self.page_base = number.u("number") as i64 - self.pages.len() as i64;
            }
            let sec = section
                .find("secPr")
                .context("Missing section properties")?;
            let p = sec.child("pagePr").context("Missing page size")?;
            self.pw = p.n("width") / 100.;
            self.ph = p.n("height") / 100.;
            let m = margin(p, "margin");
            self.left = m[0];
            self.right = m[1];
            self.top = m[2] + val(p.child("margin"), "header");
            self.bottom = m[3] + val(p.child("margin"), "footer");
            self.header = val(p.child("margin"), "header");
            self.footer = val(p.child("margin"), "footer");
            let col = section_columns(section);
            let mut cols = col.map(|c| c.u("colCount")).unwrap_or(1).max(1);
            let mut gap = val(col, "sameGap");
            let mut cw = (self.pw - self.left - self.right - gap * (cols - 1) as f64) / cols as f64;
            self.active_columns = cols;
            self.active_gap = gap;
            self.decorations(section, true, cols, gap)?;
            if force_reflow || section.children_named("p").any(|p| lines(p).is_empty()) {
                reflow_used = true;
                note_start = self.reflow_text_section(section, cw, cols, gap)?;
            } else {
                let mut col = 0;
                let mut last = None;
                let mut flow_shift = 0f64;
                let mut table_reflowed = false;
                let fallback_flow = section
                    .children_named("p")
                    .filter(|p| !text(p).trim().is_empty())
                    .count()
                    > 1
                    && section
                        .children_named("p")
                        .flat_map(lines)
                        .all(|l| l.n("vertpos") == 0.);
                if fallback_flow {
                    self.report.warnings.insert(
                        "Rebuilt paragraph flow: all cached paragraph origins were zero".into(),
                    );
                }
                for p in section.children_named("p") {
                    let ls = lines(p);
                    if ls.is_empty() {
                        bail!("Missing saved layout cache in paragraph {}", p.a("id"))
                    }
                    if let Some(def) = paragraph_columns(p) {
                        let next_count = def.u("colCount").max(1);
                        let next_gap = val(Some(def), "sameGap");
                        if next_count != cols || next_gap != gap {
                            if last.is_some_and(|y| ls[0].n("vertpos") < y)
                                || p.a("pageBreak") == "1"
                            {
                                self.finish()?;
                                last = None;
                                flow_shift = 0.;
                            }
                            cols = next_count;
                            gap = next_gap;
                            cw = (self.pw - self.left - self.right - gap * (cols - 1) as f64)
                                / cols as f64;
                            col = 0;
                            self.active_columns = cols;
                            self.active_gap = gap;
                        }
                    }
                    for (i, l) in ls.iter().enumerate() {
                        let y = l.n("vertpos");
                        let floor = self.flow_floor.take().filter(|f| f.0 == self.pages.len());
                        let table_consumed_break =
                            floor.is_some_and(|f| f.2) && p.a("pageBreak") != "1";
                        if (last.is_some_and(|last| y < last) && !table_consumed_break)
                            || (last.is_some() && i == 0 && p.a("pageBreak") == "1")
                        {
                            col += 1;
                            flow_shift = 0.;
                            table_reflowed = false;
                            if i == 0 && p.a("pageBreak") == "1" {
                                col = cols
                            }
                            if col >= cols {
                                self.finish()?;
                                col = 0;
                                self.decorations(section, false, cols, gap)?;
                            }
                        }
                        if let Some((page, bottom, _)) = floor.filter(|f| f.0 == self.pages.len()) {
                            let _ = page;
                            flow_shift = flow_shift.max(bottom - self.top - y / 100.);
                            table_reflowed = true;
                        }
                        let end = ls.get(i + 1).map(|l| l.u("textpos")).unwrap_or(usize::MAX);
                        let its = Self::items(p);
                        let incoming: Vec<_> = its
                            .iter()
                            .filter(|it| it.pos >= l.u("textpos") && it.pos < end)
                            .filter_map(|it| match &it.kind {
                                ItemKind::Control(n) if n.name == "footNote" => Some(n),
                                _ => None,
                            })
                            .collect();
                        let reserve = self
                            .footnote_reservation(&incoming, self.left + col as f64 * (cw + gap));
                        let visible = its
                            .iter()
                            .filter(|it| it.pos >= l.u("textpos") && it.pos < end)
                            .any(|it| match &it.kind {
                                ItemKind::Char(c) => !c.is_whitespace(),
                                ItemKind::Object(_) => true,
                                ItemKind::Control(n) => {
                                    matches!(n.name.as_str(), "footNote" | "endNote" | "autoNum")
                                }
                                _ => false,
                            });
                        if visible
                            && (reserve > 0. || fallback_flow || table_reflowed)
                            && self.top + flow_shift + (y + l.n("vertsize")) / 100.
                                > self.ph - self.bottom - reserve
                            && self.top + flow_shift + y / 100. > self.top + 0.1
                        {
                            col += 1;
                            if col >= cols {
                                self.finish()?;
                                col = 0;
                                self.decorations(section, false, cols, gap)?;
                            }
                            flow_shift = -y / 100.;
                        }
                        self.line(
                            p,
                            i,
                            self.left + col as f64 * (cw + gap),
                            self.top + flow_shift,
                            cw,
                        )?;
                        let end = ls.get(i + 1).map(|n| n.u("textpos")).unwrap_or(usize::MAX);
                        let growth = Self::items(p)
                            .iter()
                            .filter(|it| it.pos >= l.u("textpos") && it.pos < end)
                            .filter_map(|it| {
                                if let ItemKind::Object(n) = &it.kind {
                                    if n.child("pos")
                                        .map(|p| p.a("treatAsChar") == "1")
                                        .unwrap_or(false)
                                    {
                                        Some(
                                            (effective_height(n) - val(n.child("sz"), "height"))
                                                .max(0.),
                                        )
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            })
                            .fold(0., f64::max);
                        if growth > 0.01 {
                            self.report.warnings.insert(format!("Repaired stale inline shape height: paragraph {}, +{growth:.2}pt; subsequent lines shifted",p.a("id")));
                            flow_shift += growth;
                        }
                        note_start = (
                            self.top + flow_shift + (y + l.n("vertsize") + l.n("spacing")) / 100.,
                            col,
                            cw,
                            gap,
                        );
                        last = Some(y);
                        if fallback_flow {
                            flow_shift += (l.n("vertsize") + l.n("spacing")) / 100.;
                        }
                    }
                }
            }
            if self
                .note_spec(true)
                .and_then(|n| n.child("placement"))
                .is_some_and(|n| n.a("place") == "END_OF_SECTION")
            {
                self.render_endnotes(note_start, Some(section_index))?;
            }
            if section_index + 1 < self.doc.sections.len() {
                self.finish()?;
                self.drain_note_tail()?;
            }
        }
        self.render_endnotes(note_start, None)?;
        if !self.current.ops.is_empty() || (reflow_used && self.pages.is_empty()) {
            self.finish()?;
        }
        self.drain_note_tail()?;
        self.report.pages = self.pages.len();
        if !reflow_used {
            self.report.warnings.insert("Saved-layout renderer: font substitutions can differ from Hancom; no general paragraph reflow".into());
        }
        Ok(())
    }
}
