//! Inspect source graphic geometry independently of text-overlap QA.
use anyhow::Result;
use hwp_engine::model::{Document, Node};
use serde_json::{Value, json};
use std::path::Path;
fn properties(n: &Node) -> Value {
    json!({"kind":n.name,"attrs":n.attrs,"children":n.children.iter()
        .filter(|c| c.name != "drawText" && c.name != "subList")
        .map(|c| json!({"kind":c.name,"attrs":c.attrs})).collect::<Vec<_>>()})
}
fn walk(n: &Node, path: &str, out: &mut Vec<Value>) {
    let here = format!("{path}/{}:{}", n.name, n.a("id"));
    if matches!(
        n.name.as_str(),
        "line" | "rect" | "ellipse" | "polygon" | "curve" | "arc" | "group"
    ) {
        out.push(json!({"path":here,"node":properties(n)}));
    }
    for c in &n.children {
        walk(c, &here, out);
    }
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let manifest: Vec<Value> = serde_json::from_slice(&std::fs::read(&args[0])?)?;
    let mut result = Vec::new();
    for entry in manifest {
        let path = entry["input"]
            .as_str()
            .unwrap()
            .replace('\\', "/")
            .replace("D:/", "/mnt/d/");
        let d = Document::open(Path::new(&path))?;
        let mut objects = Vec::new();
        for n in d.sections.iter().chain(&d.masters) {
            walk(n, "", &mut objects);
        }
        let diagonal_borders: Vec<_> = d
            .borders
            .values()
            .filter(|b| {
                ["slash", "backSlash"].iter().any(|name| {
                    b.child(name)
                        .is_some_and(|d| !matches!(d.a("type"), "" | "0" | "NONE"))
                })
            })
            .map(properties)
            .collect();
        result.push(
            json!({"name":entry["name"],"objects":objects,"diagonal_borders":diagonal_borders}),
        );
    }
    std::fs::write(&args[1], serde_json::to_vec_pretty(&result)?)?;
    println!("Inspected {} inputs", result.len());
    Ok(())
}
