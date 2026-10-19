//! Inventory positioned objects in every frozen input.
use anyhow::Result;
use hwp_engine::model::{Document, Node};
use std::{collections::BTreeMap, path::Path};
fn walk(node: &Node, known: &mut BTreeMap<String, usize>, unknown: &mut Vec<serde_json::Value>) {
    if (node.child("pos").is_some() && node.child("sz").is_some())
        || (node.child("offset").is_some() && node.child("curSz").is_some())
    {
        *known.entry(node.name.clone()).or_default() += 1;
        if !matches!(
            node.name.as_str(),
            "tbl"
                | "pic"
                | "equation"
                | "rect"
                | "line"
                | "ellipse"
                | "polygon"
                | "curve"
                | "arc"
                | "group"
                | "chart"
                | "checkBtn"
        ) {
            unknown.push(serde_json::json!({"kind":node.name,"id":node.a("id")}));
        }
    }
    for child in &node.children {
        walk(child, known, unknown);
    }
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let bytes = std::fs::read(&args[0])?;
    let entries: Vec<serde_json::Value> =
        serde_json::from_slice(bytes.strip_prefix(&[239, 187, 191]).unwrap_or(&bytes))?;
    let mut results = Vec::new();
    for e in entries {
        let path = e["input"]
            .as_str()
            .unwrap()
            .replace('\\', "/")
            .replace("D:/", "/mnt/d/");
        match Document::open(Path::new(&path)) {
            Ok(d) => {
                let mut known = BTreeMap::new();
                let mut unknown = Vec::new();
                for root in d.sections.iter().chain(&d.masters) {
                    walk(root, &mut known, &mut unknown);
                }
                results.push(serde_json::json!({"name":e["name"],"input":e["input"],"objects":known,"unknown":unknown}));
            }
            Err(err) => results.push(
                serde_json::json!({"name":e["name"],"input":e["input"],"error":format!("{err:#}")}),
            ),
        }
        std::fs::write(&args[1], serde_json::to_vec_pretty(&results)?)?;
    }
    println!("Inventoried {} inputs", results.len());
    Ok(())
}
