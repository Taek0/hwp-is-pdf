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
fn content_height(list: &Node) -> f64 {
    list.children_named("p")
        .flat_map(lines)
        .map(|l| (l.n("vertpos") + l.n("vertsize")) / 100.)
        .fold(0., f64::max)
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
        declared.max(val(n.child("curSz"), "height")).max(text)
    } else {
        declared
    }
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
    ascent:f64,
    descent:f64,
}
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
    pub pages: usize,
    pub equations: usize,
    pub pictures: usize,
    pub tables: usize,
    pub warnings: BTreeSet<String>,
    pub bounds: Vec<Bounds>,
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
    footnotes:Vec<(Node,f64,f64)>,
    depth: usize,
    glyph_cache: BTreeMap<(char, String), Glyph>,
    active_section:usize,
    page_base:i64,
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
            footnotes:Vec::new(),
            depth: 0,
            glyph_cache: BTreeMap::new(),
            active_section:0,
            page_base:1,
        }
    }
    fn glyph(&mut self, c: char, style: &str) -> Result<Glyph> {
        let key = (c, style.to_string());
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
        let serif = name.contains("명조") || name.contains("바탕") || name.contains("Times");
        let file = if latin && serif {
            if bold { "timesbd.ttf" } else { "times.ttf" }
        } else if latin {
            if bold { "arialbd.ttf" } else { "arial.ttf" }
        } else if serif {
            "batang.ttc"
        } else if bold {
            "malgunbd.ttf"
        } else {
            "malgun.ttf"
        };
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
        let size =
            ch.n("height") / 100. * ch.child("relSz").map(|n| n.n(lang) / 100.).unwrap_or(1.);
        let scale = ch.child("ratio").map(|n| n.n(lang) / 100.).unwrap_or(1.);
        let spacing = ch
            .child("spacing")
            .map(|n| n.n(lang) / 100. * size)
            .unwrap_or(0.);
        let width = self.fonts.width(font, &c.to_string(), size) * scale + spacing;
        let face=ttf_parser::Face::parse(&self.fonts.list[font].bytes,0).unwrap();
        let bb=face.glyph_index(c).and_then(|id|face.glyph_bounding_box(id));
        let unit=size/face.units_per_em() as f64;
        let glyph = Glyph {
            font,
            size,
            scale,
            width,
            color: color(ch.a("textColor")),
            ascent:bb.map(|b|b.y_max as f64*unit).unwrap_or(0.),
            descent:bb.map(|b|(-b.y_min as f64*unit).max(0.)).unwrap_or(0.),
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
                    "tbl" | "pic" | "equation" | "rect" | "line" | "ellipse" | "polygon" | "curve" | "arc" => {
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
    fn line(&mut self, p: &Node, li: usize, ox: f64, oy: f64, width: f64) -> Result<()> {
        let segs = lines(p);
        let seg = segs[li];
        let start = seg.u("textpos");
        let end = segs
            .get(li + 1)
            .map(|n| n.u("textpos"))
            .unwrap_or(usize::MAX);
        let its: Vec<_> = Self::items(p)
            .into_iter()
            .filter(|i| i.pos >= start && i.pos < end)
            .collect();
        let line_y = oy + seg.n("vertpos") / 100.;
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
        let mut number_draw = None;
        if li == 0
            && let Some(heading) = style.and_then(|s| s.child("heading"))
            && heading.a("type") == "NUMBER"
        {
            let num = Self::items(p)
                .iter()
                .filter_map(|i| match &i.kind {
                    ItemKind::Control(c) if c.name == "endNote" => Some(c.u("number")),
                    _ => None,
                })
                .next();
            if let Some(num) = num {
                let mut defs = Vec::new();
                self.doc.header.all("numbering", &mut defs);
                let head = defs
                    .into_iter()
                    .find(|d| d.a("id") == heading.a("idRef"))
                    .and_then(|n| {
                        n.children_named("paraHead")
                            .find(|h| h.u("level") == heading.u("level") + 1)
                    });
                let style = head.map(|h| h.a("charPrIDRef")).unwrap_or("0");
                let g = self.glyph('1', style)?;
                let label = format!("{num}.");
                let nw = self.fonts.width(g.font, &label, g.size);
                let offset = head
                    .map(|h| {
                        if h.a("textOffsetType") == "PERCENT" {
                            g.size * h.n("textOffset") / 100.
                        } else {
                            h.n("textOffset") / 100.
                        }
                    })
                    .unwrap_or(g.size * 0.5);
                x += nw + offset;
                avail -= nw + offset;
                number_draw = Some((label, g));
            }
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
                ItemKind::Control(n) if n.name=="charOverlay"=>{let g=self.glyph('가',&i.style)?;let width=g.size;(Some(g),width)},
                ItemKind::Control(n) if n.name=="ruby"=>{let g=self.glyph(n.a("main").chars().next().unwrap_or(' '),&i.style)?;let width=self.fonts.width(g.font,n.a("main"),g.size);(Some(g),width)},
                ItemKind::Control(_) => (None, 0.),
            };
            gs.push(g);
            widths.push(w);
        }
        let mut total: f64 = widths.iter().sum();
        if self.doc.header.a("binaryHwp")=="1"{base=base.max(line_y+gs.iter().flatten().map(|g|g.ascent).fold(0.,f64::max));}
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
            && ((align == "JUSTIFY" && li + 1 < segs.len())
                || align == "DISTRIBUTE"
                || total > avail)
        {
            gap = (avail - total) / stretch as f64;
        }
        let origin = x;
        for (idx, it) in its.iter().enumerate() {
            match &it.kind {
                ItemKind::Char(c) => {
                    let g = gs[idx].as_ref().unwrap();
                    self.current.ops.push(Op::Text {
                        x,
                        y: base,
                        text: c.to_string(),
                        font: g.font,
                        size: g.size,
                        scale: g.scale,
                        color: g.color,
                    });
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
                    self.object(n, ox, line_y, base, x, width)?;
                    x += widths[idx];
                }
                ItemKind::Control(n) => match n.name.as_str() {
                    "charOverlay"=>{
                        let g=gs[idx].as_ref().unwrap();let w=widths[idx];let y=base-g.size*0.85;
                        self.report.bounds.push(Bounds{page:self.pages.len()+1,kind:"intentional-overlay".into(),id:format!("{}-{}",p.a("id"),it.pos),x,y,w,h:g.size});
                        let border=n.u("borderType");
                        if border==1{for i in 0..48{let a=i as f64*std::f64::consts::TAU/48.;let b=(i+1)as f64*std::f64::consts::TAU/48.;self.current.ops.push(Op::Line{x:x+w/2.+w/2.*a.cos(),y:y+w/2.+w/2.*a.sin(),x2:x+w/2.+w/2.*b.cos(),y2:y+w/2.+w/2.*b.sin(),width:0.4,color:g.color});}}
                        else if border!=0{self.box_lines(x,y,w,g.size,0.4,g.color);if border!=2{self.report.warnings.insert(format!("Overlay border type {border} approximated by rectangle"));}}
                        let text=n.a("text");let factor=((g.size+n.n("innerSize"))/g.size).clamp(0.2,1.);
                        let chars:Vec<String>=if n.a("spread")=="1"{vec![text.into()]}else{text.chars().map(|c|c.to_string()).collect()};
                        for (i,t)in chars.iter().enumerate(){let style=n.children.get(i).map(|c|c.a("id")).filter(|s|self.doc.chars.contains_key(*s)).unwrap_or(&it.style);let gg=self.glyph(t.chars().next().unwrap_or(' '),style)?;let size=g.size*factor;let tw=self.fonts.width(gg.font,t,size);let scale=if tw>w{w/tw}else{1.};self.current.ops.push(Op::Text{x:x+(w-tw*scale)/2.,y:y+(g.size-size)/2.+size*0.85,text:t.clone(),font:gg.font,size,scale,color:gg.color});}
                        x+=w;
                    },
                    "ruby"=>{let g=gs[idx].as_ref().unwrap();let w=widths[idx];self.current.ops.push(Op::Text{x,y:base,text:n.a("main").into(),font:g.font,size:g.size,scale:g.scale,color:g.color});let size=g.size*0.5;let tw=self.fonts.width(g.font,n.a("sub"),size);self.current.ops.push(Op::Text{x:x+(w-tw)/2.,y:if n.a("position")=="1"{base+size}else{base-g.size},text:n.a("sub").into(),font:g.font,size,scale:1.,color:g.color});x+=w;},
                    "footNote"=>{let g=self.glyph('1',&it.style)?;self.current.ops.push(Op::Text{x,y:base-g.size*0.35,text:format!("{})",n.a("number")),font:g.font,size:g.size*0.6,scale:1.,color:g.color});x+=g.size*0.6;self.footnotes.push((n.clone(),ox,width));},
                    "autoNum" => {
                        let num = if n.a("numType") == "PAGE" {
                            (self.pages.len() as i64+self.page_base).max(1) as usize
                        } else {
                            n.u("num")
                        };
                        let g = self.glyph('1', &it.style)?;
                        self.current.ops.push(Op::Text {
                            x,
                            y: base,
                            text: num.to_string(),
                            font: g.font,
                            size: g.size,
                            scale: g.scale,
                            color: g.color,
                        });
                        x += self.fonts.width(g.font, &num.to_string(), g.size);
                    }
                    "endNote" if self.depth == 0 => {
                        self.notes.push(n.clone());
                    }
                    _ => {}
                },
            }
        }
        if let Some((label, g)) = number_draw {
            self.current.ops.push(Op::Text {
                x: ox,
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
        for p in list.children_named("p") {
            for i in 0..lines(p).len() {
                self.line(p, i, x, y, w)?;
            }
        }
        self.depth -= 1;
        Ok(())
    }
    fn measured_height(&mut self,list:&Node)->Result<f64>{
        let mut height=content_height(list);
        for p in list.children_named("p"){let ls=lines(p);let items=Self::items(p);
            for (i,l)in ls.iter().enumerate(){let end=ls.get(i+1).map(|n|n.u("textpos")).unwrap_or(usize::MAX);let mut ascent=l.n("baseline")/100.;let mut descent=0f64;
                for it in items.iter().filter(|it|it.pos>=l.u("textpos")&&it.pos<end){if let ItemKind::Char(c)=it.kind{let g=self.glyph(c,&it.style)?;ascent=ascent.max(g.ascent);descent=descent.max(g.descent);}}
                height=height.max(l.n("vertpos")/100.+ascent+descent);
            }
        }Ok(height)
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
        let w = sz.n("width") / 100.;
        let h = sz.n("height") / 100.;
        let m = margin(n, "outMargin");
        let declared_h = h;
        let h = effective_height(n);
        let inline = pos.a("treatAsChar") == "1";
        let x;

        let y = if inline {
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
        match n.name.as_str() {
            "polygon"|"curve"|"arc"=>{
                let sx=w/(n.n("originalWidth")/100.).max(0.01);let sy=h/(n.n("originalHeight")/100.).max(0.01);
                let mut points:Vec<_>=n.children_named("point").map(|p|(x+p.n("x")/100.*sx,y+p.n("y")/100.*sy)).collect();
                if n.name=="arc"{let c=n.child("center").context("arc center")?;let a=n.child("axis1").context("arc axis")?;let b=n.child("axis2").context("arc axis")?;for i in 0..=32{let t=i as f64*std::f64::consts::FRAC_PI_2/32.;points.push((x+(c.n("x")+(a.n("x")-c.n("x"))*t.cos()+(b.n("x")-c.n("x"))*t.sin())/100.*sx,y+(c.n("y")+(a.n("y")-c.n("y"))*t.cos()+(b.n("y")-c.n("y"))*t.sin())/100.*sy));}}
                if n.name=="curve"{self.report.warnings.insert("Curve knots currently connected by line segments; cubic interpolation pending".into());}
                let line=n.child("lineShape");let fill=n.find("winBrush").map(|f|color(f.a("faceColor")));self.current.ops.push(Op::Path{points,closed:n.name=="polygon",width:val(line,"width").max(0.1),color:color(line.map(|s|s.a("color")).unwrap_or("#000000")),fill});
            },
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
                    self.current.ops.push(Op::Image { x, y, w, h, id });
                    self.report.pictures += 1
                } else {
                    bail!("Missing binary {id}")
                }
            }
            "tbl" => {
                self.table(n, x, y, w, h)?;
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
        for caption in n.children_named("caption"){
            if let Some(list)=caption.child("subList"){
                let gap=caption.n("gap")/100.;let ch=content_height(list);let cw=if matches!(caption.a("side"),"LEFT"|"RIGHT"){caption.n("width")/100.}else{w};
                let (cx,cy)=match caption.a("side"){"LEFT"=>(x-gap-cw,y),"RIGHT"=>(x+w+gap,y),"TOP"=>(x,y-gap-ch),_=>(x,y+h+gap)};
                self.local(list,cx,cy,cw)?;
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
        let mut minimum=vec![0f64;rows];
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
            let typed=if let Some(list)=c.child("subList"){self.measured_height(list)?}else{0.};
            let needed = content.max(typed) + m[2] + m[3];
            if rs==1{minimum[row]=minimum[row].max(needed);}else{let missing=(needed-minimum[row..row+rs].iter().sum::<f64>()).max(0.);minimum[row+rs-1]+=missing;}
            let current: f64 = rh[row..row + rs].iter().sum();
            if needed > current {
                rh[row + rs - 1] += needed - current;
            }
        }
        let mut total: f64 = rh.iter().sum();
        let min_total:f64=minimum.iter().sum();
        if n.a("binaryHwp")=="1"&&total>h&&h>=min_total&&total>min_total{
            let ratio=(h-min_total)/(total-min_total);for(i,height)in rh.iter_mut().enumerate(){*height=minimum[i]+(*height-minimum[i])*ratio;}total=h;
        }
        if total < h
            && let Some(last) = rh.last_mut()
        {
            *last += h - total;
        }
        // Cached table heights include minimum/automatic row growth; keep them
        // instead of stretching cells to a stale object rectangle.
        let header_cells:Vec<_>=cells.iter().filter(|c|c.child("cellAddr").is_some_and(|a|a.u("rowAddr")==0)).map(|c|(*c).clone()).collect();
        let mut page_row=0;let mut page_y=y;let mut previous_row=None;
        for c in &cells {
            let a = c.child("cellAddr").unwrap();
            let s = c.child("cellSpan").unwrap();
            let col = a.u("colAddr");
            let row = a.u("rowAddr");
            if self.depth==0 && (n.a("pageBreak")=="CELL" || total>self.ph-self.top-self.bottom) && previous_row!=Some(row) && row>page_row
                && page_y+rh[page_row..=row].iter().sum::<f64>()>self.ph-self.bottom {
                let merged_crossing=cells.iter().any(|cell|{let a=cell.child("cellAddr").unwrap();let s=cell.child("cellSpan").unwrap();a.u("rowAddr")<row&&a.u("rowAddr")+s.u("rowSpan")>row});
                if !merged_crossing{
                    self.finish()?;let section=&self.doc.sections[self.active_section];let columns=section.find("colPr");self.decorations(section,false,columns.map(|n|n.u("colCount")).unwrap_or(1),val(columns,"sameGap"))?;
                    page_y=self.top;page_row=row;
                    if n.a("repeatHeader")=="1"&&header_cells.iter().all(|c|c.child("cellSpan").unwrap().u("rowSpan")==1){
                        let mut header=n.clone();header.attrs.insert("rowCnt".into(),"1".into());header.attrs.insert("pageBreak".into(),"NONE".into());header.attrs.insert("repeatHeader".into(),"0".into());header.children.retain(|c|c.name!="tr");header.children.push(Node{name:"tr".into(),children:header_cells.clone(),..Default::default()});self.table(&header,x,page_y,w,rh[0])?;page_y+=rh[0];
                    }
                    self.report.warnings.insert("Long table paginated by row with repeated header".into());
                }
            }
            previous_row=Some(row);
            let xx = x + cw[..col].iter().sum::<f64>();
            let yy = page_y + rh[page_row..row].iter().sum::<f64>();
            let ww = cw[col..col + s.u("colSpan")].iter().sum::<f64>();
            let hh = rh[row..row + s.u("rowSpan")].iter().sum::<f64>();
            self.border(c.a("borderFillIDRef"), xx, yy, ww, hh);
            if let Some(list) = c.child("subList") {
                let m = if c.a("hasMargin") == "1" {
                    margin(c, "cellMargin")
                } else {
                    margin(n, "inMargin")
                };
                let (min, max) = floating_extent(list);
                let spare = (hh - m[2] - m[3] - (max - min)).max(0.);
                let shift = match list.a("vertAlign") {
                    "CENTER" => spare / 2.,
                    "BOTTOM" => spare,
                    _ => 0.,
                };
                self.local(list, xx + m[0], yy + m[2] + shift - min, ww - m[0] - m[1])?;
            }
        }
        let _ = (w, h);
        Ok(())
    }
    fn finish(&mut self) -> Result<()> {
        let notes=std::mem::take(&mut self.footnotes);
        if !notes.is_empty(){let height=notes.iter().filter_map(|(n,_,_)|n.child("subList")).map(|l|content_height(l)+4.).sum::<f64>();let mut y=self.ph-self.bottom-height-6.;let x=notes[0].1;self.current.ops.push(Op::Line{x,y:y-4.,x2:x+120.,y2:y-4.,width:0.5,color:[0.;3]});for(n,x,w)in notes{if let Some(list)=n.child("subList"){self.local(list,x,y,w)?;y+=content_height(list)+4.;}}}

        self.current.width = self.pw;
        self.current.height = self.ph;
        self.pages.push(std::mem::take(&mut self.current));
        Ok(())
    }
    fn decorations(&mut self, section: &Node, first: bool, cols: usize, gap: f64) -> Result<()> {
        let hiding=if first{section.find("pageHiding")}else{None};
        if let Some(number) = section.find("pageNumPos").filter(|_|hiding.is_none_or(|h|h.a("hidePageNum")!="1")) {
            let pos = number.u("position");
            if pos > 0 {
                let page = (self.pages.len() as i64+self.page_base).max(1) as usize;
                let font = self.fonts.get("times.ttf")?;
                let label = format!("{}{}{}", number.a("prefix"), page, number.a("suffix"));
                let w = self.fonts.width(font, &label, 10.);
                let right = match pos {
                    3 | 6 => true,
                    7 | 8 => page % 2 == 1,
                    9 | 10 => page % 2 == 0,
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
            self.border(
                b.a("borderFillIDRef"),
                self.left - m[0],
                self.top - m[2],
                self.pw - self.left - self.right + m[0] + m[1],
                self.ph - self.top - self.bottom + m[2] + m[3],
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
            if hiding.is_some_and(|h|h.a("hideMasterPage")=="1"){continue}
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
                            let parity=match c.a("applyPageType"){"ODD"=>(self.pages.len()+1)%2==1,"EVEN"=>(self.pages.len()+1)%2==0,_=>true};
                            if c.name == "header" && parity && hiding.is_none_or(|h|h.a("hideHeader")!="1") && (!first||(sec.a("binaryHwp")=="1"&&sec.a("hideFirstHeader")!="1")) {
                                self.local(
                                    list,
                                    self.left,
                                    self.top - self.header,
                                    self.pw - self.left - self.right,
                                )?;
                            }
                            if c.name == "footer" && parity && hiding.is_none_or(|h|h.a("hideFooter")!="1") && !(first&&sec.a("hideFirstFooter")=="1") {
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
    pub fn render(&mut self) -> Result<()> {
        let mut note_start = (0., 0usize, 0., 0.);
        for (section_index, section) in self.doc.sections.iter().enumerate() {
            self.active_section=section_index;
            if let Some(number)=section.find("newNumber").filter(|n|n.a("type")=="0"){self.page_base=number.u("number") as i64-self.pages.len() as i64;}
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
            let col = section.find("colPr");
            let cols = col.map(|c| c.u("colCount")).unwrap_or(1).max(1);
            let gap = val(col, "sameGap");
            let cw = (self.pw - self.left - self.right - gap * (cols - 1) as f64) / cols as f64;
            self.decorations(section, true, cols, gap)?;
            let mut col = 0;
            let mut last = None;
            let mut flow_shift = 0.;
            for p in section.children_named("p") {
                let ls = lines(p);
                if ls.is_empty() {
                    bail!("Missing saved layout cache in paragraph {}", p.a("id"))
                }
                for (i, l) in ls.iter().enumerate() {
                    let y = l.n("vertpos");
                    if last.is_some_and(|last| y < last) || (i == 0 && p.a("pageBreak") == "1") {
                        col += 1;
                        flow_shift = 0.;
                        if i == 0 && p.a("pageBreak") == "1" {
                            col = cols
                        }
                        if col >= cols {
                            self.finish()?;
                            col = 0;
                            self.decorations(section, false, cols, gap)?;
                        }
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
                }
            }
            if section_index + 1 < self.doc.sections.len() {
                self.finish()?;
            }
        }
        if !self.notes.is_empty() {
            let notes = std::mem::take(&mut self.notes);
            let (mut y, mut col, width, gap) = note_start;
            for n in notes {
                if let Some(list) = n.child("subList") {
                    let h = content_height(list);
                    if y + h > self.ph - self.bottom {
                        col += 1;
                        y = self.top;
                        if col == 2 {
                            self.finish()?;
                            col = 0;
                        }
                    }
                    self.local(list, self.left + col as f64 * (width + gap), y, width)?;
                    y += h + 8.;
                }
            }
            self.report.warnings.insert("Endnotes appended with independent column flow (not Hancom's exact note pagination)".into());
        }
        if !self.current.ops.is_empty() {
            self.finish()?;
        }
        self.report.pages = self.pages.len();
        self.report.warnings.insert("Saved-layout renderer: font substitutions can differ from Hancom; no general paragraph reflow".into());
        Ok(())
    }
}
