//! Bounded reader for the legacy VtChart 6 presentation archive.
//!
//! Follow object references, never search arbitrary payloads for type signatures.
//! Unsupported class versions fail closed; no native chart library is required.

use std::collections::BTreeMap;
use std::rc::Rc;

type Result<T> = std::result::Result<T, ()>;

#[derive(Clone, Copy)]
pub struct Axis {
    pub min: f64,
    pub max: f64,
    pub divisions: u16,
}

#[derive(Clone)]
pub struct Series {
    pub color: u32,
    pub fill: u32,
    pub width: f64,
    pub secondary: bool,
    pub index: usize,
    pub selectors: Vec<i16>,
}

pub struct Presentation {
    pub axes: [Option<Axis>; 2],
    pub chart_type: i16,
    pub series: Vec<Series>,
    pub extent: [f64; 4],
    pub plot: [f64; 4],
    pub axis_fonts: [Option<Font>; 4],
}

#[derive(Clone, Debug)]
pub struct Font {
    pub face: String,
    pub size: f64,
    pub bold: bool,
    pub italic: bool,
    pub horizontal: u16,
    pub vertical: u16,
}

enum Node {
    Empty,
    Count(usize),
    Array(Vec<Rc<Node>>),
    Text(String),
    Font(Font),
    Axis(Option<Axis>, Option<Font>),
    Series(Series),
    Section([f64; 4]),
    Plot([Option<Axis>; 4], [f64; 4], [Option<Font>; 4]),
}
fn node_font(node: &Node) -> Option<Font> {
    match node {
        Node::Font(f) => Some(f.clone()),
        Node::Array(a) => a.iter().find_map(|n| node_font(n)),
        _ => None,
    }
}

struct Reader<'a> {
    data: &'a [u8],
    at: usize,
    types: BTreeMap<i32, (String, u16)>,
    objects: BTreeMap<i32, Rc<Node>>,
    depth: usize,
    remaining: usize,
}

