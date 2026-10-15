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
    fn note_spec(&self, end: bool) -> Option<&Node> {
        self.doc.sections[self.active_section]
            .find("secPr")
            .and_then(|s| s.child(if end { "endNotePr" } else { "footNotePr" }))
    }
    fn note_gap(&self, end: bool, key: &str, default: f64) -> f64 {
        self.note_spec(end)
            .and_then(|s| s.child("noteSpacing"))
            .map(|s| s.n(key) / 100.)
            .unwrap_or(default)
    }
    fn note_label(&self, n: &Node, end: bool) -> String {
        let number = if n.a("number").is_empty() {
            n.find("autoNum").map(|a| a.u("num")).unwrap_or(1)
        } else {
            n.u("number")
        };
        let format = self.note_spec(end).and_then(|s| s.child("autoNumFormat"));
        format!(
            "{}{}{}",
            format.map(|f| f.a("prefixChar")).unwrap_or(""),
            number,
            format.map(|f| f.a("suffixChar")).unwrap_or(")")
        )
    }
    fn auto_label(&self, n: &Node) -> String {
        let number = if n.a("numType") == "PAGE" {
            (self.pages.len() as i64 + self.page_base).max(1) as usize
        } else {
            n.u("num")
        };
        let f = n.child("autoNumFormat");
        if matches!(n.a("numType"), "FOOTNOTE" | "ENDNOTE") {
            format!(
                "{}{}{}",
                f.map(|f| f.a("prefixChar")).unwrap_or(""),
                number,
                f.map(|f| f.a("suffixChar")).unwrap_or(")")
            )
        } else {
            number.to_string()
        }
    }
    fn footnote_area(&self, x: f64) -> (f64, f64) {
        let count = self.active_columns.max(1);
        let width = (self.pw - self.left - self.right - self.active_gap * (count - 1) as f64)
            / count as f64;
        let placement = self
            .note_spec(false)
            .and_then(|s| s.child("placement"))
            .map(|p| p.a("place"))
            .unwrap_or("EACH_COLUMN");
        if placement == "MERGED_COLUMN" {
            return (self.left, self.pw - self.left - self.right);
        }
        let col = if placement == "RIGHT_MOST_COLUMN" {
            count - 1
        } else {
            ((x - self.left) / (width + self.active_gap))
                .floor()
                .max(0.) as usize
        }
        .min(count - 1);
        (self.left + col as f64 * (width + self.active_gap), width)
    }
    fn footnote_reservation(&self, extra: &[&Node], x: f64) -> f64 {
        let (nx, _) = self.footnote_area(x);
        let mut count = 0;
        let mut h = 0.;
        let pending = self.continued_notes.get(&((nx * 100.).round() as i64));
        if let Some((_, _, rows)) = pending {
            h += rows.iter().map(|r| r.2).sum::<f64>();
            count += 1;
        }
        for (n, xx, _) in &self.footnotes {
            if (nx - xx).abs() < 0.1
                && let Some(l) = n.child("subList")
            {
                h += content_height(l);
                count += 1;
            }
        }
        for n in extra {
            if let Some(l) = n.child("subList") {
                h += content_height(l);
                count += 1;
            }
        }
        if count == 0 {
            return 0.;
        }
        let full = h
            + self.note_gap(false, "betweenNotes", 4.) * (count - 1) as f64
            + self.note_gap(false, "aboveLine", 6.)
            + self.note_gap(false, "belowLine", 4.)
            + 1.;
        let body = self.ph - self.top - self.bottom;
        if pending.is_some() {
            full.min(body * 0.5)
        } else if full > body - 48. {
            body * 0.5
        } else {
            full
        }
    }
    fn note_separator(&mut self, end: bool, x: f64, y: f64, width: f64) {
        let line = self.note_spec(end).and_then(|s| s.child("noteLine"));
        if line.is_some_and(|l| l.a("type") == "NONE" || l.n("length") == 0.) {
            return;
        }
        let length = line.map(|l| l.n("length")).unwrap_or(-1.);
        let length = if length < 0. {
            width / 3.
        } else {
            (length / 100.).min(width)
        };
        let thickness = line
            .map(|l| {
                if l.a("width").ends_with("mm") {
                    l.a("width")
                        .trim_end_matches("mm")
                        .trim()
                        .parse::<f64>()
                        .unwrap_or(0.12)
                        * 72.
                        / 25.4
                } else {
                    [
                        0.1, 0.12, 0.15, 0.2, 0.25, 0.3, 0.4, 0.5, 0.6, 0.7, 1., 1.5, 2., 3., 4.,
                        5.,
                    ][l.u("widthIndex").min(15)]
                        * 72.
                        / 25.4
                }
            })
            .unwrap_or(0.34);
        let c = color(line.map(|l| l.a("color")).unwrap_or("#000000"));
        self.current.ops.push(Op::Line {
            x,
            y,
            x2: x + length,
            y2: y,
            width: thickness,
            color: c,
        });
    }
    fn next_note_column(&mut self, col: &mut usize, y: &mut f64, gap: f64) -> Result<()> {
        *col += 1;
        if *col >= self.active_columns.max(1) {
            self.finish()?;
            *col = 0;
            let section = &self.doc.sections[self.active_section];
            self.decorations(section, false, self.active_columns, gap)?;
        }
        *y = self.top;
        Ok(())
    }
    fn render_endnotes(
        &mut self,
        start: (f64, usize, f64, f64),
        section_only: Option<usize>,
    ) -> Result<()> {
        let (notes, pending): (Vec<_>, Vec<_>) = std::mem::take(&mut self.notes)
            .into_iter()
            .partition(|n| section_only.is_none_or(|s| n.u("sourceSection") == s));
        self.notes = pending;
        if notes.is_empty() {
            return Ok(());
        }
        let (mut y, mut col, width, gap) = start;
        let above = self.note_gap(true, "aboveLine", 0.);
        let below = self.note_gap(true, "belowLine", 0.);
        let between = self.note_gap(true, "betweenNotes", 8.);
        let mut separator = true;
        self.note_rendering = true;
        for n in notes {
            if let Some(list) = n.child("subList") {
                let h = content_height(list);
                if y + h + if separator { above + below + 1. } else { 0. } > self.ph - self.bottom
                    && y > self.top + 0.1
                {
                    self.next_note_column(&mut col, &mut y, gap)?;
                    self.note_rendering = true;
                    separator = true;
                }
                if separator {
                    self.note_separator(
                        true,
                        self.left + col as f64 * (width + gap),
                        y + above,
                        width,
                    );
                    y += above + below + 1.;
                    separator = false;
                }
                let x = self.left + col as f64 * (width + gap);
                if y + h <= self.ph - self.bottom {
                    self.report.bounds.push(Bounds {
                        page: self.pages.len() + 1,
                        kind: "endnote".into(),
                        id: n.a("number").into(),
                        x,
                        y,
                        w: width,
                        h,
                    });
                    self.local(list, x, y, width)?;
                    y += h + between;
                } else {
                    let mut previous = 0.;
                    self.depth += 1;
                    for (p, i, yy) in flowing_lines(list) {
                        let line = lines(p)[i];
                        let height = line.n("vertsize") / 100.;
                        y += (yy - previous).max(0.);
                        if y + height > self.ph - self.bottom {
                            self.next_note_column(&mut col, &mut y, gap)?;
                            self.note_rendering = true;
                            self.note_separator(
                                true,
                                self.left + col as f64 * (width + gap),
                                y + above,
                                width,
                            );
                            y += above + below + 1.;
                        }
                        self.line(
                            p,
                            i,
                            self.left + col as f64 * (width + gap),
                            y - line.n("vertpos") / 100.,
                            width,
                        )?;
                        y += height;
                        previous = yy + height;
                    }
                    self.depth -= 1;
                    y += between;
                }
            }
        }
        self.note_rendering = false;
        self.report.warnings.insert("Endnotes follow saved lines with configured spacing and column count; exact Hancom pagination remains unverified".into());
        Ok(())
    }
    fn glyph(&mut self, c: char, style: &str) -> Result<Glyph> {
        self.glyph_with_font(c, style, None)
    }
    fn glyph_with_font(
        &mut self,
        c: char,
        style: &str,
        override_font: Option<&str>,
    ) -> Result<Glyph> {
        let key = (c, format!("{style}\0{}", override_font.unwrap_or("")));
        if let Some(g) = self.glyph_cache.get(&key) {
            return Ok(g.clone());
        }
        let ch = self.doc.chars.get(style).context("Invalid charPrIDRef")?;
        let latin = c.is_ascii();
        let lang = if latin { "latin" } else { "hangul" };
        let langkey = if latin { "LATIN" } else { "HANGUL" };
        let fontref = ch.child("fontRef").map(|n| n.a(lang)).unwrap_or("0");
        let name = self
            .doc
            .fonts
            .get(&(langkey.into(), fontref.into()))
            .map(String::as_str)
            .unwrap_or("unknown");
        let bold = ch.child("bold").is_some();
        let italic = ch.child("italic").is_some();
        let serif = name.contains("명조") || name.contains("바탕") || name.contains("Times");
        let file = if name.eq_ignore_ascii_case("Wingdings") {
            "wingding.ttf"
        } else if name.eq_ignore_ascii_case("Symbol") {
            "symbol.ttf"
        } else if latin && serif {
            match (bold, italic) {
                (true, true) => "timesbi.ttf",
                (false, true) => "timesi.ttf",
                (true, false) => "timesbd.ttf",
                _ => "times.ttf",
            }
        } else if latin {
            match (bold, italic) {
                (true, true) => "arialbi.ttf",
                (false, true) => "ariali.ttf",
                (true, false) => "arialbd.ttf",
                _ => "arial.ttf",
            }
        } else if serif {
            "batang.ttc"
        } else if bold {
            "malgunbd.ttf"
        } else {
            "malgun.ttf"
        };
        let file = override_font.unwrap_or(file);
        self.report
            .warnings
            .insert(format!("Font substitution: {name} -> {file}"));
        let mut font = self.fonts.get(file)?;
        if !self.fonts.has(font, c) {
            for fallback in ["malgun.ttf", "simsun.ttc", "msgothic.ttc", "seguisym.ttf"] {
                if !self.fonts.root.join(fallback).exists() {
                    continue;
                }
                let other = self.fonts.get(fallback)?;
                if self.fonts.has(other, c) {
                    font = other;
                    break;
                }
            }
            if !self.fonts.has(font, c) {
                self.report
                    .warnings
                    .insert(format!("Missing glyph U+{:04X}", c as u32));
            }
        }
        let nominal_size =
            ch.n("height") / 100. * ch.child("relSz").map(|n| n.n(lang) / 100.).unwrap_or(1.);
        let superscript = ch.child("supscript").is_some();
        let subscript = ch.child("subscript").is_some();
        let script_scale = if superscript || subscript { 0.65 } else { 1. };
        let size = nominal_size * script_scale;
        let offset_lang = match c as u32 {
            0x3040..=0x30ff | 0x31f0..=0x31ff => "japanese",
            0x3400..=0x9fff | 0xf900..=0xfaff | 0x20000..=0x323af => "hanja",
            0x1100..=0x11ff | 0x3130..=0x318f | 0xa960..=0xa97f | 0xac00..=0xd7ff => "hangul",
            0..=0x024f => "latin",
            _ => "other",
        };
        let offset = ch.child("offset").map(|n| n.n(offset_lang)).unwrap_or(0.);
        let shift = nominal_size
            * (offset / 100.
                + if superscript {
                    -0.35
                } else if subscript {
                    0.15
                } else {
                    0.
                });
        if superscript || subscript {
            self.report.warnings.insert("Superscript/subscript use 65% size and nominal-font baseline shifts; exact Hancom metrics unverified".into());
        }
        let scale = ch.child("ratio").map(|n| n.n(lang) / 100.).unwrap_or(1.);
        let spacing = ch
            .child("spacing")
            .map(|n| n.n(lang) / 100. * size)
            .unwrap_or(0.);
        let width = self.fonts.width(font, &c.to_string(), size) * scale + spacing;
        let face = ttf_parser::Face::parse(&self.fonts.list[font].bytes, 0).unwrap();
        let shear = if italic && !face.is_italic() {
            12f64.to_radians().tan()
        } else {
            0.
        };
        if shear != 0. {
            self.report.warnings.insert("Italic without native italic face uses 12-degree synthetic slant; exact Hancom angle unverified".into());
        }
        let bb = crate::pdf::glyph_id(&face, c).and_then(|id| face.glyph_bounding_box(id));
        let unit = size / face.units_per_em() as f64;
        let glyph = Glyph {
            font,
            size,
            scale,
            width,
            color: color(ch.a("textColor")),
            ascent: (bb.map(|b| b.y_max as f64 * unit).unwrap_or(0.) - shift).max(0.),
            descent: (bb.map(|b| -b.y_min as f64 * unit).unwrap_or(0.) + shift).max(0.),
            ink_left: bb
                .map(|b| (b.x_min as f64 * scale + shear * b.y_min as f64) * unit)
                .unwrap_or(0.),
            ink_right: bb
                .map(|b| (b.x_max as f64 * scale + shear * b.y_max as f64) * unit)
                .unwrap_or(0.),
            shift,
            shear,
            raw_ascent: bb.map(|b| b.y_max as f64 * unit).unwrap_or(0.),
            raw_descent: bb.map(|b| (-b.y_min as f64 * unit).max(0.)).unwrap_or(0.),
        };
        self.glyph_cache.insert(key, glyph.clone());
        Ok(glyph)
    }
    fn items(p: &Node) -> Vec<Item> {
        let mut items = Vec::new();
        let mut pos = 0;
        for run in p.children_named("run") {
            let style = run.a("charPrIDRef").to_string();
            for n in &run.children {
                match n.name.as_str() {
                    "t" => {
                        for child in &n.children {
                            match child.name.as_str() {
                                "#text" => {
                                    for c in child.text.chars() {
                                        items.push(Item {
                                            pos,
                                            style: style.clone(),
                                            kind: ItemKind::Char(c),
                                        });
                                        pos += c.len_utf16()
                                    }
                                }
                                "tab" => {
                                    items.push(Item {
                                        pos,
                                        style: style.clone(),
                                        kind: ItemKind::Tab(child.n("width") / 100.),
                                    });
                                    pos += 8;
                                }
                                "lineBreak" => pos += 1,
                                "hwpDisplay" => {
                                    for c in child.a("value").chars() {
                                        items.push(Item {
                                            pos,
                                            style: style.clone(),
                                            kind: ItemKind::Char(c),
                                        });
                                    }
                                    pos += child.u("units");
                                }
                                "nbSpace" | "fwSpace" => {
                                    items.push(Item {
                                        pos,
                                        style: style.clone(),
                                        kind: ItemKind::Char(' '),
                                    });
                                    pos += 1;
                                }
                                _ => {}
                            }
                        }
                    }
                    "tbl" | "pic" | "equation" | "rect" | "line" | "ellipse" | "polygon"
                    | "curve" | "arc" | "group" | "chart" => {
                        items.push(Item {
                            pos,
                            style: style.clone(),
                            kind: ItemKind::Object(n.clone()),
                        });
                        pos += 8;
                    }
                    "ctrl" => {
                        for c in n.children.iter().filter(|c| c.name != "#text") {
                            items.push(Item {
                                pos,
                                style: style.clone(),
                                kind: ItemKind::Control(c.clone()),
                            });
                            pos += 8;
                        }
                    }
                    "secPr" => pos += 8,
                    _ => {}
                }
            }
        }
        items
    }
    fn paragraph_number(&mut self, p: &Node, reflow: bool) -> Result<Option<(String, Glyph, f64)>> {
        let style = self.doc.paras.get(p.a("paraPrIDRef"));
        if let Some(heading) = style.and_then(|s| s.child("heading"))
            && matches!(heading.a("type"), "NUMBER" | "BULLET")
        {
            let bullet = heading.a("type") == "BULLET";
            let mut defs = Vec::new();
            self.doc
                .header
                .all(if bullet { "bullet" } else { "numbering" }, &mut defs);
            let definition = defs.into_iter().find(|d| d.a("id") == heading.a("idRef"));
            let head = if bullet {
                definition
            } else {
                definition.and_then(|n| {
                    n.children_named("paraHead")
                        .find(|h| h.u("level") == heading.u("level") + 1)
                })
            }
            .cloned();
            if let Some(mut head) = head {
                // HWPX stores bullet layout attributes in a paraHead child;
                // normalized HWP stores them on the bullet itself.
                if reflow
                    && bullet
                    && let Some(properties) = head.child("paraHead").cloned()
                {
                    head.attrs.extend(properties.attrs);
                }
                let key = (self.active_section, p.a("id").to_string());
                let label = if bullet {
                    anyhow::ensure!(
                        !reflow || !head.a("char").is_empty(),
                        "Image-only or empty bullet reflow is not supported"
                    );
                    head.a("char").to_string()
                } else if let Some(label) = self.labels.get(&key) {
                    label.clone()
                } else {
                    let derived = Self::items(p).iter().find_map(|i| match &i.kind {
                        ItemKind::Control(c) if c.name == "endNote" => Some(c.u("number")),
                        _ => None,
                    });
                    let nk = (heading.a("idRef").to_string(), heading.u("level"));
                    let num = if let Some(n) = derived {
                        n
                    } else {
                        let start = if reflow && head.u("start") > 0 {
                            head.u("start")
                        } else {
                            definition.map(|d| d.u("start")).unwrap_or(1).max(1)
                        };
                        let n = self.numbering.entry(nk.clone()).or_insert(start - 1);
                        *n += 1;
                        *n
                    };
                    let mut fmt = text(&head);
                    if fmt.is_empty() {
                        fmt = format!("{num}.");
                    } else {
                        for level in (1..=10).rev() {
                            let v = if level == heading.u("level") + 1 {
                                num
                            } else {
                                *self.numbering.get(&(nk.0.clone(), level - 1)).unwrap_or(&1)
                            };
                            fmt = fmt.replace(&format!("^{level}"), &v.to_string());
                        }
                    }
                    if reflow {
                        self.numbering
                            .retain(|(id, level), _| id != &nk.0 || *level <= nk.1);
                    }
                    self.labels.insert(key, fmt.clone());
                    fmt
                };
                let fallback = p
                    .children_named("run")
                    .next()
                    .map(|n| n.a("charPrIDRef"))
                    .unwrap_or("0");
                let style = if self.doc.chars.contains_key(head.a("charPrIDRef")) {
                    head.a("charPrIDRef")
                } else {
                    fallback
                };
                let first = label.chars().next().unwrap_or('1');
                let legacy_symbol = bullet
                    && !self.doc.chars.contains_key(head.a("charPrIDRef"))
                    && ('\u{f020}'..='\u{f0ff}').contains(&first)
                    && self.fonts.root.join("wingding.ttf").exists();
                if legacy_symbol {
                    self.report.warnings.insert(
                        "Unassigned legacy symbol bullet font: using Wingdings cmap".into(),
                    );
                }
                let g =
                    self.glyph_with_font(first, style, legacy_symbol.then_some("wingding.ttf"))?;
                let nw = self.fonts.width(g.font, &label, g.size);
                let offset = if head.a("textOffsetType") == "PERCENT" {
                    g.size * head.n("textOffset") / 100.
                } else {
                    head.n("textOffset") / 100.
                };
                let reserved = if reflow {
                    nw * g.scale + offset.max(0.)
                } else {
                    nw + offset
                };
                return Ok(Some((label, g, reserved)));
            }
        }
        Ok(None)
    }
    fn line(&mut self, p: &Node, li: usize, ox: f64, oy: f64, width: f64) -> Result<()> {
        let segs = lines(p);
        let seg = segs[li];
        if li == 0 {
            for item in Self::items(p) {
                if let ItemKind::Object(n) = item.kind
                    && n.a("reflowGeneral") == "1"
                {
                    let anchor_y = oy + seg.n("vertpos") / 100.;
                    self.object(&n, ox, anchor_y, anchor_y, ox, width)?;
                }
            }
        }
        let start = seg.u("textpos");
        let end = segs
            .get(li + 1)
            .map(|n| n.u("textpos"))
            .unwrap_or(usize::MAX);
        let its: Vec<_> = Self::items(p)
            .into_iter()
            .filter(|i| i.pos >= start && i.pos < end)
            .filter(|i| !matches!(&i.kind,ItemKind::Object(n) if n.a("reflowGeneral")=="1"))
            .filter(|i| p.a("reflowed") != "1" || !matches!(i.kind, ItemKind::Char('\n' | '\r')))
            .collect();
        let mut line_y = oy + seg.n("vertpos") / 100.;
        let mut base = line_y + seg.n("baseline") / 100.;
        let mut x = ox + seg.n("horzpos") / 100.;
        let mut avail = if seg.n("horzsize") > 0. {
            seg.n("horzsize") / 100.
        } else {
            width
        };
        if self.depth > 0 {
            avail = avail.min((width - seg.n("horzpos") / 100.).max(0.));
        }
        let style = self.doc.paras.get(p.a("paraPrIDRef"));
        let align = style
            .and_then(|n| n.child("align"))
            .map(|n| n.a("horizontal"))
            .unwrap_or("LEFT");
        let number_draw = if li == 0 {
            self.paragraph_number(p, p.a("reflowed") == "1")?
        } else {
            None
        };
        if p.a("reflowed") != "1"
            && let Some((_, _, reserved)) = &number_draw
        {
            x += reserved;
            avail -= reserved;
        }
        let mut widths = Vec::new();
        let mut gs = Vec::new();
        for i in &its {
            let (g, w) = match &i.kind {
                ItemKind::Char(c) => {
                    let g = self.glyph(*c, &i.style)?;
                    let w = g.width;
                    (Some(g), w)
                }
                ItemKind::Tab(w) => (None, *w),
                ItemKind::Object(n) => {
                    let m = margin(n, "outMargin");
                    let inline = n
                        .child("pos")
                        .map(|p| p.a("treatAsChar") == "1")
                        .unwrap_or(false);
                    (
                        None,
                        if inline {
                            val(n.child("sz"), "width") + m[0] + m[1]
                        } else {
                            0.
                        },
                    )
                }
                ItemKind::Control(n) if n.name == "charOverlay" => {
                    let g = self.glyph('가', &i.style)?;
                    let width = g.size;
                    (Some(g), width)
                }
                ItemKind::Control(n) if n.name == "ruby" => {
                    let g = self.glyph(n.a("main").chars().next().unwrap_or(' '), &i.style)?;
                    let width = self.fonts.width(g.font, n.a("main"), g.size);
                    (Some(g), width)
                }
                ItemKind::Control(n)
                    if n.name == "footNote" || (n.name == "endNote" && number_draw.is_none()) =>
                {
                    let g = self.glyph('1', &i.style)?;
                    let label = self.note_label(n, n.name == "endNote");
                    let w = self.fonts.width(g.font, &label, g.size * 0.6);
                    (Some(g), w)
                }
                ItemKind::Control(n) if n.name == "autoNum" => {
                    let g = self.glyph('1', &i.style)?;
                    let w = self.fonts.width(g.font, &self.auto_label(n), g.size) * g.scale;
                    (Some(g), w)
                }
                ItemKind::Control(_) => (None, 0.),
            };
            gs.push(g);
            widths.push(w);
        }
        // Source negative tracking can exceed a narrow replacement glyph's advance.
        // Preserve distinct glyph ink before fitting the whole line to its cached width.
        for i in 0..its.len().saturating_sub(1) {
            if matches!(its[i].kind, ItemKind::Char(_))
                && matches!(its[i + 1].kind, ItemKind::Char(_))
                && let (Some(left), Some(right)) = (&gs[i], &gs[i + 1])
            {
                widths[i] = widths[i].max(left.ink_right - right.ink_left + 0.02);
            }
        }
        let mut total: f64 = widths.iter().sum();
        if self.doc.header.a("binaryHwp") == "1" {
            base = base.max(line_y + gs.iter().flatten().map(|g| g.raw_ascent).fold(0., f64::max));
        }
        if total > avail && !its.iter().any(|i| matches!(i.kind, ItemKind::Tab(_))) {
            let fixed: f64 = widths
                .iter()
                .zip(&gs)
                .filter(|(_, g)| g.is_none())
                .map(|(w, _)| *w)
                .sum();
            let text_width = total - fixed;
            if text_width > 0. && avail > fixed {
                let factor = (avail - fixed) / text_width;
                for (i, g) in gs.iter_mut().enumerate() {
                    if let Some(g) = g {
                        g.scale *= factor;
                        g.width *= factor;
                        widths[i] *= factor;
                    }
                }
                total = widths.iter().sum();
            }
        }
        let mut gap = 0.;
        if align == "CENTER" {
            x += (avail - total).max(0.) / 2.
        } else if align == "RIGHT" {
            x += (avail - total).max(0.);
        }
        // Never justify through tabs or into the measured boxes of equations/images.
        let atomic_display = its.windows(2).any(|p| p[0].pos == p[1].pos);
        let stretch = if atomic_display {
            its.windows(2)
                .filter(|p| {
                    p[0].pos != p[1].pos
                        && matches!(p[0].kind, ItemKind::Char(_))
                        && matches!(p[1].kind, ItemKind::Char(_))
                })
                .count()
        } else {
            its.iter()
                .filter(|i| matches!(i.kind, ItemKind::Char(_)))
                .count()
                .saturating_sub(1)
        };
        if !its.iter().any(|i| matches!(i.kind, ItemKind::Tab(_)))
            && stretch > 0
            && ((align == "JUSTIFY" && li + 1 < segs.len() && seg.a("hardBreak") != "1")
                || align == "DISTRIBUTE"
                || total > avail)
        {
            gap = (avail - total) / stretch as f64;
        }
        let origin = x;
        let mut caption_cursor = line_y;
        let mut underlines: Vec<(String, f64, f64, f64, f64, f64)> = Vec::new();
        let mut strikes: Vec<(String, f64, f64, f64, f64)> = Vec::new();
        for (idx, it) in its.iter().enumerate() {
            match &it.kind {
                ItemKind::Char(c) => {
                    let g = gs[idx].as_ref().unwrap();
                    if self
                        .doc
                        .chars
                        .get(&it.style)
                        .and_then(|s| s.child("strikeout"))
                        .and_then(crate::decoration::style)
                        .is_some()
                    {
                        let end = x + widths[idx] + gap;
                        if let Some(span) = strikes.last_mut().filter(|s| {
                            s.0 == it.style
                                && (s.2 - x).abs() < 0.01
                                && (s.4 - g.shift).abs() < 0.01
                        }) {
                            span.2 = end;
                        } else {
                            strikes.push((it.style.clone(), x, end, g.size, g.shift));
                        }
                    }
                    if g.shear != 0. {
                        self.current.ops.push(Op::TransformBegin {
                            matrix: [1., 0., -g.shear, 1., g.shear * (base + g.shift), 0.],
                        });
                    }
                    self.current.ops.push(Op::Text {
                        x,
                        y: base + g.shift,
                        text: c.to_string(),
                        font: g.font,
                        size: g.size,
                        scale: g.scale,
                        color: g.color,
                    });
                    if g.shear != 0. {
                        self.current.ops.push(Op::TransformEnd);
                    }
                    if self
                        .doc
                        .chars
                        .get(&it.style)
                        .and_then(|s| s.child("underline"))
                        .filter(|n| matches!(n.a("type"), "BOTTOM" | "TOP"))
                        .is_some()
                    {
                        let end = x + widths[idx] + gap;
                        if let Some(span) = underlines.last_mut().filter(|s| {
                            s.0 == it.style
                                && (s.2 - x).abs() < 0.01
                                && (s.5 - g.shift).abs() < 0.01
                        }) {
                            span.2 = end;
                            span.4 = span.4.max(g.raw_descent);
                        } else {
                            underlines.push((
                                it.style.clone(),
                                x,
                                end,
                                g.size,
                                g.raw_descent,
                                g.shift,
                            ));
                        }
                    }
                    x += widths[idx]
                        + if atomic_display
                            && its.get(idx + 1).is_some_and(|next| next.pos == it.pos)
                        {
                            0.
                        } else {
                            gap
                        };
                }
                ItemKind::Tab(w) => {
                    x += *w;
                }
                ItemKind::Object(n) => {
                    let pos = n.child("pos");
                    let object_start_page = self.pages.len();
                    let block = n.a("textWrap") == "TOP_AND_BOTTOM"
                        && pos.is_some_and(|p| {
                            p.a("treatAsChar") != "1" && p.a("vertRelTo") == "PARA"
                        });
                    let mut object_y = line_y;
                    if block && n.child("caption").is_some() {
                        let e = caption_extent(n);
                        let m = margin(n, "outMargin");
                        let extent = effective_height(n) + e[2] + e[3] + m[2] + m[3];
                        if self.depth == 0
                            && caption_cursor + extent > self.ph - self.bottom
                            && extent < self.ph - self.top - self.bottom
                        {
                            self.finish()?;
                            caption_cursor = self.top;
                        }
                        object_y = caption_cursor;
                        caption_cursor += extent;
                    } else if block
                        && n.name == "pic"
                        && segs[0].n("vertpos") / 100. + 0.01
                            >= effective_height(n) + margin(n, "outMargin")[3]
                        && its
                            .iter()
                            .any(|i| matches!(i.kind, ItemKind::Char(c) if !c.is_whitespace()))
                    {
                        // Block pictures precede their paragraph's first text line in the cache.
                        object_y -= effective_height(n) + margin(n, "outMargin")[3];
                    }
                    self.object(n, ox, object_y, base, x, width)?;
                    if n.name == "tbl"
                        && self.pages.len() > object_start_page
                        && its[idx + 1..]
                            .iter()
                            .any(|i| matches!(i.kind,ItemKind::Char(c) if !c.is_whitespace()))
                    {
                        line_y = self.last_table_end;
                        let reserve = self.footnote_reservation(&[], ox);
                        if line_y + seg.n("vertsize") / 100. > self.ph - self.bottom - reserve {
                            self.finish()?;
                            let s = &self.doc.sections[self.active_section];
                            self.decorations(s, false, self.active_columns, self.active_gap)?;
                            line_y = self.top;
                        }
                        base = line_y + seg.n("baseline") / 100.;
                        x = ox + seg.n("horzpos") / 100.;
                        self.flow_floor =
                            Some((self.pages.len(), line_y + seg.n("vertsize") / 100., true));
                    }
                    x += widths[idx];
                }
                ItemKind::Control(n) => match n.name.as_str() {
                    "charOverlay" => {
                        let g = gs[idx].as_ref().unwrap();
                        let w = widths[idx];
                        let y = base - g.size * 0.85;
                        self.report.bounds.push(Bounds {
                            page: self.pages.len() + 1,
                            kind: "intentional-overlay".into(),
                            id: format!("{}-{}", p.a("id"), it.pos),
                            x,
                            y,
                            w,
                            h: g.size,
                        });
                        let border = n.u("borderType");
                        if border == 1 {
                            for i in 0..48 {
                                let a = i as f64 * std::f64::consts::TAU / 48.;
                                let b = (i + 1) as f64 * std::f64::consts::TAU / 48.;
                                self.current.ops.push(Op::Line {
                                    x: x + w / 2. + w / 2. * a.cos(),
                                    y: y + w / 2. + w / 2. * a.sin(),
                                    x2: x + w / 2. + w / 2. * b.cos(),
                                    y2: y + w / 2. + w / 2. * b.sin(),
                                    width: 0.4,
                                    color: g.color,
                                });
                            }
                        } else if border != 0 {
                            self.box_lines(x, y, w, g.size, 0.4, g.color);
                            if border != 2 {
                                self.report.warnings.insert(format!(
                                    "Overlay border type {border} approximated by rectangle"
                                ));
                            }
                        }
                        let text = n.a("text");
                        let factor = ((g.size + n.n("innerSize")) / g.size).clamp(0.2, 1.);
                        let chars: Vec<String> = if n.a("spread") == "1" {
                            vec![text.into()]
                        } else {
                            text.chars().map(|c| c.to_string()).collect()
                        };
                        for (i, t) in chars.iter().enumerate() {
                            let style = n
                                .children
                                .get(i)
                                .map(|c| c.a("id"))
                                .filter(|s| self.doc.chars.contains_key(*s))
                                .unwrap_or(&it.style);
                            let gg = self.glyph(t.chars().next().unwrap_or(' '), style)?;
                            let size = g.size * factor;
                            let tw = self.fonts.width(gg.font, t, size);
                            let scale = if tw > w { w / tw } else { 1. };
                            self.current.ops.push(Op::Text {
                                x: x + (w - tw * scale) / 2.,
                                y: y + (g.size - size) / 2. + size * 0.85,
                                text: t.clone(),
                                font: gg.font,
                                size,
                                scale,
                                color: gg.color,
                            });
                        }
                        x += w;
                    }
                    "ruby" => {
                        let g = gs[idx].as_ref().unwrap();
                        let w = widths[idx];
                        self.current.ops.push(Op::Text {
                            x,
                            y: base,
                            text: n.a("main").into(),
                            font: g.font,
                            size: g.size,
                            scale: g.scale,
                            color: g.color,
                        });
                        let size = g.size * 0.5;
                        let tw = self.fonts.width(g.font, n.a("sub"), size);
                        self.current.ops.push(Op::Text {
                            x: x + (w - tw) / 2.,
                            y: if n.a("position") == "1" {
                                base + size
                            } else {
                                base - g.size
                            },
                            text: n.a("sub").into(),
                            font: g.font,
                            size,
                            scale: 1.,
                            color: g.color,
                        });
                        x += w;
                    }
                    "footNote" | "endNote" => {
                        let end = n.name == "endNote";
                        let mut note_top = base;
                        if !end || number_draw.is_none() {
                            let g = gs[idx].as_ref().unwrap();
                            note_top = base - g.size * 1.2;
                            self.current.ops.push(Op::Text {
                                x,
                                y: base - g.size * 0.35,
                                text: self.note_label(n, end),
                                font: g.font,
                                size: g.size * 0.6,
                                scale: 1.,
                                color: g.color,
                            });
                            x += widths[idx];
                        }
                        if !self.note_rendering {
                            if end {
                                let mut note = n.clone();
                                note.attrs.insert(
                                    "sourceSection".into(),
                                    self.active_section.to_string(),
                                );
                                self.notes.push(note);
                            } else {
                                let (nx, nw) = self.footnote_area(ox);
                                if self.capture_table_headers {
                                    self.table_note_anchors.push(PlannedTableNote {
                                        node: n.clone(),
                                        x: nx,
                                        width: nw,
                                        y: base,
                                        top: note_top,
                                        op_index: self.current.ops.len() - 1,
                                    });
                                } else {
                                    self.footnotes.push((n.clone(), nx, nw));
                                }
                            }
                        }
                    }
                    "autoNum" => {
                        let num = self.auto_label(n);
                        let g = gs[idx].as_ref().unwrap();
                        self.current.ops.push(Op::Text {
                            x,
                            y: base,
                            text: num,
                            font: g.font,
                            size: g.size,
                            scale: g.scale,
                            color: g.color,
                        });
                        x += widths[idx];
                    }
                    _ => {}
                },
            }
        }
        for (style, x1, x2, size, descent, shift) in underlines {
            let def = self.doc.chars[&style].child("underline").unwrap();
            let Some(line_style) = crate::decoration::style(def) else {
                if def.a("shape") != "NONE" {
                    self.report.warnings.insert(format!(
                        "Unsupported underline shape: {} {}",
                        def.a("shape"),
                        def.a("style")
                    ));
                }
                continue;
            };
            let y = if def.a("type") == "TOP" {
                base + shift - size
            } else {
                base + shift + descent + 0.9
            };
            self.current.ops.extend(crate::decoration::underline(
                x1,
                x2,
                y,
                line_style,
                color(def.a("color")),
            ));
        }
        for (style, x1, x2, size, shift) in strikes {
            let def = self.doc.chars[&style].child("strikeout").unwrap();
            self.current.ops.extend(crate::decoration::underline(
                x1,
                x2,
                base + shift - size * 0.3,
                crate::decoration::style(def).unwrap(),
                color(def.a("color")),
            ));
        }
        if let Some((label, g, _)) = number_draw {
            self.current.ops.push(Op::Text {
                x: ox
                    + if p.a("reflowed") == "1" {
                        p.n("reflowNumberX")
                    } else {
                        0.
                    },
                y: base,
                text: label,
                font: g.font,
                size: g.size,
                scale: g.scale,
                color: g.color,
            });
        }
        self.report.bounds.push(Bounds {
            page: self.pages.len() + 1,
            kind: "line".into(),
            id: p.a("id").into(),
            x: origin,
            y: line_y,
            w: (x - origin).max(0.),
            h: seg.n("vertsize") / 100.,
        });
        Ok(())
    }
    fn local(&mut self, list: &Node, x: f64, y: f64, w: f64) -> Result<()> {
        self.depth += 1;
        if self.depth > 64 {
            bail!("Excessive nested layout")
        }
        for (p, i, flow_y, _) in self.measured_flow(list)? {
            self.line(p, i, x, y + flow_y - lines(p)[i].n("vertpos") / 100., w)?;
        }
        self.depth -= 1;
        Ok(())
    }
    fn measured_height(&mut self, list: &Node) -> Result<f64> {
        Ok(self
            .measured_flow(list)?
            .iter()
            .map(|(_, _, y, h)| y + h)
            .fold(0f64, f64::max))
    }
    fn measured_flow<'b>(&mut self, list: &'b Node) -> Result<Vec<(&'b Node, usize, f64, f64)>> {
        if self.doc.header.a("binaryHwp") != "1"
            || list.children_named("p").all(|p| p.a("reflowed") == "1")
        {
            return Ok(flowing_lines(list)
                .into_iter()
                .map(|(p, i, y)| (p, i, y, lines(p)[i].n("vertsize") / 100.))
                .collect());
        }
        let mut out = Vec::new();
        let mut bottom = 0f64;
        for (p, i, y) in flowing_lines(list) {
            let ls = lines(p);
            let l = ls[i];
            let items = Self::items(p);
            let end = ls.get(i + 1).map(|n| n.u("textpos")).unwrap_or(usize::MAX);
            let mut ascent = l.n("baseline") / 100.;
            let mut descent = 0f64;
            for it in items
                .iter()
                .filter(|it| it.pos >= l.u("textpos") && it.pos < end)
            {
                if let ItemKind::Char(c) = it.kind {
                    let g = self.glyph(c, &it.style)?;
                    ascent = ascent.max(g.ascent);
                    descent = descent.max(g.descent);
                }
            }
            let h = (ascent + descent).max(l.n("vertsize") / 100.);
            let actual = y.max(bottom);
            out.push((p, i, actual, h));
            bottom = actual + h;
        }
        Ok(out)
    }
    fn object(
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
            (if vr == "PAPER" {
                0.
            } else if vr == "PAGE" {
                self.top
            } else {
                line_y
            }) + pos.signed("vertOffset") / 100.
                * if pos.a("vertAlign") == "BOTTOM" {
                    -1.
                } else {
                    1.
                }
                + match pos.a("vertAlign") {
                    "CENTER" => (extent - h) / 2.,
                    "BOTTOM" => extent - h,
                    _ => 0.,
                }
        };
        if !inline && n.child("caption").is_some() && n.a("binaryHwp") == "1" {
            let e = caption_extent(n);
            x += m[0] + e[0];
            y += m[2] + e[2];
            // Position the combined caption/object box inside its containing column.
            x = x
                .max(ox + e[0])
                .min((ox + container - w - e[1]).max(ox + e[0]));
        }
        let fit_start = self.report.bounds.len();
        let fit = if self.depth == 0
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
        self.report.bounds.push(Bounds {
            page: self.pages.len() + 1,
            kind: if n.a("textWrap") == "BEHIND_TEXT" && n.name == "pic" {
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
                for child in n.children.iter().filter(|c| c.child("pos").is_some()) {
                    self.object(child, x, y, y, x, w)?;
                }
            }
            "polygon" | "curve" | "arc" => {
                let ow = if n.n("originalWidth") > 0. {
                    n.n("originalWidth")
                } else {
                    n.child("orgSz").map(|s| s.n("width")).unwrap_or(w * 100.)
                };
                let oh = if n.n("originalHeight") > 0. {
                    n.n("originalHeight")
                } else {
                    n.child("orgSz").map(|s| s.n("height")).unwrap_or(h * 100.)
                };
                let sx = w / (ow / 100.).max(0.01);
                let sy = h / (oh / 100.).max(0.01);
                let mut points: Vec<_> = n
                    .children
                    .iter()
                    .filter(|p| p.name == "point" || p.name == "pt")
                    .map(|p| (x + p.n("x") / 100. * sx, y + p.n("y") / 100. * sy))
                    .collect();
                let mut kinds: Vec<_> = n.children_named("segment").map(|s| s.u("type")).collect();
                if points.is_empty() && n.name == "curve" {
                    for seg in n.children_named("seg") {
                        if points.is_empty() {
                            points.push((x + seg.n("x1") / 100. * sx, y + seg.n("y1") / 100. * sy));
                        }
                        points.push((x + seg.n("x2") / 100. * sx, y + seg.n("y2") / 100. * sy));
                        kinds.push(usize::from(seg.a("type") == "CURVE"));
                    }
                }
                if n.name == "arc" {
                    let c = n.child("center").context("arc center")?;
                    let a = n.child("axis1").context("arc axis")?;
                    let b = n.child("axis2").context("arc axis")?;
                    for i in 0..=32 {
                        let t = i as f64 * std::f64::consts::FRAC_PI_2 / 32.;
                        points.push((
                            x + (c.n("x")
                                + (a.n("x") - c.n("x")) * t.cos()
                                + (b.n("x") - c.n("x")) * t.sin())
                                / 100.
                                * sx,
                            y + (c.n("y")
                                + (a.n("y") - c.n("y")) * t.cos()
                                + (b.n("y") - c.n("y")) * t.sin())
                                / 100.
                                * sy,
                        ));
                    }
                }
                let line = n.child("lineShape");
                let fill = n.find("winBrush").map(|f| color(f.a("faceColor")));
                if n.name == "curve" && !points.is_empty() {
                    let segments = crate::graphics::curve(&points, &kinds);
                    self.current.ops.push(Op::Bezier {
                        start: points[0],
                        segments,
                        closed: points.first() == points.last(),
                        width: val(line, "width").max(0.1),
                        color: color(line.map(|s| s.a("color")).unwrap_or("#000000")),
                        fill,
                    });
                    self.report.warnings.insert("Curve knots rendered as cardinal spline (tension 0.5); exact Hancom interpolation unverified".into());
                } else {
                    self.current.ops.push(Op::Path {
                        points,
                        closed: n.name == "polygon",
                        width: val(line, "width").max(0.1),
                        color: color(line.map(|s| s.a("color")).unwrap_or("#000000")),
                        fill,
                    });
                }
            }
            "equation" => {
                let script = n.child("script").map(text).unwrap_or_default();
                let b = math::layout(&script, n.n("baseUnit") / 100., &mut self.fonts)?;
                self.current
                    .ops
                    .extend(math::place(b, x, y + h * n.n("baseLine") / 100., w, h));
                self.report.equations += 1;
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
                if let Some(fill) = n.find("winBrush") {
                    self.current.ops.push(Op::Rect {
                        x,
                        y,
                        w,
                        h,
                        fill: color(fill.a("faceColor")),
                    });
                }
                let s = n.child("lineShape");
                if s.is_none_or(|s| s.a("style") != "NONE") {
                    self.box_lines(
                        x,
                        y,
                        w,
                        h,
                        val(s, "width").max(0.1),
                        color(s.map(|n| n.a("color")).unwrap_or("#000000")),
                    );
                }
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
        for caption in n.children_named("caption") {
            if captions_handled {
                continue;
            }
            if let Some(list) = caption.child("subList") {
                let gap = caption.n("gap") / 100.;
                let ch = content_height(list);
                let cw = if matches!(caption.a("side"), "LEFT" | "RIGHT") {
                    caption.n("width") / 100.
                } else if caption.a("expandToMargin") == "1" {
                    w + m[0] + m[1]
                } else {
                    w
                };
                let cy = y + match list.a("vertAlign") {
                    "CENTER" => (h - ch) / 2.,
                    "BOTTOM" => h - ch,
                    _ => 0.,
                };
                let (cx, cy) = match caption.a("side") {
                    "LEFT" => (x - gap - cw, cy),
                    "RIGHT" => (x + w + gap, cy),
                    "TOP" => (x - (cw - w) / 2., y - gap - ch),
                    _ => (x - (cw - w) / 2., y + h + gap),
                };
                self.local(list, cx, cy, cw)?;
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
    fn box_lines(&mut self, x: f64, y: f64, w: f64, h: f64, width: f64, c: [f64; 3]) {
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
    fn border(&mut self, id: &str, x: f64, y: f64, w: f64, h: f64) {
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
    fn table_slices(
        &mut self,
        n: &Node,
        cells: &[&Node],
        cw: &[f64],
        rh: &[f64],
        x: f64,
        mut top: f64,
    ) -> Result<()> {
        let total = rh.iter().sum::<f64>();
        let mut contents = Vec::new();
        for c in cells {
            let a = c.child("cellAddr").unwrap();
            let s = c.child("cellSpan").unwrap();
            let row = a.u("rowAddr");
            let col = a.u("colAddr");
            let cy = rh[..row].iter().sum::<f64>();
            let ch = rh[row..row + s.u("rowSpan")].iter().sum::<f64>();
            let cx = x + cw[..col].iter().sum::<f64>();
            let ww = cw[col..col + s.u("colSpan")].iter().sum::<f64>();
            let m = if c.a("hasMargin") == "1" {
                margin(c, "cellMargin")
            } else {
                margin(n, "inMargin")
            };
            let mut flow = Vec::new();
            if let Some(l) = c.child("subList") {
                let (min, max) = floating_extent(l);
                let spare = (ch - m[2] - m[3] - (max - min)).max(0.);
                let shift = match l.a("vertAlign") {
                    "CENTER" => spare / 2.,
                    "BOTTOM" => spare,
                    _ => 0.,
                };
                for (p, i, yy) in flowing_lines(l) {
                    flow.push((p, i, cy + m[2] + shift - min + yy));
                }
            }
            contents.push((*c, cx, cy, ww, ch, m, flow));
        }
        let mut start = 0.;
        while start < total - 0.01 {
            if start > 0. && n.a("repeatHeader") == "1" {
                let header_cells: Vec<Node> = cells
                    .iter()
                    .filter(|c| c.child("cellAddr").unwrap().u("rowAddr") == 0)
                    .map(|c| (*c).clone())
                    .collect();
                if !header_cells.is_empty()
                    && header_cells.iter().all(|c| {
                        c.child("cellSpan").unwrap().u("rowSpan") == 1
                            && c.find("footNote").is_none()
                    })
                    && rh[0] < (self.ph - self.top - self.bottom) / 3.
                {
                    let mut header = n.clone();
                    header.attrs.insert("rowCnt".into(), "1".into());
                    header.attrs.insert("repeatHeader".into(), "0".into());
                    header.attrs.insert("pageBreak".into(), "NONE".into());
                    header.children.retain(|c| c.name != "tr");
                    header.children.push(Node {
                        name: "tr".into(),
                        children: header_cells,
                        ..Default::default()
                    });
                    self.depth += 1;
                    self.table(&header, x, top, cw.iter().sum(), rh[0])?;
                    self.depth -= 1;
                    top += rh[0];
                }
            }
            let mut cut = (start + self.ph - self.bottom - top).min(total);
            for _ in 0..64 {
                let mut notes = Vec::new();
                for (_, _, _, _, _, _, flow) in &contents {
                    for (p, i, yy) in flow {
                        if *yy >= start - 0.01 && *yy < cut {
                            let ls = lines(p);
                            let end = ls.get(i + 1).map(|l| l.u("textpos")).unwrap_or(usize::MAX);
                            for it in Self::items(p) {
                                if it.pos >= ls[*i].u("textpos")
                                    && it.pos < end
                                    && let ItemKind::Control(n) = it.kind
                                    && n.name == "footNote"
                                {
                                    notes.push(n);
                                }
                            }
                        }
                    }
                }
                let refs: Vec<_> = notes.iter().collect();
                let reserve = self.footnote_reservation(&refs, x);
                let mut next = (start + self.ph - self.bottom - top - reserve).min(cut);
                for (_, _, _, _, _, _, flow) in &contents {
                    for (p, i, yy) in flow {
                        let end = yy + lines(p)[*i].n("vertsize") / 100. + 0.5;
                        if *yy < next && end > next {
                            next = next.min(*yy);
                        }
                    }
                }
                if (next - cut).abs() < 0.01 {
                    break;
                }
                cut = next;
            }
            if cut <= start + 0.1 {
                if top > self.top + 0.1 {
                    self.finish()?;
                    let sec = &self.doc.sections[self.active_section];
                    self.decorations(sec, false, self.active_columns, self.active_gap)?;
                    top = self.top;
                    continue;
                }
                bail!("Table line with footnotes exceeds a full page");
            }
            for (c, cx, cy, ww, ch, m, flow) in &contents {
                let a = cy.max(start);
                let b = (cy + ch).min(cut);
                if b <= a {
                    continue;
                }
                self.border(c.a("borderFillIDRef"), *cx, top + a - start, *ww, b - a);
                self.depth += 1;
                for (p, i, yy) in flow {
                    if *yy >= start - 0.01 && *yy < cut - 0.01 {
                        self.line(
                            p,
                            *i,
                            cx + m[0],
                            top + yy - start - lines(p)[*i].n("vertpos") / 100.,
                            ww - m[0] - m[1],
                        )?;
                    }
                }
                self.depth -= 1;
            }
            self.last_table_end = top + cut - start;
            start = cut;
            if start < total - 0.01 {
                self.finish()?;
                let sec = &self.doc.sections[self.active_section];
                self.decorations(sec, false, self.active_columns, self.active_gap)?;
                top = self.top;
            }
        }
        self.report.warnings.insert("Oversized table row split at shared text-line boundaries; merged cell borders continued".into());
        Ok(())
    }
    /// Reflow consumes columns before advancing a page; saved layouts keep
    /// their original page-only splitting behavior.
    fn advance_table_fragment(&mut self, x: &mut f64) -> Result<()> {
        if let Some(col) = self.reflow_table_column {
            let width = (self.pw
                - self.left
                - self.right
                - self.active_gap * (self.active_columns - 1) as f64)
                / self.active_columns as f64;
            let next = (col + 1) % self.active_columns;
            *x += (next as f64 - col as f64) * (width + self.active_gap);
            self.reflow_table_column = Some(next);
            if next != 0 {
                return Ok(());
            }
        }
        self.finish()?;
        let section = &self.doc.sections[self.active_section];
        self.decorations(section, false, self.active_columns, self.active_gap)?;
        Ok(())
    }
    /// Paginate measured vectors at safe row/ink boundaries. Only rules and
    /// fills are clipped; glyphs and transformed objects remain indivisible.
    fn split_nested_vectors(
        &mut self,
        n: &Node,
        mut x: f64,
        mut top: f64,
        w: f64,
        total: f64,
    ) -> Result<bool> {
        let canvas_x = x;
        let captions: Vec<_> = n.children_named("caption").collect();
        let mut all_captions = Vec::new();
        n.all("caption", &mut all_captions);
        if all_captions.len() != captions.len()
            || captions
                .iter()
                .any(|c| !matches!(c.a("side"), "TOP" | "BOTTOM"))
        {
            return Ok(false);
        }
        let caption_height = |side: &str| {
            captions
                .iter()
                .filter(|c| c.a("side") == side)
                .map(|c| c.child("subList").map(content_height).unwrap_or(0.) + c.n("gap") / 100.)
                .sum::<f64>()
        };
        let above = caption_height("TOP");
        let below = caption_height("BOTTOM");
        if above + below > (self.ph - self.top - self.bottom) * 0.5 {
            return Ok(false);
        }
        if n.find("endNote").is_some()
            || n.child("rotationInfo").is_some_and(|r| r.n("angle") != 0.)
            || n.child("flip")
                .is_some_and(|r| r.a("horizontal") == "1" || r.a("vertical") == "1")
        {
            return Ok(false);
        }
        let previous = std::mem::take(&mut self.current);
        let mut table_nodes = Vec::new();
        n.all("tbl", &mut table_nodes);
        let requested_headers = table_nodes
            .iter()
            .filter(|t| t.a("repeatHeader") == "1")
            .count();
        let bound_start = self.report.bounds.len();
        let counts = (
            self.report.tables,
            self.report.pictures,
            self.report.equations,
            self.report.charts,
        );
        self.capture_table_headers = true;
        self.table_headers.clear();
        self.table_note_anchors.clear();
        self.depth += 1;
        let rendered = self.table(n, x, 0., w, total);
        self.depth -= 1;
        self.capture_table_headers = false;
        let mut headers = std::mem::take(&mut self.table_headers);
        let mut planned_notes = std::mem::take(&mut self.table_note_anchors);
        headers.sort_by(|a, b| a.y.total_cmp(&b.y));
        let canvas = std::mem::replace(&mut self.current, previous);
        let bounds = self.report.bounds.split_off(bound_start);
        rendered?;
        let atoms = crate::pagination::atoms(&canvas.ops, &self.fonts)?;
        let supported = atoms.iter().all(|a| {
            a.bottom - a.top
                < self.ph
                    - self.top
                    - self.bottom
                    - above
                    - below
                    - headers.iter().map(|h| h.height).sum::<f64>()
        });
        let mut owner = vec![None; canvas.ops.len()];
        for (id, atom) in atoms.iter().enumerate() {
            for slot in &mut owner[atom.start..atom.end] {
                *slot = Some(id);
            }
        }
        for note in &mut planned_notes {
            if let Some(id) = owner[note.op_index] {
                let atom = &atoms[id];
                note.top = atom.top;
                note.y = (atom.top + atom.bottom) / 2.;
            }
        }
        if !supported
            || headers.len() != requested_headers
            || headers.iter().map(|h| h.height).sum::<f64>()
                > self.ph - self.top - self.bottom - above - below - 24.
        {
            (
                self.report.tables,
                self.report.pictures,
                self.report.equations,
                self.report.charts,
            ) = counts;
            return Ok(false);
        }
        let mut ink = Vec::new();
        ink.extend(atoms.iter().map(|a| (a.top - 0.6, a.bottom + 0.6)));
        ink.extend(planned_notes.iter().map(|n| (n.top, n.y + 0.6)));
        ink.extend(
            bounds
                .iter()
                .filter(|b| {
                    matches!(
                        b.kind.as_str(),
                        "pic" | "rect" | "group" | "polygon" | "curve" | "arc" | "equation"
                    )
                })
                .map(|b| (b.y - 0.6, b.y + b.h + 0.6)),
        );
        let mut boundaries = Vec::new();
        for (op_index, op) in canvas.ops.iter().enumerate() {
            if owner[op_index].is_some() {
                continue;
            }
            match op {
                Op::Text {
                    font,
                    size,
                    x,
                    scale,
                    y,
                    text,
                    ..
                } => {
                    let face = ttf_parser::Face::parse(&self.fonts.list[*font].bytes, 0).unwrap();
                    let unit = size / face.units_per_em() as f64;
                    let right = x + self.fonts.width(*font, text, *size) * scale;
                    let decoration_bottom = canvas
                        .ops
                        .iter()
                        .filter_map(|o| match o {
                            Op::Path { points, .. }
                                if points.len() == 2
                                    && points[0].1 >= *y
                                    && points[0].1 < y + size * 0.5 + 4.
                                    && points[0].0.min(points[1].0) < right
                                    && points[0].0.max(points[1].0) > *x =>
                            {
                                Some(points[0].1 + 0.6)
                            }
                            _ => None,
                        })
                        .fold(*y, f64::max);
                    let decoration_top = canvas
                        .ops
                        .iter()
                        .filter_map(|o| match o {
                            Op::Path { points, .. }
                                if points.len() == 2
                                    && points[0].1 <= *y
                                    && points[0].1 > y - size * 1.1
                                    && points[0].0.min(points[1].0) < right
                                    && points[0].0.max(points[1].0) > *x =>
                            {
                                Some(points[0].1 - 0.6)
                            }
                            _ => None,
                        })
                        .fold(*y, f64::min);
                    for c in text.chars().filter(|c| !c.is_whitespace()) {
                        if let Some(bb) = crate::pdf::glyph_id(&face, c)
                            .and_then(|id| face.glyph_bounding_box(id))
                        {
                            ink.push((
                                (y - bb.y_max as f64 * unit).min(decoration_top) - 0.6,
                                (y - bb.y_min as f64 * unit).max(decoration_bottom) + 0.6,
                            ));
                        }
                    }
                }
                Op::Line { y, y2, .. } if (y - y2).abs() < 0.001 => boundaries.push(*y),
                _ => {}
            }
        }
        let safe = |cut: f64| !ink.iter().any(|(a, b)| *a < cut && *b > cut);
        let mut start = 0.;
        let mut first = true;
        top = top.max(self.top + above);
        let first_min = headers.iter().map(|h| h.y + h.height).fold(0., f64::max) + 24.;
        if top + first_min > self.ph - self.bottom {
            self.advance_table_fragment(&mut x)?;
            top = self.top + above;
        }
        let mut caption_y = top - above;
        for c in captions.iter().filter(|c| c.a("side") == "TOP") {
            if let Some(list) = c.child("subList") {
                let h = content_height(list);
                self.local(list, x, caption_y, w)?;
                self.report.bounds.push(Bounds {
                    page: self.pages.len() + 1,
                    kind: "caption".into(),
                    id: n.a("id").into(),
                    x,
                    y: caption_y,
                    w,
                    h,
                });
                caption_y += h + c.n("gap") / 100.;
            }
        }
        while start < total - 0.01 {
            let mut header_height = 0.;
            let mut prior_y = None;
            let mut group_top = top;
            let mut group_height = 0f64;
            if !first {
                for header in headers
                    .iter()
                    .filter(|h| start >= h.y + h.height - 0.01 && start < h.end - 0.01)
                {
                    if prior_y.is_some_and(|y: f64| (y - header.y).abs() > 0.01) {
                        header_height += group_height;
                        group_top = top + header_height;
                        group_height = 0.;
                    }
                    prior_y = Some(header.y);
                    group_height = group_height.max(header.height);
                    self.current.ops.push(Op::TransformBegin {
                        matrix: [1., 0., 0., 1., x - canvas_x, group_top - header.y],
                    });
                    self.current.ops.extend(header.ops.clone());
                    self.current.ops.push(Op::TransformEnd);
                }
                header_height += group_height;
            }
            top += header_height;
            let mut capacity = self.ph - self.bottom - top - self.footnote_reservation(&[], x);
            if start + capacity >= total {
                capacity -= below;
            }
            let mut cut = (start + capacity).min(total);
            for _ in 0..planned_notes.len() + 2 {
                let incoming: Vec<_> = planned_notes
                    .iter()
                    .filter(|n| n.y >= start - 0.01 && n.y < cut)
                    .map(|n| &n.node)
                    .collect();
                let reserve = self.footnote_reservation(&incoming, x);
                let next = (start + self.ph
                    - self.bottom
                    - top
                    - reserve
                    - if start + capacity >= total { below } else { 0. })
                .min(cut);
                if (next - cut).abs() < 0.01 {
                    break;
                }
                cut = next;
            }
            if cut < total - 0.01 {
                if let Some(row) = boundaries
                    .iter()
                    .copied()
                    .filter(|v| *v > start + capacity * 0.5 && *v <= cut && safe(*v))
                    .max_by(f64::total_cmp)
                {
                    cut = row;
                } else {
                    for _ in 0..ink.len() + 1 {
                        let next = ink
                            .iter()
                            .filter(|(a, b)| *a < cut && *b > cut)
                            .map(|(a, _)| *a)
                            .fold(cut, f64::min);
                        if (next - cut).abs() < 0.001 {
                            break;
                        }
                        cut = next;
                    }
                }
            }
            if below > 0.
                && cut < total - 0.01
                && !ink.iter().any(|(a, _)| *a >= cut)
                && let Some(last) = ink.iter().map(|(a, _)| *a).max_by(f64::total_cmp)
            {
                cut = boundaries
                    .iter()
                    .copied()
                    .filter(|b| *b > start + 0.1 && *b <= last && safe(*b))
                    .max_by(f64::total_cmp)
                    .unwrap_or(last);
                for _ in 0..ink.len() + 1 {
                    let next = ink
                        .iter()
                        .filter(|(a, b)| *a < cut && *b > cut)
                        .map(|(a, _)| *a)
                        .fold(cut, f64::min);
                    if (next - cut).abs() < 0.001 {
                        break;
                    }
                    cut = next;
                }
            }
            if cut <= start + 0.1 {
                anyhow::ensure!(
                    top > self.top + header_height + 0.1,
                    "Nested-table text line exceeds printable page"
                );
                self.advance_table_fragment(&mut x)?;
                top = self.top;
                continue;
            }
            let shift = top - start;
            for note in planned_notes
                .iter()
                .filter(|n| n.y >= start - 0.01 && n.y < cut - 0.01)
            {
                let (nx, nw) = if self.reflow_table_column.is_some() {
                    self.footnote_area(x)
                } else {
                    (note.x, note.width)
                };
                self.footnotes.push((note.node.clone(), nx, nw));
            }
            self.current.ops.push(Op::TransformBegin {
                matrix: [1., 0., 0., 1., x - canvas_x, shift],
            });
            for (op_index, op) in canvas.ops.iter().enumerate() {
                if let Some(id) = owner[op_index] {
                    let atom = &atoms[id];
                    if op_index == atom.start
                        && atom.bottom > start - 0.001
                        && atom.top < cut - 0.001
                    {
                        self.current
                            .ops
                            .extend_from_slice(&canvas.ops[atom.start..atom.end]);
                    }
                    continue;
                }
                let clipped = match op {
                    Op::Text { y, .. } => {
                        if *y >= start && *y < cut {
                            Some(op.clone())
                        } else {
                            None
                        }
                    }
                    Op::Line {
                        x,
                        y,
                        x2,
                        y2,
                        width,
                        color,
                    } => {
                        if (y - y2).abs() < 0.001 {
                            if *y >= start - 0.001 && *y <= cut + 0.001 {
                                Some(op.clone())
                            } else {
                                None
                            }
                        } else {
                            let a = y.min(*y2).max(start);
                            let b = y.max(*y2).min(cut);
                            (b > a).then_some(Op::Line {
                                x: *x,
                                y: a,
                                x2: *x2,
                                y2: b,
                                width: *width,
                                color: *color,
                            })
                        }
                    }
                    Op::Rect { x, y, w, h, fill } => {
                        let a = y.max(start);
                        let b = (y + h).min(cut);
                        (b > a).then_some(Op::Rect {
                            x: *x,
                            y: a,
                            w: *w,
                            h: b - a,
                            fill: *fill,
                        })
                    }
                    Op::Path { points, .. } => {
                        if points.first().is_some_and(|p| p.1 >= start && p.1 < cut) {
                            Some(op.clone())
                        } else {
                            None
                        }
                    }
                    _ => None,
                };
                if let Some(op) = clipped {
                    self.current.ops.push(op);
                }
            }
            self.current.ops.push(Op::TransformEnd);
            for b in &bounds {
                let a = b.y.max(start);
                let end = (b.y + b.h).min(cut);
                if end > a {
                    self.report.bounds.push(Bounds {
                        page: self.pages.len() + 1,
                        kind: b.kind.clone(),
                        id: b.id.clone(),
                        x: b.x + x - canvas_x,
                        y: a + shift,
                        w: b.w,
                        h: end - a,
                    });
                }
            }
            if first {
                if let Some(b) = self.report.bounds[..bound_start]
                    .iter_mut()
                    .rev()
                    .find(|b| b.kind == "tbl" && b.id == n.a("id"))
                {
                    b.x = x;
                    b.y = top;
                    b.page = self.pages.len() + 1;
                    b.h = cut - start;
                }
            } else {
                self.report.bounds.push(Bounds {
                    page: self.pages.len() + 1,
                    kind: "tbl".into(),
                    id: n.a("id").into(),
                    x,
                    y: top,
                    w,
                    h: cut - start,
                });
            }
            self.last_table_end = top + cut - start;
            start = cut;
            first = false;
            if start < total - 0.01 {
                self.advance_table_fragment(&mut x)?;
                top = self.top;
            }
        }
        let mut caption_y = self.last_table_end;
        for c in captions.iter().filter(|c| c.a("side") == "BOTTOM") {
            if let Some(list) = c.child("subList") {
                caption_y += c.n("gap") / 100.;
                let h = content_height(list);
                self.local(list, x, caption_y, w)?;
                self.report.bounds.push(Bounds {
                    page: self.pages.len() + 1,
                    kind: "caption".into(),
                    id: n.a("id").into(),
                    x,
                    y: caption_y,
                    w,
                    h,
                });
                caption_y += h;
            }
        }
        self.last_table_end = caption_y;
        self.split_caption_handled = !captions.is_empty();
        self.flow_floor = Some((self.pages.len(), self.last_table_end, true));
        self.report.warnings.insert("Nested text table split at safe row/ink boundaries without scaling; border/fill fragments continued".into());
        Ok(true)
    }
    fn table(&mut self, n: &Node, x: f64, y: f64, w: f64, h: f64) -> Result<()> {
        let rows = n.u("rowCnt");
        let cols = n.u("colCnt");
        if rows == 0 || cols == 0 || rows > 1000 || cols > 1000 {
            bail!("Invalid table dimensions")
        }
        let cells: Vec<_> = n
            .children_named("tr")
            .flat_map(|r| r.children_named("tc"))
            .collect();
        let mut cw = vec![0f64; cols];
        let mut rh = vec![0f64; rows];
        let mut minimum = vec![0f64; rows];
        for c in &cells {
            let a = c.child("cellAddr").context("cellAddr")?;
            let span = c.child("cellSpan").context("cellSpan")?;
            let sz = c.child("cellSz").context("cellSz")?;
            let col = a.u("colAddr");
            let row = a.u("rowAddr");
            let cs = span.u("colSpan");
            let rs = span.u("rowSpan");
            if cs == 0 || rs == 0 || col + cs > cols || row + rs > rows {
                bail!("Cell outside table")
            }
            if cs == 1 {
                cw[col] = cw[col].max(sz.n("width") / 100.)
            }
            if rs == 1 {
                rh[row] = rh[row].max(sz.n("height") / 100.)
            }
        }
        for _ in 0..2 {
            for c in &cells {
                let a = c.child("cellAddr").unwrap();
                let span = c.child("cellSpan").unwrap();
                let sz = c.child("cellSz").unwrap();
                for (v, start, count, size) in [
                    (
                        &mut cw,
                        a.u("colAddr"),
                        span.u("colSpan"),
                        sz.n("width") / 100.,
                    ),
                    (
                        &mut rh,
                        a.u("rowAddr"),
                        span.u("rowSpan"),
                        sz.n("height") / 100.,
                    ),
                ] {
                    let missing: Vec<_> = (start..start + count).filter(|&i| v[i] == 0.).collect();
                    if !missing.is_empty() {
                        let value = (size - v[start..start + count].iter().sum::<f64>()).max(0.)
                            / missing.len() as f64;
                        for i in missing {
                            v[i] = value;
                        }
                    }
                }
            }
        }
        for c in &cells {
            let a = c.child("cellAddr").unwrap();
            let span = c.child("cellSpan").unwrap();
            let row = a.u("rowAddr");
            let rs = span.u("rowSpan");
            let m = if c.a("hasMargin") == "1" {
                margin(c, "cellMargin")
            } else {
                margin(n, "inMargin")
            };
            let content = c
                .child("subList")
                .map(|l| {
                    let (a, b) = floating_extent(l);
                    b - a
                })
                .unwrap_or(0.);
            let typed = if let Some(list) = c.child("subList") {
                self.measured_height(list)?
            } else {
                0.
            };
            let needed = content.max(typed) + m[2] + m[3];
            if rs == 1 {
                minimum[row] = minimum[row].max(needed);
            } else {
                let missing = (needed - minimum[row..row + rs].iter().sum::<f64>()).max(0.);
                minimum[row + rs - 1] += missing;
            }
            let current: f64 = rh[row..row + rs].iter().sum();
            if needed > current {
                rh[row + rs - 1] += needed - current;
            }
        }
        let mut total: f64 = rh.iter().sum();
        let min_total: f64 = minimum.iter().sum();
        if n.a("binaryHwp") == "1" && total > h && h >= min_total && total > min_total {
            let ratio = (h - min_total) / (total - min_total);
            for (i, height) in rh.iter_mut().enumerate() {
                *height = minimum[i] + (*height - minimum[i]) * ratio;
            }
            total = h;
        }
        if total < h
            && let Some(last) = rh.last_mut()
        {
            *last += h - total;
        }
        if self.depth == 0 && self.reflow_table_column.is_some() {
            anyhow::ensure!(
                self.split_nested_vectors(n, x, y, w, total)?,
                "This table structure cannot yet be split across reflow columns"
            );
            return Ok(());
        }
        let fixed_bottom = n.child("pos").is_some_and(|p| p.a("vertAlign") == "BOTTOM");
        let nested = cells.iter().filter_map(|c| c.child("subList")).any(|l| {
            l.children_named("p").any(|p| {
                Self::items(p)
                    .iter()
                        .any(|i| matches!(&i.kind, ItemKind::Object(o) if effective_height(o) > self.ph-self.top-self.bottom))
            })
        });
        if self.depth == 0 && nested && total > self.ph - self.top - self.bottom {
            if self.split_nested_vectors(n, x, y, w, total)? {
                return Ok(());
            }
            let target_y = y.max(self.top).min(self.ph - self.bottom - 1.);
            let factor = ((self.ph - self.bottom - target_y) / total).min(1.);
            let matrix = [
                factor,
                0.,
                0.,
                factor,
                x * (1. - factor),
                target_y - factor * y,
            ];
            let begin = self.report.bounds.len();
            self.current.ops.push(Op::TransformBegin { matrix });
            self.depth += 1;
            self.table(n, x, y, w, total)?;
            self.depth -= 1;
            self.current.ops.push(Op::TransformEnd);
            for b in &mut self.report.bounds[begin..] {
                let r = crate::graphics::rect(matrix, [b.x, b.y, b.x + b.w, b.y + b.h]);
                b.x = r[0];
                b.y = r[1];
                b.w = r[2] - r[0];
                b.h = r[3] - r[1];
            }
            self.last_table_end = target_y + total * factor;
            self.flow_floor = Some((self.pages.len(), self.last_table_end, false));
            if let Some(b) = self.report.bounds[..begin]
                .iter_mut()
                .rev()
                .find(|b| b.kind == "tbl" && b.id == n.a("id") && b.page == self.pages.len() + 1)
            {
                b.x = x;
                b.y = target_y;
                b.w = w * factor;
                b.h = total * factor;
            }
            self.report.warnings.insert(format!("Oversized nested table {} fitted proportionally at {:.1}%; structural splitting remains limited",n.a("id"),factor*100.));
            return Ok(());
        }
        if self.depth == 0
            && !fixed_bottom
            && cells.len() > 1
            && rh.iter().any(|h| *h > self.ph - self.top - self.bottom)
            && cells
                .iter()
                .filter_map(|c| c.child("subList"))
                .flat_map(|l| l.children_named("p"))
                .all(|p| {
                    Self::items(p)
                        .iter()
                        .all(|i| !matches!(i.kind, ItemKind::Object(_)))
                })
        {
            return self.table_slices(n, &cells, &cw, &rh, x, y);
        }
        if self.depth == 0
            && !fixed_bottom
            && cells.len() == 1
            && cols == 1
            && total > self.ph - self.bottom - y
        {
            let cell = cells[0];
            let list = cell.child("subList").context("Cell paragraphs")?;
            let m = if cell.a("hasMargin") == "1" {
                margin(cell, "cellMargin")
            } else {
                margin(n, "inMargin")
            };
            let flow = flowing_lines(list);
            let mut at = 0;
            let mut top = y;
            while at < flow.len() {
                let start = flow[at].2;
                let capacity = self.ph - self.bottom - top - m[2] - m[3];
                let mut end = at;
                while end < flow.len() {
                    let (p, i, yy) = flow[end];
                    if end > at && yy - start + lines(p)[i].n("vertsize") / 100. > capacity {
                        break;
                    }
                    end += 1;
                }
                let (p, i, yy) = flow[end - 1];
                let height = yy - start + lines(p)[i].n("vertsize") / 100. + m[2] + m[3];
                self.border(cell.a("borderFillIDRef"), x, top, w, height);
                self.depth += 1;
                for (p, i, yy) in &flow[at..end] {
                    self.line(
                        p,
                        *i,
                        x + m[0],
                        top + m[2] + yy - start - lines(p)[*i].n("vertpos") / 100.,
                        w - m[0] - m[1],
                    )?;
                }
                self.depth -= 1;
                at = end;
                if at < flow.len() {
                    self.finish()?;
                    let section = &self.doc.sections[self.active_section];
                    self.decorations(section, false, 1, 0.)?;
                    top = self.top;
                }
            }
            self.report
                .warnings
                .insert("Long single-cell table split at line boundaries".into());
            return Ok(());
        }
        // Cached table heights include minimum/automatic row growth; keep them
        // instead of stretching cells to a stale object rectangle.
        let header_cells: Vec<_> = cells
            .iter()
            .filter(|c| c.child("cellAddr").is_some_and(|a| a.u("rowAddr") == 0))
            .map(|c| (*c).clone())
            .collect();
        let mut page_row = 0;
        let mut page_y = y;
        let mut previous_row = None;
        let captured_header = if self.capture_table_headers
            && n.a("repeatHeader") == "1"
            && rows > 1
            && rh[0] < (self.ph - self.top - self.bottom) / 3.
            && header_cells.iter().all(|c| {
                c.child("cellSpan").is_some_and(|s| s.u("rowSpan") == 1)
                    && c.find("footNote").is_none()
            }) {
            let index = self.table_headers.len();
            self.table_headers.push(TableHeaderPaint {
                y,
                height: rh[0],
                end: y + rh.iter().sum::<f64>(),
                ops: Vec::new(),
            });
            Some(index)
        } else {
            None
        };
        for c in &cells {
            let a = c.child("cellAddr").unwrap();
            let s = c.child("cellSpan").unwrap();
            let col = a.u("colAddr");
            let row = a.u("rowAddr");
            let mut band_end = if row == 0 && n.a("repeatHeader") == "1" && rows > 1 {
                2
            } else {
                row + 1
            };
            loop {
                let next = cells
                    .iter()
                    .filter(|c| {
                        let r = c.child("cellAddr").unwrap().u("rowAddr");
                        r >= row && r < band_end
                    })
                    .map(|c| {
                        c.child("cellAddr").unwrap().u("rowAddr")
                            + c.child("cellSpan").unwrap().u("rowSpan")
                    })
                    .max()
                    .unwrap_or(band_end);
                if next == band_end {
                    break;
                }
                band_end = next;
            }
            let mut incoming_notes = Vec::new();
            for c in cells.iter().filter(|c| {
                let r = c.child("cellAddr").unwrap().u("rowAddr");
                r >= row && r < band_end
            }) {
                c.all("footNote", &mut incoming_notes);
            }
            let reserve = self.footnote_reservation(&incoming_notes, x);
            let band_height = rh[row..band_end].iter().sum::<f64>();
            if self.depth == 0
                && !fixed_bottom
                && (n.a("pageBreak") == "CELL"
                    || (n.a("pageBreak") == "TABLE" && (total > h + 0.5 || reserve > 0.))
                    || total > self.ph - self.top - self.bottom)
                && previous_row != Some(row)
                && (row > page_row || page_y > self.top + 0.1)
                && band_height < self.ph - self.top - self.bottom
                && page_y + rh[page_row..band_end].iter().sum::<f64>()
                    > self.ph - self.bottom - reserve
            {
                let merged_crossing = cells.iter().any(|cell| {
                    let a = cell.child("cellAddr").unwrap();
                    let s = cell.child("cellSpan").unwrap();
                    a.u("rowAddr") < row && a.u("rowAddr") + s.u("rowSpan") > row
                });
                if !merged_crossing {
                    self.finish()?;
                    let section = &self.doc.sections[self.active_section];
                    let columns = section_columns(section);
                    self.decorations(
                        section,
                        false,
                        columns.map(|n| n.u("colCount")).unwrap_or(1),
                        val(columns, "sameGap"),
                    )?;
                    page_y = self.top;
                    page_row = row;
                    if n.a("repeatHeader") == "1"
                        && header_cells
                            .iter()
                            .all(|c| c.child("cellSpan").unwrap().u("rowSpan") == 1)
                    {
                        let mut header = n.clone();
                        header.attrs.insert("rowCnt".into(), "1".into());
                        header.attrs.insert("pageBreak".into(), "NONE".into());
                        header.attrs.insert("repeatHeader".into(), "0".into());
                        header.children.retain(|c| c.name != "tr");
                        header.children.push(Node {
                            name: "tr".into(),
                            children: header_cells.clone(),
                            ..Default::default()
                        });
                        self.table(&header, x, page_y, w, rh[0])?;
                        page_y += rh[0];
                    }
                    self.report
                        .warnings
                        .insert("Long table paginated by row with repeated header".into());
                }
            }
            previous_row = Some(row);
            let xx = x + cw[..col].iter().sum::<f64>();
            let yy = page_y + rh[page_row..row].iter().sum::<f64>();
            let ww = cw[col..col + s.u("colSpan")].iter().sum::<f64>();
            let hh = rh[row..row + s.u("rowSpan")].iter().sum::<f64>();
            let cell_paint_start = self.current.ops.len();
            self.border(c.a("borderFillIDRef"), xx, yy, ww, hh);
            if let Some(list) = c.child("subList") {
                let m = if c.a("hasMargin") == "1" {
                    margin(c, "cellMargin")
                } else {
                    margin(n, "inMargin")
                };
                let (min, mut max) = floating_extent(list);
                max = max.max(self.measured_height(list)?);
                let spare = (hh - m[2] - m[3] - (max - min)).max(0.);
                let shift = match list.a("vertAlign") {
                    "CENTER" => spare / 2.,
                    "BOTTOM" => spare,
                    _ => 0.,
                };
                self.local(list, xx + m[0], yy + m[2] + shift - min, ww - m[0] - m[1])?;
            }
            if row == 0
                && let Some(index) = captured_header
            {
                self.table_headers[index]
                    .ops
                    .extend_from_slice(&self.current.ops[cell_paint_start..]);
            }
        }
        let _ = (w, h);
        if self.depth == 0 {
            self.last_table_end = page_y + rh[page_row..].iter().sum::<f64>();
        }
        Ok(())
    }
    fn finish(&mut self) -> Result<()> {
        let body = self.ph - self.top - self.bottom;
        let has_long_group = self.footnotes.iter().any(|(_, x, _)| {
            self.footnotes
                .iter()
                .filter(|(_, xx, _)| (xx - x).abs() < 0.1)
                .map(|(n, _, _)| {
                    n.child("subList").map(content_height).unwrap_or(0.)
                        + self.note_gap(false, "betweenNotes", 4.)
                })
                .sum::<f64>()
                - self.note_gap(false, "betweenNotes", 4.)
                + self.note_gap(false, "aboveLine", 6.)
                + self.note_gap(false, "belowLine", 4.)
                + 1.
                > body - 48.
        });
        if has_long_group || !self.continued_notes.is_empty() {
            return self.finish_long_notes();
        }
        let notes = std::mem::take(&mut self.footnotes);
        if !notes.is_empty() {
            let mut groups: BTreeMap<i64, Vec<(Node, f64, f64)>> = BTreeMap::new();
            for n in notes {
                groups
                    .entry((n.1 * 100.).round() as i64)
                    .or_default()
                    .push(n);
            }
            for notes in groups.into_values() {
                let gap = self.note_gap(false, "betweenNotes", 4.);
                let height = notes
                    .iter()
                    .filter_map(|(n, _, _)| n.child("subList"))
                    .map(content_height)
                    .sum::<f64>()
                    + gap * notes.len().saturating_sub(1) as f64;
                let mut y = self.ph - self.bottom - height;
                self.note_separator(
                    false,
                    notes[0].1,
                    y - self.note_gap(false, "belowLine", 4.),
                    notes[0].2,
                );
                self.note_rendering = true;
                for (n, x, w) in notes {
                    if let Some(list) = n.child("subList") {
                        self.report.bounds.push(Bounds {
                            page: self.pages.len() + 1,
                            kind: "footnote".into(),
                            id: n.a("number").into(),
                            x,
                            y,
                            w,
                            h: content_height(list),
                        });
                        self.local(list, x, y, w)?;
                        y += content_height(list) + gap;
                    }
                }
                self.note_rendering = false;
            }
        }

        self.current.width = self.pw;
        self.current.height = self.ph;
        self.pages.push(std::mem::take(&mut self.current));
        Ok(())
    }
    /// Drain long notes onto dedicated continuation pages before resuming body flow.
    /// Lines are indivisible; numbering is already present in their first line.
    fn finish_long_notes(&mut self) -> Result<()> {
        let mut groups = std::mem::take(&mut self.continued_notes);
        for (note, x, w) in std::mem::take(&mut self.footnotes) {
            if let Some(list) = note.child("subList") {
                let flow = flowing_lines(list);
                let entry = groups.entry((x * 100.).round() as i64).or_insert((
                    x,
                    w,
                    std::collections::VecDeque::new(),
                ));
                for (idx, (p, line, y)) in flow.iter().enumerate() {
                    let height = flow
                        .get(idx + 1)
                        .map(|next| next.2 - y)
                        .unwrap_or_else(|| {
                            (lines(p)[*line].n("vertsize") + lines(p)[*line].n("spacing")) / 100.
                        })
                        .max(lines(p)[*line].n("vertsize") / 100.)
                        .max(0.1);
                    entry
                        .2
                        .push_back(((*p).clone(), *line, height, note.a("number").into()));
                }
                if let Some(last) = entry.2.back_mut() {
                    last.2 += self.note_gap(false, "betweenNotes", 4.);
                }
            }
        }
        let mut first = !self.draining_notes;
        loop {
            let overhead =
                self.note_gap(false, "aboveLine", 6.) + self.note_gap(false, "belowLine", 4.) + 1.;
            let capacity = (if first {
                let text_bottom = self
                    .current
                    .ops
                    .iter()
                    .filter_map(|op| match op {
                        Op::Text { y, size, .. }
                            if *y >= self.top && *y < self.ph - self.bottom =>
                        {
                            Some(y + size * 0.35)
                        }
                        _ => None,
                    })
                    .fold(self.top, f64::max);
                let object_bottom = self
                    .report
                    .bounds
                    .iter()
                    .filter(|b| b.page == self.pages.len() + 1 && b.kind != "watermark")
                    .map(|b| b.y + b.h)
                    .fold(self.top, f64::max);
                // Body layout already reserved half a page. At finalization the
                // unoccupied space above it can safely hold more complete lines.
                (self.ph - self.bottom - text_bottom.max(object_bottom))
                    .max((self.ph - self.top - self.bottom) * 0.5)
            } else {
                self.ph - self.top - self.bottom
            }) - overhead;
            anyhow::ensure!(capacity > 0., "No printable area for footnote continuation");
            self.note_rendering = true;
            for (x, w, rows) in groups.values_mut() {
                let mut batch = Vec::new();
                let mut height = 0.;
                while let Some(row) = rows.front() {
                    if height + row.2 > capacity + 0.01 {
                        break;
                    }
                    let row = rows.pop_front().unwrap();
                    height += row.2;
                    batch.push(row);
                }
                if batch.is_empty() && !rows.is_empty() {
                    anyhow::ensure!(first, "A single footnote line exceeds a printable page");
                    continue;
                }
                if batch.is_empty() {
                    continue;
                }
                let mut y = self.ph - self.bottom - height;
                self.note_separator(false, *x, y - self.note_gap(false, "belowLine", 4.), *w);
                for (p, line, h, id) in batch {
                    self.report.bounds.push(Bounds {
                        page: self.pages.len() + 1,
                        kind: "footnote-fragment".into(),
                        id,
                        x: *x,
                        y,
                        w: *w,
                        h,
                    });
                    self.depth += 1;
                    self.line(&p, line, *x, y - lines(&p)[line].n("vertpos") / 100., *w)?;
                    self.depth -= 1;
                    y += h;
                }
            }
            self.note_rendering = false;
            self.current.width = self.pw;
            self.current.height = self.ph;
            self.pages.push(std::mem::take(&mut self.current));
            groups.retain(|_, (_, _, rows)| !rows.is_empty());
            if !self.draining_notes {
                self.continued_notes = groups;
                break;
            }
            if groups.values().all(|(_, _, r)| r.is_empty()) {
                break;
            }
            let section = &self.doc.sections[self.active_section];
            self.decorations(section, false, self.active_columns, self.active_gap)?;
            first = false;
        }
        self.report.warnings.insert("Long footnotes share following body pages with a bounded note area; remaining section-end notes drain before the next section".into());
        Ok(())
    }
    fn drain_note_tail(&mut self) -> Result<()> {
        if self.continued_notes.is_empty() {
            return Ok(());
        }
        let section = &self.doc.sections[self.active_section];
        self.decorations(section, false, self.active_columns, self.active_gap)?;
        self.draining_notes = true;
        let result = self.finish_long_notes();
        self.draining_notes = false;
        result
    }
    fn decorations(&mut self, section: &Node, first: bool, cols: usize, gap: f64) -> Result<()> {
        let hiding = if first {
            section.find("pageHiding")
        } else {
            None
        };
        if let Some(number) = section
            .find("pageNumPos")
            .filter(|_| hiding.is_none_or(|h| h.a("hidePageNum") != "1"))
        {
            let pos = number.u("position");
            if pos > 0 {
                let page = (self.pages.len() as i64 + self.page_base).max(1) as usize;
                let font = self.fonts.get("times.ttf")?;
                let label = format!("{}{}{}", number.a("prefix"), page, number.a("suffix"));
                let w = self.fonts.width(font, &label, 10.);
                let right = match pos {
                    3 | 6 => true,
                    7 | 8 => page % 2 == 1,
                    9 | 10 => page.is_multiple_of(2),
                    _ => false,
                };
                let x = if pos == 2 || pos == 5 {
                    (self.pw - w) / 2.
                } else if right {
                    self.pw - self.right - w
                } else {
                    self.left
                };
                let y = if matches!(pos, 1 | 2 | 3 | 7 | 9) {
                    self.top - self.header.max(15.) / 2.
                } else {
                    self.ph - self.bottom + self.footer.max(15.) / 2. + 7.
                };
                self.current.ops.push(Op::Text {
                    x,
                    y,
                    text: label,
                    font,
                    size: 10.,
                    scale: 1.,
                    color: [0.; 3],
                });
            }
        }
        let sec = section.find("secPr").unwrap();
        if let Some(b) = sec
            .children_named("pageBorderFill")
            .find(|n| n.a("type") == "BOTH")
        {
            let m = margin(b, "offset");
            let paper = b.a("textBorder") == "PAPER";
            self.border(
                b.a("borderFillIDRef"),
                if paper { m[0] } else { self.left - m[0] },
                if paper { m[2] } else { self.top - m[2] },
                if paper {
                    self.pw - m[0] - m[1]
                } else {
                    self.pw - self.left - self.right + m[0] + m[1]
                },
                if paper {
                    self.ph - m[2] - m[3]
                } else {
                    self.ph - self.top - self.bottom + m[2] + m[3]
                },
            );
        }
        if cols > 1 && sec.a("binaryHwp") != "1" {
            let x = self.left + (self.pw - self.left - self.right) / 2.;
            let first_body = section
                .children_named("p")
                .skip(1)
                .flat_map(lines)
                .next()
                .map(|l| l.n("vertpos") / 100.)
                .unwrap_or(0.);
            self.current.ops.push(Op::Line {
                x,
                y: if first {
                    self.top + first_body
                } else {
                    self.top
                },
                x2: x,
                y2: self.ph - self.bottom,
                width: 0.34,
                color: [0.; 3],
            });
        }
        for master in &self.doc.masters {
            if hiding.is_some_and(|h| h.a("hideMasterPage") == "1") {
                continue;
            }
            if first && sec.a("hideFirstMasterPage") == "1" {
                continue;
            }
            let odd = (self.pages.len() + 1) % 2 == 1;
            if (master.a("type") == "ODD" && !odd) || (master.a("type") == "EVEN" && odd) {
                continue;
            }
            if let Some(list) = master.find("subList") {
                self.local(
                    list,
                    self.left,
                    if master.a("binaryHwp") == "1" {
                        0.
                    } else {
                        self.top
                    },
                    self.pw - self.left - self.right,
                )?;
            }
        }
        // Only section-level header/footer controls, never table/note descendants.
        for p in section.children_named("p") {
            for run in p.children_named("run") {
                for ctrl in run.children_named("ctrl") {
                    for c in &ctrl.children {
                        if let Some(list) = c.child("subList") {
                            let parity = match c.a("applyPageType") {
                                "ODD" => (self.pages.len() + 1) % 2 == 1,
                                "EVEN" => (self.pages.len() + 1).is_multiple_of(2),
                                _ => true,
                            };
                            if c.name == "header"
                                && parity
                                && hiding.is_none_or(|h| h.a("hideHeader") != "1")
                                && (!first
                                    || (sec.a("binaryHwp") == "1"
                                        && sec.a("hideFirstHeader") != "1"))
                            {
                                self.local(
                                    list,
                                    self.left,
                                    self.top - self.header,
                                    self.pw - self.left - self.right,
                                )?;
                            }
                            if c.name == "footer"
                                && parity
                                && hiding.is_none_or(|h| h.a("hideFooter") != "1")
                                && !(first && sec.a("hideFirstFooter") == "1")
                            {
                                self.local(
                                    list,
                                    self.left,
                                    self.ph - self.bottom + self.footer - content_height(list),
                                    self.pw - self.left - self.right,
                                )?;
                            }
                        }
                    }
                }
            }
        }
        let _ = gap;
        Ok(())
    }
    fn reflow_advance(
        &mut self,
        section: &Node,
        col: &mut usize,
        y: &mut f64,
        cols: usize,
        gap: f64,
    ) -> Result<()> {
        *col += 1;
        if *col >= cols {
            self.finish()?;
            *col = 0;
            self.decorations(section, false, cols, gap)?;
        }
        *y = self.top;
        Ok(())
    }
    fn reflow_list(&mut self, list: &mut Node, width: f64, nesting: usize) -> Result<f64> {
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
    fn reflow_table_contents(&mut self, n: &mut Node, width: f64, nesting: usize) -> Result<()> {
        let rows = n.u("rowCnt");
        let cols = n.u("colCnt");
        anyhow::ensure!(
            rows > 0 && cols > 0 && rows <= 1000 && cols <= 1000,
            "Invalid reflow table dimensions"
        );
        let mut cw = vec![0f64; cols];
        for cell in n.children_named("tr").flat_map(|r| r.children_named("tc")) {
            let a = cell.child("cellAddr").context("cellAddr")?;
            let span = cell.child("cellSpan").context("cellSpan")?;
            anyhow::ensure!(
                a.u("colAddr") + span.u("colSpan") <= cols
                    && a.u("rowAddr") + span.u("rowSpan") <= rows
                    && span.u("colSpan") > 0
                    && span.u("rowSpan") > 0,
                "Cell outside table"
            );
            if span.u("colSpan") == 1 {
                cw[a.u("colAddr")] = cw[a.u("colAddr")].max(val(cell.child("cellSz"), "width"));
            }
        }
        for cell in n.children_named("tr").flat_map(|r| r.children_named("tc")) {
            let col = cell.child("cellAddr").unwrap().u("colAddr");
            let count = cell.child("cellSpan").unwrap().u("colSpan");
            let missing = (col..col + count)
                .filter(|i| cw[*i] == 0.)
                .collect::<Vec<_>>();
            if !missing.is_empty() {
                let each = (val(cell.child("cellSz"), "width")
                    - cw[col..col + count].iter().sum::<f64>())
                .max(0.)
                    / missing.len() as f64;
                for i in missing {
                    cw[i] = each;
                }
            }
        }
        for c in &mut cw {
            if *c <= 0. {
                *c = width / cols as f64;
            }
        }
        let ratio = (width / cw.iter().sum::<f64>()).min(1.);
        for c in &mut cw {
            *c *= ratio;
        }
        let m_default = margin(n, "inMargin");
        let mut rh = vec![0f64; rows];
        let mut needs = Vec::new();
        for row in n.children.iter_mut().filter(|r| r.name == "tr") {
            for cell in row.children.iter_mut().filter(|c| c.name == "tc") {
                let a = cell.child("cellAddr").unwrap();
                let col = a.u("colAddr");
                let row = a.u("rowAddr");
                let span = cell.child("cellSpan").unwrap();
                let cs = span.u("colSpan");
                let rs = span.u("rowSpan");
                let w = cw[col..col + cs].iter().sum::<f64>();
                let m = if cell.a("hasMargin") == "1" {
                    margin(cell, "cellMargin")
                } else {
                    m_default
                };
                let minimum = val(cell.child("cellSz"), "height");
                let height =
                    if let Some(list) = cell.children.iter_mut().find(|n| n.name == "subList") {
                        self.reflow_list(list, w - m[0] - m[1], nesting + 1)? + m[2] + m[3]
                    } else {
                        0.
                    };
                let h = minimum.max(height);
                if rs == 1 {
                    rh[row] = rh[row].max(h);
                }
                needs.push((row, rs, h));
                node_mut(cell, "cellSz")
                    .attrs
                    .insert("width".into(), (w * 100.).to_string());
            }
        }
        for (row, span, h) in needs {
            let extra = (h - rh[row..row + span].iter().sum::<f64>()).max(0.);
            rh[row + span - 1] += extra;
        }
        for row in n.children.iter_mut().filter(|r| r.name == "tr") {
            for cell in row.children.iter_mut().filter(|c| c.name == "tc") {
                let r = cell.child("cellAddr").unwrap().u("rowAddr");
                let span = cell.child("cellSpan").unwrap().u("rowSpan");
                node_mut(cell, "cellSz").attrs.insert(
                    "height".into(),
                    (rh[r..r + span].iter().sum::<f64>() * 100.).to_string(),
                );
            }
        }
        let sz = node_mut(n, "sz");
        sz.attrs
            .insert("width".into(), (cw.iter().sum::<f64>() * 100.).to_string());
        sz.attrs
            .insert("height".into(), (rh.iter().sum::<f64>() * 100.).to_string());
        Ok(())
    }
    fn reflow_children(&mut self, p: &mut Node, width: f64, nesting: usize) -> Result<()> {
        anyhow::ensure!(nesting < 64, "Excessive nested reflow");
        for run in p.children.iter_mut().filter(|r| r.name == "run") {
            for n in &mut run.children {
                match n.name.as_str() {
                    "tbl" | "pic" | "rect" | "ellipse" | "polygon" => {
                        anyhow::ensure!(
                            n.a("textWrap") != "BEHIND_TEXT",
                            "Reflow of page-background objects is not supported"
                        );
                        let block = n.name == "tbl"
                            || n.child("pos").is_none_or(|p| p.a("treatAsChar") != "1");
                        if n.name == "tbl" {
                            self.reflow_table_contents(n, width, nesting)?;
                        } else {
                            let old_w = val(n.child("sz"), "width");
                            let ratio = (width / old_w.max(0.1)).min(1.);
                            for key in ["sz", "curSz"] {
                                if let Some(sz) = n.children.iter_mut().find(|s| s.name == key) {
                                    for dimension in ["width", "height"] {
                                        let value = sz.n(dimension) * ratio;
                                        sz.attrs.insert(dimension.into(), value.to_string());
                                    }
                                }
                            }
                            if n.name == "rect" {
                                let angle =
                                    n.child("rotationInfo").map(|r| r.n("angle")).unwrap_or(0.);
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
                        let w = val(n.child("sz"), "width");
                        for caption in n.children.iter_mut().filter(|c| c.name == "caption") {
                            anyhow::ensure!(
                                matches!(caption.a("side"), "TOP" | "BOTTOM")
                                    && caption.find("footNote").is_none(),
                                "Reflow supports plain top/bottom captions only"
                            );
                            if let Some(list) =
                                caption.children.iter_mut().find(|n| n.name == "subList")
                            {
                                self.reflow_list(list, w, nesting + 1)?;
                            }
                        }
                    }
                    "ctrl" => {
                        for control in &mut n.children {
                            if control.name == "footNote" {
                                let (_, w) = self.footnote_area(self.left);
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
    fn general_objects(items: &[Item]) -> bool {
        let objects: Vec<_> = items
            .iter()
            .filter_map(|i| match &i.kind {
                ItemKind::Object(n) => Some((i.pos, n)),
                _ => None,
            })
            .collect();
        objects.iter().all(|(_, n)| {
            matches!(n.name.as_str(), "pic" | "rect" | "ellipse" | "polygon")
                && n.a("textWrap") == "SQUARE"
                && n.child("caption").is_none()
                && n.child("pos").is_some_and(|p| {
                    p.a("treatAsChar") != "1"
                        && matches!(p.a("horzRelTo"), "PARA" | "COLUMN")
                        && p.a("vertRelTo") == "PARA"
                        && matches!(p.a("vertAlign"), "" | "TOP")
                })
        })
    }
    fn position_region(
        &mut self,
        n: &mut Node,
        width: f64,
        existing: &[crate::wrap_region::Region],
    ) -> Result<crate::wrap_region::Region> {
        let margin = margin(n, "outMargin").map(|v| v.max(0.));
        let angle = n.child("rotationInfo").map(|a| a.n("angle")).unwrap_or(0.);
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
        let pos = n.child("pos").unwrap();
        let mut x = pos.signed("horzOffset") / 100.
            + match pos.a("horzAlign") {
                "CENTER" => (width - w) / 2.,
                "RIGHT" => width - w - margin[1],
                _ => margin[0],
            };
        let mut y = pos.signed("vertOffset") / 100. + margin[2];
        anyhow::ensure!(
            y >= 0.,
            "Negative vertical float anchors require saved-layout conversion"
        );
        anyhow::ensure!(w <= width + 0.01, "Rotated float is wider than its column");
        let bounded = x.clamp(0., (width - w).max(0.));
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
        } else if self.contour_wrap && n.name == "polygon" {
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
            let points = n
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
            anyhow::ensure!(
                points.len() >= 3 && points.len() <= 4096,
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
            points,
            margin: margin.map(|m| m + clearance),
            flow: n.a("textFlow").into(),
        };
        if pos.a("allowOverlap") != "1" {
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
    fn reflow_paragraph(
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
                general.push(region);
            }
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
        let items = Self::items(&p);
        let mut units = Vec::new();
        let mut glyphs = Vec::new();
        for item in &items {
            let (unit, glyph) = match &item.kind {
                ItemKind::Char(c) if *c == '\n' || *c == '\r' => {
                    let g = self.glyph(' ', &item.style)?;
                    (
                        crate::reflow::Unit {
                            pos: item.pos,
                            ch: Some('\n'),
                            ascent: g.ascent,
                            descent: g.descent,
                            em: g.size,
                            block: false,
                            ..Default::default()
                        },
                        None,
                    )
                }
                ItemKind::Char(c) => {
                    let g = self.glyph(*c, &item.style)?;
                    (
                        crate::reflow::Unit {
                            pos: item.pos,
                            ch: Some(*c),
                            width: g.width.max(0.),
                            ascent: g.ascent,
                            descent: g.descent,
                            em: g.size,
                            block: false,
                        },
                        Some(g),
                    )
                }
                ItemKind::Tab(w) => (
                    crate::reflow::Unit {
                        pos: item.pos,
                        width: *w,
                        ch: Some(' '),
                        ..Default::default()
                    },
                    None,
                ),
                ItemKind::Object(n) if n.a("reflowFloat") == "1" => (
                    crate::reflow::Unit {
                        pos: item.pos,
                        ..Default::default()
                    },
                    None,
                ),
                ItemKind::Object(n)
                    if matches!(
                        n.name.as_str(),
                        "tbl" | "pic" | "rect" | "ellipse" | "polygon"
                    ) =>
                {
                    let block =
                        n.name == "tbl" || n.child("pos").is_none_or(|p| p.a("treatAsChar") != "1");
                    let m = margin(n, "outMargin");
                    let cap = caption_extent(n);
                    let h = effective_height(n) + m[2] + m[3] + cap[2] + cap[3];
                    (
                        crate::reflow::Unit {
                            pos: item.pos,
                            ch: Some(' '),
                            width: if block {
                                0.
                            } else {
                                val(n.child("sz"), "width") + m[0] + m[1]
                            },
                            ascent: if block { h } else { h * 0.85 },
                            descent: if block { 0. } else { h * 0.15 },
                            block,
                            ..Default::default()
                        },
                        None,
                    )
                }
                ItemKind::Control(n) if n.name == "footNote" => {
                    let g = self.glyph('1', &item.style)?;
                    let w = self
                        .fonts
                        .width(g.font, &self.note_label(n, false), g.size * 0.6)
                        * g.scale;
                    (
                        crate::reflow::Unit {
                            pos: item.pos,
                            width: w,
                            ascent: g.size,
                            em: g.size,
                            ..Default::default()
                        },
                        None,
                    )
                }
                ItemKind::Control(n) if n.name == "autoNum" => {
                    let g = self.glyph('1', &item.style)?;
                    let w = self.fonts.width(g.font, &self.auto_label(n), g.size) * g.scale;
                    (
                        crate::reflow::Unit {
                            pos: item.pos,
                            width: w,
                            ascent: g.ascent,
                            descent: g.descent,
                            em: g.size,
                            ..Default::default()
                        },
                        None,
                    )
                }
                ItemKind::Control(c)
                    if matches!(
                        c.name.as_str(),
                        "colPr"
                            | "pageNumPos"
                            | "pageHiding"
                            | "newNumber"
                            | "header"
                            | "footer"
                            | "cachedField"
                            | "bookmark"
                    ) =>
                {
                    (
                        crate::reflow::Unit {
                            pos: item.pos,
                            ..Default::default()
                        },
                        None,
                    )
                }
                _ => bail!(
                    "Plain-text reflow cannot lay out objects/notes/control in paragraph {}",
                    p.a("id")
                ),
            };
            units.push(unit);
            glyphs.push(glyph);
        }
        for i in 0..units.len().saturating_sub(1) {
            if let (Some(a), Some(b)) = (&glyphs[i], &glyphs[i + 1]) {
                units[i].width = units[i].width.max(a.ink_right - b.ink_left + 0.02);
            }
        }
        for i in 1..items.len() {
            if matches!(&items[i].kind,ItemKind::Control(n) if n.name=="footNote") {
                units[i - 1].ch = None;
            }
        }
        let mut grouped: Vec<crate::reflow::Unit> = Vec::new();
        for unit in units {
            if let Some(last) = grouped.last_mut().filter(|last| last.pos == unit.pos) {
                last.width += unit.width;
                last.ascent = last.ascent.max(unit.ascent);
                last.descent = last.descent.max(unit.descent);
                last.em = last.em.max(unit.em);
                last.ch = None;
            } else {
                grouped.push(unit);
            }
        }
        // The label is not part of the UTF-16 text stream, but its ink must fit
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
    fn general_plans(
        &mut self,
        section: &Node,
        width: f64,
        cols: usize,
        gap: f64,
    ) -> Result<Vec<(Node, Vec<crate::reflow::Line>, crate::reflow::Settings)>> {
        let mut plans: Vec<(Node, Vec<crate::reflow::Line>, crate::reflow::Settings)> = Vec::new();
        let mut carry: Vec<crate::wrap_region::Region> = Vec::new();
        let mut group: Option<(usize, f64)> = None;
        for source in section.children_named("p") {
            if let Some(c) = paragraph_columns(source) {
                anyhow::ensure!(
                    c.u("colCount").max(1) == cols && (val(Some(c), "sameGap") - gap).abs() < 0.01,
                    "Reflow with mid-section column changes is not supported"
                );
            }
            let style = self.doc.paras.get(source.a("paraPrIDRef"));
            let settings = crate::reflow::settings(style);
            let source_items = Self::items(source);
            let supported = Self::general_objects(&source_items);
            let follow = source.a("pageBreak") != "1"
                && !settings.page
                && !settings.keep
                && !settings.next
                && style
                    .and_then(|s| s.child("heading"))
                    .is_none_or(|h| !matches!(h.a("type"), "NUMBER" | "BULLET"))
                && supported;
            if !follow && !carry.is_empty() {
                if let Some((_, _, s)) = plans.last_mut() {
                    s.after += carry.iter().map(|r| r.bounds()[3]).fold(0f64, f64::max);
                }
                carry.clear();
                group = None;
            }
            for r in &mut carry {
                r.shift_y(-settings.before);
            }
            carry.retain(|r| r.bounds()[3] > 0.);
            if carry.is_empty() {
                group = None;
            }
            let mut source = source.clone();
            source.attrs.insert("floatFlow".into(), "1".into());
            if !carry.is_empty() {
                source
                    .attrs
                    .insert("generalCarry".into(), serde_json::to_string(&carry)?);
            }
            let (mut p, lines, s) = self.reflow_paragraph(&source, width, 0)?;
            if !p.a("generalRegions").is_empty() {
                carry = serde_json::from_str(p.a("generalRegions"))?;
            }
            let remaining = carry.iter().map(|r| r.bounds()[3]).fold(0f64, f64::max);
            if remaining > 0. {
                if let Some((origin, elapsed)) = group {
                    let height = (elapsed + s.before + remaining)
                        .max(plans[origin].0.n("reflowFloatHeight"));
                    plans[origin]
                        .0
                        .attrs
                        .insert("reflowFloatHeight".into(), height.to_string());
                    p.attrs.remove("reflowFloatHeight");
                    p.attrs
                        .insert("reflowFloatCarry".into(), remaining.to_string());
                    group = Some((origin, elapsed + s.before));
                } else {
                    group = Some((plans.len(), 0.));
                }
            }
            let consumed = lines.iter().map(|l| l.height).sum::<f64>() + s.after;
            for r in &mut carry {
                r.shift_y(-consumed);
            }
            carry.retain(|r| r.bounds()[3] > 0.);
            if let Some((origin, elapsed)) = group {
                group = (!carry.is_empty()).then_some((origin, elapsed + consumed));
            }
            plans.push((p, lines, s));
        }
        if let Some((_, _, s)) = plans.last_mut() {
            s.after += carry.iter().map(|r| r.bounds()[3]).fold(0f64, f64::max);
        }
        Ok(plans)
    }
    fn reflow_text_section(
        &mut self,
        section: &Node,
        width: f64,
        cols: usize,
        gap: f64,
    ) -> Result<(f64, usize, f64, f64)> {
        self.general_wrap = self.contour_wrap
            || section.children_named("p").any(|p| {
                let items = Self::items(p);
                let objects: Vec<_> = items
                    .iter()
                    .filter_map(|i| match &i.kind {
                        ItemKind::Object(n)
                            if n.a("textWrap") == "SQUARE"
                                && n.child("pos").is_some_and(|p| p.a("treatAsChar") != "1") =>
                        {
                            Some(n)
                        }
                        _ => None,
                    })
                    .collect();
                let sides: BTreeSet<_> = objects
                    .iter()
                    .map(|n| n.child("pos").unwrap().a("horzAlign"))
                    .collect();
                objects.len() > 2
                    || sides.len() < objects.len()
                    || (!objects.is_empty()
                        && items
                            .iter()
                            .rposition(|i| matches!(i.kind, ItemKind::Object(_)))
                            .is_some_and(|last| {
                                items[..last]
                                    .iter()
                                    .any(|i| matches!(i.kind, ItemKind::Char(_)))
                            }))
                    || objects.iter().any(|n| {
                        matches!(n.name.as_str(), "ellipse" | "polygon")
                            || !matches!(n.a("textFlow"), "" | "BOTH_SIDES")
                            || n.child("pos").is_some_and(|p| {
                                p.a("horzAlign") == "CENTER"
                                    || p.signed("horzOffset") != 0.
                                    || p.signed("vertOffset") != 0.
                            })
                    })
            });
        let plans = if self.general_wrap {
            self.general_plans(section, width, cols, gap)?
        } else {
            let mut plans: Vec<(Node, Vec<crate::reflow::Line>, crate::reflow::Settings)> =
                Vec::new();
            let mut carry: Vec<(f64, f64, f64)> = Vec::new();
            for source in section.children_named("p") {
                if let Some(c) = paragraph_columns(source) {
                    anyhow::ensure!(
                        c.u("colCount").max(1) == cols
                            && (val(Some(c), "sameGap") - gap).abs() < 0.01,
                        "Reflow with mid-section column changes is not supported"
                    );
                }
                let style = self.doc.paras.get(source.a("paraPrIDRef"));
                let source_settings = crate::reflow::settings(style);
                let (cut_left, cut_right) = carry
                    .iter()
                    .filter(|(_, _, h)| *h > source_settings.before)
                    .fold((0f64, 0f64), |(l, r), (a, b, _)| (l.max(*a), r.max(*b)));
                let side_width = width
                    - (source_settings.left + source_settings.indent.max(0.)).max(cut_left)
                    - source_settings.right.max(cut_right);
                let can_follow = source.a("pageBreak") != "1"
                    && side_width > 24.
                    && !source_settings.page
                    && !source_settings.keep
                    && !source_settings.next
                    && style
                        .and_then(|s| s.child("heading"))
                        .is_none_or(|h| !matches!(h.a("type"), "NUMBER" | "BULLET"))
                    && Self::items(source)
                        .iter()
                        .all(|i| !matches!(i.kind, ItemKind::Object(_)));
                if !can_follow && !carry.is_empty() {
                    if let Some((_, _, last_settings)) = plans.last_mut() {
                        last_settings.after +=
                            carry.iter().map(|(_, _, h)| *h).fold(0f64, f64::max);
                    }
                    carry.clear();
                }
                let mut source = source.clone();
                source.attrs.insert("floatFlow".into(), "1".into());
                for (_, _, remaining) in &mut carry {
                    *remaining -= source_settings.before;
                }
                carry.retain(|(_, _, h)| *h > 0.);
                if !carry.is_empty() {
                    source
                        .attrs
                        .insert("floatCarryRegions".into(), serde_json::to_string(&carry)?);
                }
                let (p, lines, settings) = self.reflow_paragraph(&source, width, 0)?;
                if p.n("reflowFloatHeight") > 0. {
                    carry = serde_json::from_str(p.a("reflowFloatRegions"))?;
                }
                let consumed = lines.iter().map(|l| l.height).sum::<f64>() + settings.after;
                for (_, _, remaining) in &mut carry {
                    *remaining -= consumed;
                }
                carry.retain(|(_, _, h)| *h > 0.);
                plans.push((p, lines, settings));
            }
            if let Some((_, _, settings)) = plans.last_mut() {
                settings.after += carry.iter().map(|(_, _, h)| *h).fold(0f64, f64::max);
            }

            plans
        };
        let mut col = 0;
        let mut y = self.top;
        let body = self.ph - self.top - self.bottom;
        self.flow_floor = None;
        for (pi, (p, ls, s)) in plans.iter().enumerate() {
            let items = Self::items(p);
            let notes_for = |index: usize| {
                let end = ls.get(index + 1).map(|l| l.pos).unwrap_or(usize::MAX);
                items
                    .iter()
                    .filter(|i| i.pos >= ls[index].pos && i.pos < end)
                    .filter_map(|i| match &i.kind {
                        ItemKind::Control(n) if n.name == "footNote" => Some(n),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            };
            if (p.a("pageBreak") == "1" || s.page) && (y > self.top + 0.01 || col > 0) {
                self.finish()?;
                col = 0;
                y = self.top;
                self.decorations(section, false, cols, gap)?;
            }
            let height = s.before + ls.iter().map(|l| l.height).sum::<f64>() + s.after;
            let mut keep_height = height;
            if s.next {
                let mut j = pi + 1;
                while let Some((next, nl, ns)) = plans.get(j) {
                    if next.a("pageBreak") == "1" || ns.page {
                        break;
                    }
                    let all = nl.iter().map(|l| l.height).sum::<f64>();
                    keep_height += ns.before
                        + if ns.next || (ns.keep && keep_height + ns.before + all <= body) {
                            all + ns.after
                        } else {
                            nl.iter()
                                .take(if ns.widow { 2 } else { 1 })
                                .map(|l| l.height)
                                .sum::<f64>()
                        };
                    if !ns.next {
                        break;
                    }
                    j += 1;
                }
            }
            if (s.keep || s.next)
                && keep_height <= body
                && y + keep_height > self.ph - self.bottom
                && y > self.top + 0.01
            {
                self.reflow_advance(section, &mut col, &mut y, cols, gap)?;
            }
            if p.n("reflowFloatHeight") > 0. {
                let mut span = 0.;
                let mut owned_notes = Vec::new();
                'float_band: for (j, (part, part_lines, part_settings)) in
                    plans.iter().enumerate().skip(pi)
                {
                    if j > pi {
                        span = (span + part_settings.before).min(p.n("reflowFloatHeight"));
                        if span >= p.n("reflowFloatHeight") {
                            break;
                        }
                    }
                    let part_items = Self::items(part);
                    for (index, line) in part_lines.iter().enumerate() {
                        let end = part_lines
                            .get(index + 1)
                            .map(|l| l.pos)
                            .unwrap_or(usize::MAX);
                        for item in part_items
                            .iter()
                            .filter(|i| i.pos >= line.pos && i.pos < end)
                        {
                            if let ItemKind::Control(note) = &item.kind
                                && note.name == "footNote"
                            {
                                owned_notes.push(note.clone());
                            }
                        }
                        span += line.height;
                        if span >= p.n("reflowFloatHeight") {
                            break 'float_band;
                        }
                    }
                    span = (span + part_settings.after).min(p.n("reflowFloatHeight"));
                    if span >= p.n("reflowFloatHeight") {
                        break;
                    }
                }
                let notes: Vec<_> = owned_notes.iter().collect();
                let reserve =
                    self.footnote_reservation(&notes, self.left + col as f64 * (width + gap));
                if y + s.before + span > self.ph - self.bottom - reserve && y > self.top + 0.01 {
                    self.reflow_advance(section, &mut col, &mut y, cols, gap)?;
                }
                let reserve =
                    self.footnote_reservation(&notes, self.left + col as f64 * (width + gap));
                anyhow::ensure!(
                    s.before + span + reserve <= body,
                    "Floating object and anchored text exceed column capacity"
                );
            }
            y += s.before;
            let mut at = 0;
            while at < ls.len() {
                if ls[at].block {
                    let end = ls.get(at + 1).map(|l| l.pos).unwrap_or(usize::MAX);
                    let object = items
                        .iter()
                        .filter(|i| i.pos >= ls[at].pos && i.pos < end)
                        .find_map(|i| {
                            if let ItemKind::Object(n) = &i.kind {
                                Some(n)
                            } else {
                                None
                            }
                        })
                        .context("Missing reflow block")?;

                    let ox = self.left + col as f64 * (width + gap);
                    let reserve = self.footnote_reservation(&[], ox);
                    if ls[at].height <= body
                        && y + ls[at].height > self.ph - self.bottom - reserve
                        && y > self.top + 0.01
                    {
                        self.reflow_advance(section, &mut col, &mut y, cols, gap)?;
                    }
                    let ox = self.left + col as f64 * (width + gap);
                    let cap = caption_extent(object);
                    let old_page = self.pages.len();
                    let bounds_start = self.report.bounds.len();
                    if cols > 1 && object.name == "tbl" {
                        let placement = self
                            .note_spec(false)
                            .and_then(|s| s.child("placement"))
                            .map(|p| p.a("place"))
                            .unwrap_or("EACH_COLUMN");
                        anyhow::ensure!(
                            placement == "EACH_COLUMN",
                            "Mixed-column reflow requires per-column footnote placement"
                        );
                        self.reflow_table_column = Some(col);
                    }
                    let rendered = self.object(object, ox, y + cap[2], y + cap[2], ox, width);
                    let final_column = self.reflow_table_column.take();
                    rendered?;
                    if object.name == "tbl" {
                        y = self.last_table_end
                            + if self.split_caption_handled {
                                0.
                            } else {
                                cap[3]
                            };
                    } else {
                        y = self.report.bounds[bounds_start..]
                            .iter()
                            .filter(|b| b.kind == object.name)
                            .map(|b| b.y + b.h)
                            .fold(y + ls[at].height - cap[3], f64::max)
                            + cap[3];
                    }
                    if let Some(final_column) = final_column {
                        col = final_column;
                    } else if self.pages.len() != old_page {
                        col = 0;
                    }
                    self.flow_floor = None;
                    at += 1;
                    continue;
                }
                let mut count = 0;
                let mut used = 0.;
                let mut incoming = Vec::new();
                while at + count < ls.len() && !ls[at + count].block {
                    incoming.extend(notes_for(at + count));
                    let reserve = self
                        .footnote_reservation(&incoming, self.left + col as f64 * (width + gap));
                    if y + used + ls[at + count].height > self.ph - self.bottom - reserve + 0.001 {
                        break;
                    }
                    used += ls[at + count].height;
                    count += 1;
                }
                if count == 0 {
                    anyhow::ensure!(
                        y > self.top + 0.01,
                        "A reflowed line is taller than the printable page"
                    );
                    self.reflow_advance(section, &mut col, &mut y, cols, gap)?;
                    continue;
                }
                if s.widow
                    && !(at == 0 && (p.n("reflowFloatHeight") > 0. || p.n("reflowFloatCarry") > 0.))
                    && at + count < ls.len()
                    && ls[at].height + ls.get(at + 1).map(|l| l.height).unwrap_or(0.) <= body
                {
                    if count == 1 && y > self.top + 0.01 {
                        self.reflow_advance(section, &mut col, &mut y, cols, gap)?;
                        continue;
                    }
                    if ls.len() - at - count == 1 && count > 1 {
                        count -= 1;
                    }
                }
                for (i, line) in ls.iter().enumerate().skip(at).take(count) {
                    self.line(
                        p,
                        i,
                        self.left + col as f64 * (width + gap),
                        y - lines(p)[i].n("vertpos") / 100.,
                        width,
                    )?;
                    self.report.bounds.push(Bounds {
                        page: self.pages.len() + 1,
                        kind: "reflow-line".into(),
                        id: p.a("id").into(),
                        x: self.left + col as f64 * (width + gap) + line.left,
                        y,
                        w: line.width,
                        h: line.height,
                    });
                    y += line.height;
                }
                at += count;
                if at < ls.len() && !ls[at].block {
                    self.reflow_advance(section, &mut col, &mut y, cols, gap)?;
                }
            }
            y += s.after;
        }
        self.report.warnings.insert("Reflow uses font metrics and recursively lays out table cells and footnotes; leading left/right square pictures and rectangles wrap across following ordinary paragraphs; other floating objects become blocks; advanced wrapping and exact Hancom line breaking remain limited".into());
        Ok((y, col, width, gap))
    }
    pub fn render(&mut self) -> Result<()> {
        self.render_mode(false)
    }
    pub fn render_reflow_text(&mut self) -> Result<()> {
        self.render_mode(true)
    }
    pub fn render_contours(&mut self) -> Result<()> {
        self.contour_wrap = true;
        self.report.warnings.insert("Opt-in geometric contour wrapping; largest free interval per line; images retain rectangular exclusions".into());
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
