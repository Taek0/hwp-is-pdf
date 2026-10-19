//! Resolve HWPX alternate representations without executing embedded objects.
use super::Node;
use anyhow::{Context, Result, ensure};
use std::{
    collections::BTreeMap,
    io::{Cursor, Read},
};

pub(super) fn resolve(
    node: &mut Node,
    binaries: &mut BTreeMap<String, Vec<u8>>,
    files: &BTreeMap<String, Vec<u8>>,
) -> Result<()> {
    let mut children = Vec::new();
    for mut child in std::mem::take(&mut node.children) {
        if child.name != "switch" {
            resolve(&mut child, binaries, files)?;
            children.push(child);
            continue;
        }
        // Modern character-unit properties include explicit HWPUNIT values.
        // Select one representation before consumers enumerate direct children
        // (notably tab stops); recursively searching both branches is ambiguous.
        if let Some(case) = child.children_named("case").find(|case| {
            case.a("required-namespace") == "http://www.hancom.co.kr/hwpml/2016/HwpUnitChar"
        }) {
            let mut selected = case.clone();
            resolve(&mut selected, binaries, files)?;
            children.extend(selected.children);
            continue;
        }
        let chart = child
            .children_named("case")
            .filter(|case| {
                case.a("required-namespace") == "http://www.hancom.co.kr/hwpml/2016/ooxmlchart"
            })
            .find_map(|case| case.child("chart"));
        if let Some(chart) = chart {
            if let Some(bytes) = files.get(chart.a("chartIDRef")) {
                let mut source = super::xml(bytes)?;
                if crate::chart::supports_source_chart(&source) {
                    source.attrs.insert("nonBody".into(), "1".into());
                    if let Some(child) = source.children.iter_mut().find(|n| n.name == "chart") {
                        child.name = "ooxmlChart".into();
                    }
                    let mut chart = chart.clone();
                    chart.children.push(source);
                    children.push(chart);
                    continue;
                }
            }
            let ole = child
                .child("default")
                .and_then(|d| d.child("ole"))
                .context("Chart alternate has no static OLE representation")?;
            let id = ole.a("binaryItemIDRef");
            let raw = binaries.get(id).context("Missing alternate OLE binary")?;
            let signature = [0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1];
            let raw = if raw.get(4..12) == Some(&signature) {
                &raw[4..]
            } else {
                raw.as_slice()
            };
            let mut storage = cfb::CompoundFile::open(Cursor::new(raw))?;
            let mut bytes = Vec::new();
            storage
                .open_stream("/\u{2}OlePres000")?
                .take(64 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)?;
            ensure!(
                bytes.len() <= 64 * 1024 * 1024,
                "Oversized chart presentation"
            );
            let png = crate::metafile::presentation::picture(&bytes)
                .context("Unsupported complete chart presentation")?;
            let mut chart = chart.clone();
            let image_id = format!("{id}-alternate-chart-presentation");
            chart
                .attrs
                .insert("presentationIDRef".into(), image_id.clone());
            binaries.insert(image_id, png);
            children.push(chart);
        } else {
            let mut fallback = child
                .child("default")
                .cloned()
                .context("Unsupported HWPX switch without default")?;
            resolve(&mut fallback, binaries, files)?;
            children.extend(fallback.children);
        }
    }
    node.children = children;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn character_unit_tabs_and_margins_select_one_supported_branch() {
        let mut node = crate::model::xml(br#"<head><tabPr><switch><case required-namespace="http://www.hancom.co.kr/hwpml/2016/HwpUnitChar"><tabItem pos="5000" unit="HWPUNIT"/></case><default><tabItem pos="10000"/></default></switch></tabPr><paraPr><switch><case required-namespace="http://www.hancom.co.kr/hwpml/2016/HwpUnitChar"><margin><left value="1600"/></margin></case><default><margin><left value="3200"/></margin></default></switch></paraPr></head>"#).unwrap();
        resolve(&mut node, &mut BTreeMap::new(), &BTreeMap::new()).unwrap();
        let tabs = node.child("tabPr").unwrap();
        assert_eq!(tabs.children_named("tabItem").count(), 1);
        assert_eq!(tabs.child("tabItem").unwrap().n("pos"), 5000.);
        assert_eq!(
            node.child("paraPr")
                .unwrap()
                .find("left")
                .unwrap()
                .n("value"),
            1600.
        );
        assert!(node.find("switch").is_none());
    }
    #[test]
    fn supported_source_chart_does_not_require_an_ole_preview() {
        let mut node=super::super::xml(br#"<run><switch><case required-namespace="http://www.hancom.co.kr/hwpml/2016/ooxmlchart"><chart chartIDRef="Chart/chart1.xml"/></case></switch></run>"#).unwrap();
        let source=br#"<chartSpace><chart><plotArea><barChart><grouping val="clustered"/></barChart><catAx/><valAx/></plotArea></chart></chartSpace>"#.to_vec();
        let files = BTreeMap::from([("Chart/chart1.xml".into(), source)]);
        resolve(&mut node, &mut BTreeMap::new(), &files).unwrap();
        let chart = node.child("chart").unwrap();
        assert!(chart.child("chartSpace").is_some());
        assert!(chart.a("presentationIDRef").is_empty());
    }
    #[test]
    fn unknown_extension_selects_default_once() {
        let mut node = crate::model::xml(br#"<run><switch><case required-namespace="unknown"><t>wrong</t></case><default><t>visible</t></default></switch></run>"#).unwrap();
        resolve(&mut node, &mut BTreeMap::new(), &BTreeMap::new()).unwrap();
        assert_eq!(crate::model::text(&node), "visible");
        assert_eq!(node.children.len(), 1);
    }
    #[test]
    fn missing_alternate_is_an_error_not_silent_empty_output() {
        let mut node = crate::model::xml(b"<run><switch><case/></switch></run>").unwrap();
        assert!(resolve(&mut node, &mut BTreeMap::new(), &BTreeMap::new()).is_err());
    }
}
