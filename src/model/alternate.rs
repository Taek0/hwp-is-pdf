//! Resolve HWPX alternate representations without executing embedded objects.
use super::Node;
use anyhow::{Context, Result, ensure};
use std::{
    collections::BTreeMap,
    io::{Cursor, Read},
};

pub(super) fn resolve(node: &mut Node, binaries: &mut BTreeMap<String, Vec<u8>>) -> Result<()> {
    let mut children = Vec::new();
    for mut child in std::mem::take(&mut node.children) {
        if child.name != "switch" {
            resolve(&mut child, binaries)?;
            children.push(child);
            continue;
        }
        let chart = child
            .children_named("case")
            .filter(|case| {
                case.a("required-namespace") == "http://www.hancom.co.kr/hwpml/2016/ooxmlchart"
            })
            .find_map(|case| case.child("chart"));
        if let Some(chart) = chart {
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
            resolve(&mut fallback, binaries)?;
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
    fn unknown_extension_selects_default_once() {
        let mut node = crate::model::xml(br#"<run><switch><case required-namespace="unknown"><t>wrong</t></case><default><t>visible</t></default></switch></run>"#).unwrap();
        resolve(&mut node, &mut BTreeMap::new()).unwrap();
        assert_eq!(crate::model::text(&node), "visible");
        assert_eq!(node.children.len(), 1);
    }
    #[test]
    fn missing_alternate_is_an_error_not_silent_empty_output() {
        let mut node = crate::model::xml(b"<run><switch><case/></switch></run>").unwrap();
        assert!(resolve(&mut node, &mut BTreeMap::new()).is_err());
    }
}
