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
    if list.a("savedColumns") == "1" {
        return list
            .children_named("p")
            .flat_map(|p| {
                lines(p)
                    .into_iter()
                    .enumerate()
                    .map(move |(i, l)| (p, i, l.n("vertpos") / 100.))
            })
            .collect();
    }
    let mut out = Vec::new();
    let mut shift = 0.;
    let mut last = None;
    let mut end = 0.;
    let mut previous_text = false;
    for p in list.children_named("p") {
        let has_text = !text(p).trim().is_empty();
        let cached = lines(p);
        let mut recovering_stalled = false;
        for (i, l) in cached.iter().enumerate() {
            let y = l.n("vertpos") / 100.;
            // A stalled complete line is distinct from another horizontal
            // segment of the same line. Retain its authored leading.
            let separate_line = i > 0
                && l.u("textpos") > cached[i - 1].u("textpos")
                && l.n("horzpos") == cached[i - 1].n("horzpos")
                && (l.u("flags") == 0 || l.u("flags") & (1 << 17) != 0)
                && (cached[i - 1].u("flags") == 0 || cached[i - 1].u("flags") & (1 << 18) != 0);
            if last
                .is_some_and(|last| y < last || (i == 0 && y == last && has_text && previous_text))
            {
                shift = end - y;
                recovering_stalled = false;
            }
            if separate_line && y == cached[i - 1].n("vertpos") / 100. {
                recovering_stalled = true;
            }
            let mut actual = y + shift;
            if separate_line && recovering_stalled {
                if actual < end {
                    actual = end;
                } else {
                    recovering_stalled = false;
                }
            }
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
    #[test]
    fn advancing_cached_lines_keep_their_authored_vertical_positions() {
        let n=crate::model::xml(br#"<subList><p><run><t>AB</t></run><linesegarray><lineseg textpos="0" vertpos="0" horzpos="0" vertsize="1000" spacing="600"/><lineseg textpos="1" vertpos="900" horzpos="0" vertsize="1000" spacing="600"/></linesegarray></p></subList>"#).unwrap();
        assert_eq!(
            flowing_lines(&n).iter().map(|l| l.2).collect::<Vec<_>>(),
            vec![0., 9.]
        );
    }
    #[test]
    fn stalled_lines_rejoin_the_source_grid_without_accumulating_a_shift() {
        let n=crate::model::xml(br#"<subList><p><run><t>ABC</t></run><linesegarray><lineseg textpos="0" vertpos="0" horzpos="0" vertsize="1000" spacing="600"/><lineseg textpos="1" vertpos="0" horzpos="0" vertsize="1000" spacing="600"/><lineseg textpos="2" vertpos="1600" horzpos="0" vertsize="1000" spacing="600"/></linesegarray></p><p><run><t>D</t></run><linesegarray><lineseg textpos="0" vertpos="4800" horzpos="0" vertsize="1000" spacing="600"/></linesegarray></p></subList>"#).unwrap();
        assert_eq!(
            flowing_lines(&n).iter().map(|l| l.2).collect::<Vec<_>>(),
            vec![0., 16., 32., 48.]
        );
    }
    #[test]
    fn repeated_complete_lines_keep_leading_without_moving_parallel_segments() {
        for (x, columns, expected) in [(0, 0, 16.), (2000, 0, 0.), (0, 1, 0.)] {
            let n=crate::model::xml(format!(r#"<subList savedColumns="{columns}"><p><run><t>AB</t></run><linesegarray><lineseg textpos="0" vertpos="0" horzpos="0" vertsize="1000" spacing="600" flags="393216"/><lineseg textpos="1" vertpos="0" horzpos="{x}" vertsize="1000" spacing="600" flags="393216"/></linesegarray></p></subList>"#).as_bytes()).unwrap();
            let flow = flowing_lines(&n);
            assert_eq!(flow[0].2, 0.);
            assert_eq!(flow[1].2, expected);
        }
    }

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
    emphasis: Option<text_marks::Emphasis>,
    font: usize,
    size: f64,
    scale: f64,
    width: f64,
    tracking: f64,
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
struct TableCellPaint {
    y: f64,
    height: f64,
    ops: std::ops::Range<usize>,
    borders: std::ops::Range<usize>,
    bounds: std::ops::Range<usize>,
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
    /// Opt-in reproduction of Hancom's restricted PCX visibility, not a security sandbox.
    pub hancom_hide_pcx: bool,
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
    table_cells: Vec<TableCellPaint>,
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
    page_float_floor: f64,
    column_widths: Vec<f64>,
    column_origins: Vec<f64>,
    column_definition: Option<Node>,
    column_band_top: f64,
    reflow_column: usize,
    shared_note_reserve: f64,
    furniture: Vec<Node>,
    field_stack: Vec<(String, Option<String>)>,
    flow_floor: Option<(usize, f64, bool)>,
}
mod anchors;
mod border_strokes;
mod caption_layout;
use caption_layout::side_caption_height;
mod column_balance;
mod column_flow;
mod column_geometry;
mod column_notes;
mod complex_text;
mod complex_unicode;
mod endnote_flow;
mod equation_layout;
mod field_navigation;
mod field_values;
mod font_selection;
mod generated_text;
mod list_labels;
mod local_flow;
mod native_cell_cache;
mod native_table_rows;
mod note_space;
mod notes;
mod object_paint;
mod object_position;
mod page_borders;
mod page_decorations;
mod page_furniture;
mod paragraph_borders;
mod rectangles;
mod reflow;
mod reflow_objects;
mod reflow_paragraph;
mod reflow_table;
mod reflow_units;
mod saved_blank_flow;
mod saved_cell_text;
mod saved_column_bands;
mod saved_float_order;
mod saved_numbering;
mod section_render;
mod shape_paths;
mod tab_layout;
mod table_cell_split;
mod table_headers;
mod table_layout;
mod table_measure;
mod table_pagination;
mod table_vectors;
mod text_decorations;
mod text_marks;
mod text_metrics;
mod text_paint;
mod text_style;
mod toc;

impl<'a> Engine<'a> {
    pub fn new(doc: &'a Document, fonts: &Path) -> Self {
        Self {
            doc,
            fonts: Fonts::new(fonts),
            pages: Vec::new(),
            report: Report::default(),
            hancom_hide_pcx: false,
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
            table_cells: Vec::new(),
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
            page_float_floor: 0.,
            column_widths: Vec::new(),
            column_origins: Vec::new(),
            column_definition: None,
            column_band_top: 0.,
            reflow_column: 0,
            shared_note_reserve: 0.,
            furniture: Vec::new(),
            field_stack: Vec::new(),
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
}

mod native_paragraph_cache;

mod character_borders;

mod page_numbers;

mod table_cells;