impl<'a> Reader<'a> {
    fn bytes(&mut self, len: usize) -> Result<&'a [u8]> {
        let end = self.at.checked_add(len).ok_or(())?;
        let bytes = self.data.get(self.at..end).ok_or(())?;
        self.at = end;
        Ok(bytes)
    }

    fn skip(&mut self, len: usize) -> Result<()> {
        self.bytes(len).map(|_| ())
    }

    fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(
            self.bytes(2)?.try_into().map_err(|_| ())?,
        ))
    }

    fn i16(&mut self) -> Result<i16> {
        Ok(self.u16()? as i16)
    }

    fn i32(&mut self) -> Result<i32> {
        Ok(i32::from_le_bytes(
            self.bytes(4)?.try_into().map_err(|_| ())?,
        ))
    }

    fn f64(&mut self) -> Result<f64> {
        Ok(f64::from_le_bytes(
            self.bytes(8)?.try_into().map_err(|_| ())?,
        ))
    }

    fn rect(&mut self) -> Result<[f64; 4]> {
        let mut result = [0.0; 4];
        for value in &mut result {
            *value = f32::from_le_bytes(self.bytes(4)?.try_into().map_err(|_| ())?) as f64;
            if !value.is_finite() {
                return Err(());
            }
        }
        Ok(result)
    }

    fn class(&mut self) -> Result<(String, u16)> {
        let id = self.i32()?;
        if let Some(class) = self.types.get(&id) {
            return Ok(class.clone());
        }
        if id < 0 || self.types.len() >= 64 {
            return Err(());
        }
        let len = self.u16()? as usize;
        if !(2..=64).contains(&len) {
            return Err(());
        }
        let bytes = self.bytes(len)?;
        if bytes.last() != Some(&0) {
            return Err(());
        }
        let name = std::str::from_utf8(&bytes[..len - 1])
            .map_err(|_| ())?
            .to_owned();
        let version = self.u16()?;
        self.types.insert(id, (name.clone(), version));
        Ok((name, version))
    }

    fn base(&mut self, expected: &str) -> Result<Node> {
        let (name, version) = self.class()?;
        if name != expected {
            return Err(());
        }
        self.body(&name, version)
    }

    fn object(&mut self) -> Result<Rc<Node>> {
        self.remaining = self.remaining.checked_sub(1).ok_or(())?;
        let id = self.i32()?;
        if id == -1 {
            return Ok(Rc::new(Node::Empty));
        }
        if let Some(object) = self.objects.get(&id) {
            return Ok(Rc::clone(object));
        }
        if id < 0 || self.depth >= 32 || self.objects.len() >= 32768 {
            return Err(());
        }
        let (name, version) = self.class()?;
        self.depth += 1;
        let result = self.body(&name, version);
        self.depth -= 1;
        let object = Rc::new(result?);
        self.objects.insert(id, Rc::clone(&object));
        Ok(object)
    }

    fn body(&mut self, name: &str, version: u16) -> Result<Node> {
        let expected_version = match name {
            "VtChartPlot" => 4,
            "VtAxis" => 3,
            "VtSeries" | "VtTextBlock" | "VtWindow" => 2,
            _ => 1,
        };
        if version != expected_version {
            return Err(());
        }
        match name {
            "VtObject" => {}
            "VtCollection" => {
                let count = self.u16()? as usize;
                self.base("VtObject")?;
                return Ok(Node::Count(count));
            }
            "VtList" => {
                let Node::Count(0) = self.base("VtCollection")? else {
                    return Err(());
                };
            }
            "VtArray" => {
                let capacity = self.u16()? as usize;
                let Node::Count(count) = self.base("VtCollection")? else {
                    return Err(());
                };
                if count > capacity || capacity > self.remaining {
                    return Err(());
                }
                let mut items = Vec::with_capacity(capacity);
                for _ in 0..capacity {
                    items.push(self.object()?);
                }
                return Ok(Node::Array(items));
            }
            "VtDataGrid" => {
                self.base("VtMatrix")?;
                self.skip(8)?;
            }
            "VtMatrix" => {
                self.base("VtCollection")?;
                let rows = self.u16()? as usize;
                let cols = self.u16()? as usize;
                let count = rows.checked_mul(cols).ok_or(())?;
                if count > self.remaining {
                    return Err(());
                }
                for _ in 0..count {
                    self.object()?;
                }
            }
            "VtString" => {
                let len = self.u16()? as usize;
                let mut value = String::new();
                if len > 0 {
                    // Legacy archives reserve one trailing byte, but newer
                    // writers do not always zero it. Keep the length/bounds
                    // check without treating that byte as a NUL contract.
                    let bytes = self.bytes(len + 1)?;
                    let end = bytes[..len].iter().position(|b| *b == 0).unwrap_or(len);
                    value = encoding_rs::EUC_KR.decode(&bytes[..end]).0.into_owned();
                }
                self.base("VtValue")?;
                return Ok(Node::Text(value));
            }
            "VtInteger" => {
                self.skip(4)?;
                self.base("VtValue")?;
            }
            "VtDouble" => {
                self.skip(10)?;
                self.base("VtValue")?;
            }
            "VtValue" => {
                self.base("VtObject")?;
            }
            "VtBackdrop" => {
                self.skip(50)?;
                self.object()?;
                self.base("VtObject")?;
            }
            "VtFill" => {
                self.skip(34)?;
                self.object()?;
                self.skip(2)?;
                self.base("VtObject")?;
            }
            "VtPicture" => {
                self.skip(2)?;
                if self.i16()? != 0 {
                    // The embedded-picture branch still uses an object reference.
                    // A null reference is a valid empty backdrop/marker, not a
                    // bitmap payload. Nonempty VtPict objects remain unsupported.
                    if self.i32()? != -1 {
                        return Err(());
                    }
                } else {
                    self.object()?;
                }
                self.base("VtObject")?;
            }
            "VtFont" => {
                let name = self.object()?;
                let info = self.bytes(14)?;
                let size = f32::from_le_bytes(info[..4].try_into().map_err(|_| ())?) as f64;
                if !size.is_finite() || size <= 0. || size > 1000. {
                    return Err(());
                }
                let font = Font {
                    face: match name.as_ref() {
                        Node::Text(s) => s.clone(),
                        _ => String::new(),
                    },
                    size,
                    bold: info[4] != 0,
                    italic: info[6] != 0,
                    horizontal: 2,
                    vertical: 2,
                };
                self.base("VtObject")?;
                return Ok(Node::Font(font));
            }
            "VtTextBlock" => {
                let prefix = self.bytes(12)?.to_vec();
                self.object()?;
                let font = self.object()?;
                self.skip(24)?;
                self.object()?;
                self.skip(26)?;
                self.base("VtObject")?;
                if let Node::Font(f) = font.as_ref() {
                    let mut f = f.clone();
                    f.horizontal = u16::from_le_bytes(prefix[4..6].try_into().map_err(|_| ())?);
                    f.vertical = u16::from_le_bytes(prefix[6..8].try_into().map_err(|_| ())?);
                    return Ok(Node::Font(f));
                }
            }
            "VtChartText" => {
                self.object()?;
                self.base("VtChartSection")?;
            }
            "VtChartTitle" | "VtChartFootnote" => {
                self.base("VtChartText")?;
            }
            "VtChartSection" => {
                self.skip(8)?;
                let rect = self.rect()?;
                self.skip(2)?;
                self.object()?;
                self.base("VtObject")?;
                return Ok(Node::Section(rect));
            }
            "VtChartLegend" => {
                self.object()?;
                self.skip(10)?;
                self.base("VtChartSection")?;
            }
            "VtLight3" => {
                self.object()?;
                self.skip(10)?;
                self.base("VtObject")?;
            }
            "VtInfLight3" => {
                self.skip(16)?;
                self.base("VtObject")?;
            }
            "VtSeriesLabel" => {
                self.base("VtTextBlock")?;
            }
            "VtSeriesPoint" => {
                self.object()?;
                self.skip(12)?;
                self.object()?;
                self.object()?;
                self.base("VtObject")?;
            }
            "VtTextFormat" => {
                self.base("VtObject")?;
                self.skip(2)?;
                let value = self.object()?;
                if let Node::Font(f) = value.as_ref() {
                    return Ok(Node::Font(f.clone()));
                }
            }
            "VtValueBlock" => {
                let first = self.object()?;
                let second = self.object()?;
                self.skip(2)?;
                let third = self.object()?;
                self.skip(3)?;
                let base = self.base("VtTextBlock")?;
                for node in [first, second, third] {
                    if let Node::Font(f) = node.as_ref() {
                        return Ok(Node::Font(f.clone()));
                    }
                }
                return Ok(base);
            }
            "VtAxisScaleBlock" => {
                self.object()?;
                self.skip(4)?;
                return self.base("VtValueBlock");
            }
            "VtAxis" => {
                self.skip(6)?;
                let manual = self.i16()? != 0;
                self.skip(74)?;
                let first_font = self.object()?;
                let second_font = self.object()?;
                let font = [second_font, first_font].iter().find_map(|n| node_font(n));
                self.skip(34)?;
                let axis = if manual {
                    let min = self.f64()?;
                    let max = self.f64()?;
                    let divisions = self.u16()?;
                    self.skip(6)?;
                    if !min.is_finite()
                        || !max.is_finite()
                        || max <= min
                        || !(max - min).is_finite()
                        || !(1..=100).contains(&divisions)
                    {
                        return Err(());
                    }
                    Some(Axis {
                        min,
                        max,
                        divisions,
                    })
                } else {
                    None
                };
                self.skip(16)?;
                self.base("VtObject")?;
                return Ok(Node::Axis(axis, font));
            }
            "VtCLineItem" => {
                self.skip(48)?;
                self.object()?;
                self.base("VtObject")?;
            }
            "VtSurfaceDesc" => {
                self.skip(46)?;
                self.object()?;
                self.object()?;
                self.skip(2)?;
                self.object()?;
                self.object()?;
                self.skip(14)?;
                self.base("VtObject")?;
            }
            "VtChartPlot" => {
                self.object()?;
                self.skip(136)?;
                self.object()?;
                let mut axes = [None; 4];
                let mut fonts = [None, None, None, None];
                for (i, axis) in axes.iter_mut().enumerate() {
                    let object = self.object()?;
                    let Node::Axis(value, font) = object.as_ref() else {
                        return Err(());
                    };
                    *axis = *value;
                    fonts[i] = font.clone();
                }
                self.skip(30)?;
                self.object()?;
                self.skip(16)?;
                let Node::Section(rect) = self.base("VtChartSection")? else {
                    return Err(());
                };
                return Ok(Node::Plot(axes, rect, fonts));
            }
            "VtSeries" => return self.series(),
            "VtWindow" => {
                self.base("VtObject")?;
                self.skip(2)?;
            }
            _ => return Err(()),
        }
        Ok(Node::Empty)
    }

    fn series(&mut self) -> Result<Node> {
        // Two VtPen v1 values (22 bytes each), then VtBrush v1 (18 bytes).
        let prefix = self.bytes(62)?;
        for offset in [0, 22, 44] {
            if prefix[offset..offset + 2] != [1, 0] {
                return Err(());
            }
        }
        let color = u32::from_le_bytes(prefix[10..14].try_into().map_err(|_| ())?);
        let fill = u32::from_le_bytes(prefix[50..54].try_into().map_err(|_| ())?);
        let width = f32::from_le_bytes(prefix[14..18].try_into().map_err(|_| ())?) as f64;
        if !width.is_finite() || width < 0.0 {
            return Err(());
        }
        self.object()?;
        self.object()?;
        // Guides, outline pen, point options, label offsets and the v2 name.
        self.skip(12 + 22 + 16 + 16)?;
        self.object()?;
        self.object()?;
        self.object()?;
        self.skip(2)?;
        self.object()?;
        self.object()?;
        let secondary = self.i16()? != 0;
        self.skip(2)?;
        if self.u16()? != 2 {
            // Inline VtMarker v2.
            return Err(());
        }
        self.skip(34)?;
        self.object()?;
        self.skip(2 + 8 + 12)?;
        let index = self.u16()? as usize;
        let count = self.u16()? as usize;
        if count != 27 {
            return Err(());
        }
        let mut selectors = Vec::with_capacity(count);
        for _ in 0..count {
            selectors.push(self.i16()?);
        }
        let extra = self.u16()? as usize;
        if extra > 27 {
            return Err(());
        }
        self.skip(extra * 2 + 10)?;
        self.base("VtObject")?;
        Ok(Node::Series(Series {
            color,
            fill,
            width,
            secondary,
            index,
            selectors,
        }))
    }
}

