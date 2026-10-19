//! Non-executing static OLE and chart presentation normalization.
use super::*;
pub(crate) fn normalize(
    nodes: &mut [Node],
    binaries: &mut BTreeMap<String, Vec<u8>>,
) -> Result<()> {
    for node in nodes {
        if node.name == "ole" {
            let id = node
                .child("oleRef")
                .map(|n| n.a("binaryItemIDRef"))
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| node.a("binaryItemIDRef"))
                .to_string();
            let raw = binaries.get(&id).context("OLE binary missing")?;
            let raw = if raw.get(4..12) == Some(&[0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1]) {
                &raw[4..]
            } else {
                raw.as_slice()
            };
            let mut storage = cfb::CompoundFile::open(Cursor::new(raw))?;
            if storage.exists("/OOXMLChartContents") {
                let mut xml = Vec::new();
                storage
                    .open_stream("/OOXMLChartContents")?
                    .take(LIMIT as u64 + 1)
                    .read_to_end(&mut xml)?;
                ensure!(xml.len() <= LIMIT, "Oversized OOXML chart");
                let mut source = crate::model::xml(&xml)?;
                if crate::chart::supports_source_chart(&source) {
                    node.name = "chart".into();
                    node.attrs.insert(
                        "nativeType".into(),
                        if source.find("scatterChart").is_some() {
                            "16"
                        } else {
                            "0"
                        }
                        .into(),
                    );
                    source.attrs.insert("nonBody".into(), "1".into());
                    if let Some(chart) = source.children.iter_mut().find(|n| n.name == "chart") {
                        chart.name = "ooxmlChart".into();
                    }
                    node.children.push(source);
                    continue;
                }
                let kind = if source.find("scatterChart").is_some() {
                    Some("scatterChart")
                } else if source.find("bar3DChart").is_some() {
                    Some("bar3DChart")
                } else {
                    None
                };
                if let Some(kind) = kind
                    && storage.exists("/\u{2}OlePres000")
                {
                    let mut data = Vec::new();
                    storage
                        .open_stream("/\u{2}OlePres000")?
                        .take(LIMIT as u64 + 1)
                        .read_to_end(&mut data)?;
                    if let Some(png) = crate::metafile::presentation::picture(&data) {
                        let image_id = format!("{id}-chart-presentation");
                        drop(storage);
                        binaries.insert(image_id.clone(), png);
                        node.name = "chart".into();
                        node.attrs.insert("presentationIDRef".into(), image_id);
                        node.attrs.insert("chartKind".into(), kind.into());
                        continue;
                    }
                }
            }
            if storage.exists("/Contents") {
                let mut contents = vec![];
                storage
                    .open_stream("/Contents")?
                    .take(LIMIT as u64 + 1)
                    .read_to_end(&mut contents)?;
                ensure!(contents.len() <= LIMIT, "Chart data too large");
                if let Some(script) = super::ole_equations::script(&contents)? {
                    let height = node.child("sz").map(|n| n.n("height")).unwrap_or(0.);
                    ensure!(
                        height.is_finite() && height > 0. && height <= 200000.,
                        "Invalid OLE equation extent"
                    );
                    let baseline = if node.n("eqBaseLine") > 0. {
                        node.n("eqBaseLine")
                    } else {
                        63.
                    };
                    node.name = "equation".into();
                    node.attrs.insert("baseLine".into(), baseline.to_string());
                    node.attrs.insert("baseUnit".into(), height.to_string());
                    node.attrs.insert("textColor".into(), "#000000".into());
                    node.attrs.insert("oleEquation".into(), "1".into());
                    node.children.push(n("script").with(vec![txt(script)]));
                    continue;
                }
                if let Some(png) = crate::metafile::enhanced_contents_picture(&contents)? {
                    let image_id = format!("{id}-ole-enhanced");
                    drop(storage);
                    binaries.insert(image_id.clone(), png);
                    node.name = "pic".into();
                    node.attrs.insert("staticOLE".into(), "1".into());
                    node.children.push(n("img").at("binaryItemIDRef", image_id));
                    continue;
                }
                if crate::metafile::is_wmf(&contents) {
                    let image_id = format!("{id}-ole-metafile");
                    drop(storage);
                    binaries.insert(image_id.clone(), contents);
                    node.name = "pic".into();
                    node.attrs.insert("staticOLE".into(), "1".into());
                    node.children.push(n("img").at("binaryItemIDRef", image_id));
                    continue;
                }
                let chart = hwp_legacy_chart::parse_ole_chart_contents(&contents)?;
                let presentation =
                    hwp_legacy_chart::ole_chart::legacy_presentation::parse(&contents);
                node.name = "chart".into();
                node.attrs.insert(
                    "nativeType".into(),
                    presentation
                        .as_ref()
                        .map(|p| p.chart_type.to_string())
                        .unwrap_or_else(|| "unknown".into()),
                );
                node.attrs.insert(
                    "axisEvidence".into(),
                    format!("{:?}", chart.series_axis_evidence),
                );
                node.attrs
                    .insert("title".into(), chart.title.unwrap_or_default());
                if let Some(p) = &presentation {
                    let mut layout = n("nativeLayout");
                    for (name, rect) in [("extent", p.extent), ("plot", p.plot)] {
                        layout.children.push(
                            n(name)
                                .at("left", rect[0])
                                .at("top", rect[1])
                                .at("right", rect[2])
                                .at("bottom", rect[3]),
                        );
                    }
                    for (i, axis) in p.axes.iter().enumerate() {
                        if let Some(axis) = axis {
                            layout.children.push(
                                n("axis")
                                    .at("index", i)
                                    .at("min", axis.min)
                                    .at("max", axis.max)
                                    .at("divisions", axis.divisions),
                            );
                        }
                    }
                    for (i, font) in p.axis_fonts.iter().enumerate() {
                        if let Some(font) = font {
                            layout.children.push(
                                n("axisFont")
                                    .at("index", i)
                                    .at("face", &font.face)
                                    .at("size", font.size)
                                    .at("bold", usize::from(font.bold))
                                    .at("italic", usize::from(font.italic))
                                    .at("horizontal", font.horizontal)
                                    .at("vertical", font.vertical),
                            );
                        }
                    }
                    node.children.push(layout);
                }
                for category in chart.categories {
                    node.children.push(n("category").at("name", category));
                }
                for (i, series) in chart.series.into_iter().enumerate() {
                    let mut s = n("series").at(
                        "name",
                        series.name.unwrap_or_else(|| format!("Series {}", i + 1)),
                    );
                    if let Some(style) = presentation.as_ref().and_then(|p| p.series.get(i)) {
                        s = s
                            .at("color", rgb(style.fill))
                            .at("lineColor", rgb(style.color))
                            .at("lineWidth", style.width)
                            .at("axis", usize::from(style.secondary));
                        if let Some(painter) = presentation
                            .as_ref()
                            .and_then(|p| usize::try_from(p.chart_type).ok())
                            .and_then(|i| style.selectors.get(i))
                        {
                            s = s.at("painter", painter);
                        }
                    }
                    for v in series.values {
                        ensure!(v.is_finite(), "Non-finite chart value");
                        s.children.push(n("value").at("n", v));
                    }
                    node.children.push(s);
                }
                continue;
            }
            let stream = "/\u{1}Ole10Native";
            ensure!(
                storage.exists(stream),
                "OLE has no supported static raster presentation"
            );
            let mut bytes = vec![];
            storage
                .open_stream(stream)?
                .take(LIMIT as u64 + 1)
                .read_to_end(&mut bytes)?;
            ensure!(bytes.len() <= LIMIT, "OLE presentation too large");
            let payload = if bytes.get(4..6) == Some(b"BM") {
                &bytes[4..]
            } else {
                bytes.as_slice()
            };
            let image = image::load_from_memory(payload).context("Unsupported static OLE image")?;
            let mut png = Cursor::new(Vec::new());
            image.write_to(&mut png, image::ImageFormat::Png)?;
            let name = format!("{id}-ole-static");
            binaries.insert(name.clone(), png.into_inner());
            node.name = "pic".into();
            node.attrs.insert("staticOLE".into(), "1".into());
            node.children.push(n("img").at("binaryItemIDRef", name));
        }
        normalize(&mut node.children, binaries)?;
    }
    Ok(())
}