pub fn parse(data: &[u8]) -> Option<Presentation> {
    parse_inner(data).ok()
}

fn parse_inner(data: &[u8]) -> Result<Presentation> {
    if data.len() > 16 * 1024 * 1024 {
        return Err(());
    }
    // Fixed legacy Contents header, not a fixture-specific object offset.
    let mut reader = Reader {
        data,
        at: 40,
        types: BTreeMap::new(),
        objects: BTreeMap::new(),
        depth: 0,
        remaining: 262144,
    };
    let (name, version) = reader.class()?;
    if name != "VtChart" || version != 6 {
        return Err(());
    }
    reader.object()?;
    reader.skip(2)?;
    let extent = reader.rect()?;
    reader.object()?;
    reader.object()?;
    reader.object()?;
    let plot = reader.object()?;
    let series = reader.object()?;
    reader.object()?;
    reader.object()?;
    reader.skip(6)?;
    reader.object()?;
    reader.skip(10)?;
    let chart_type = reader.i16()?;
    reader.skip(4)?;
    reader.base("VtWindow")?;
    if !(0..=26).contains(&chart_type) || reader.at != data.len() {
        return Err(());
    }
    let Node::Plot(axes, plot, axis_fonts) = plot.as_ref() else {
        return Err(());
    };
    let Node::Array(items) = series.as_ref() else {
        return Err(());
    };
    if items.is_empty() || items.len() > 64 {
        return Err(());
    }
    let mut styles = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let Node::Series(style) = item.as_ref() else {
            return Err(());
        };
        // Painter 1: column; painter 6: line, selected by chart type 9.
        if style.index != index {
            return Err(());
        }
        styles.push(style.clone());
    }

    for rect in [extent, *plot] {
        if rect[2] <= rect[0] || rect[3] <= rect[1] {
            return Err(());
        }
    }
    Ok(Presentation {
        axes: [axes[1], axes[2]],
        chart_type,
        series: styles,
        extent,
        plot: *plot,
        axis_fonts: axis_fonts.clone(),
    })
}
