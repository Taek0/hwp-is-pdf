//! Reconvert the recorded corpus and compare actual decoration drawing commands.
use anyhow::{Result, ensure};
use hwp_engine::{
    layout::Engine,
    model::{Document, Node},
    pdf, qa,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, io::Read, path::Path};
fn raw_flags(path: &Path) -> Result<BTreeMap<String, u32>> {
    let mut result = BTreeMap::new();
    if path.extension().is_none_or(|e| e != "hwp") {
        return Ok(result);
    }
    let mut file = cfb::open(path)?;
    let mut header = vec![];
    file.open_stream("/FileHeader")?.read_to_end(&mut header)?;
    let mut bytes = vec![];
    file.open_stream("/DocInfo")?.read_to_end(&mut bytes)?;
    if header[36] & 1 != 0 {
        let mut decoded = vec![];
        flate2::read::DeflateDecoder::new(bytes.as_slice()).read_to_end(&mut decoded)?;
        bytes = decoded;
    }
    let mut p = 0;
    while p + 4 <= bytes.len() {
        let v = u32::from_le_bytes(bytes[p..p + 4].try_into()?);
        p += 4;
        let mut len = (v >> 20) as usize;
        if len == 4095 {
            len = u32::from_le_bytes(bytes[p..p + 4].try_into()?) as usize;
            p += 4;
        }
        if v & 1023 == 21 {
            result.insert(
                result.len().to_string(),
                u32::from_le_bytes(bytes[p + 46..p + 50].try_into()?),
            );
        }
        p += len;
    }
    Ok(result)
}
fn paths(page: &pdf::Page) -> BTreeMap<String, usize> {
    let mut map = BTreeMap::new();
    for op in &page.ops {
        if matches!(op, pdf::Op::Path { .. }) {
            *map.entry(serde_json::to_string(op).unwrap()).or_default() += 1;
        }
    }
    map
}
fn difference(a: &pdf::Page, b: &pdf::Page) -> Result<usize> {
    let a = paths(a);
    let b = paths(b);
    for (k, n) in &b {
        ensure!(
            a.get(k).copied().unwrap_or(0) >= *n,
            "existing stroke changed or disappeared"
        );
    }
    Ok(a.values().sum::<usize>() - b.values().sum::<usize>())
}
fn geometry(pages: &[pdf::Page]) -> Value {
    json!(
        pages
            .iter()
            .map(|p| json!([
                p.width,
                p.height,
                p.ops
                    .iter()
                    .filter(|o| !matches!(o, pdf::Op::Path { .. }))
                    .collect::<Vec<_>>()
            ]))
            .collect::<Vec<_>>()
    )
}
fn main() -> Result<()> {
    let out = Path::new("output/underline-regression");
    let bytes = std::fs::read(out.join("inputs.json"))?;
    let manifest: Vec<Value> =
        serde_json::from_slice(bytes.strip_prefix(&[239, 187, 191]).unwrap_or(&bytes))?;
    let mut results = vec![];
    for item in manifest {
        let source = item["input"]
            .as_str()
            .unwrap()
            .replace('\\', "/")
            .replace("D:/", "/mnt/d/");
        let path = Path::new(&source);
        let mut doc = Document::open(path)?;
        let flags = raw_flags(path)?;
        let mut reserved = vec![];
        for (id, flag) in &flags {
            let kind = (flag >> 2) & 3;
            ensure!(
                doc.chars[id].child("underline").is_some() == matches!(kind, 1 | 3),
                "raw style mismatch {source} {id}"
            );
            if kind == 2 {
                reserved.push(id.clone());
            }
        }
        let fonts = Path::new("/mnt/c/Windows/Fonts");
        let mut current = Engine::new(&doc, fonts);
        current.render()?;
        let target = out
            .join(item["group"].as_str().unwrap())
            .join(item["name"].as_str().unwrap())
            .with_extension("pdf");
        std::fs::create_dir_all(target.parent().unwrap())?;
        pdf::save(&target, &current.pages, &mut current.fonts, &doc.binaries)?;
        let audit = qa::inspect(&current.pages, &current.fonts, &current.report.bounds);
        std::fs::write(
            target.with_extension("qa.json"),
            serde_json::to_vec_pretty(&audit)?,
        )?;
        std::fs::write(
            target.with_extension("report.json"),
            serde_json::to_vec_pretty(&current.report)?,
        )?;
        let report = serde_json::to_value(&current.report)?;
        let current_pages = std::mem::take(&mut current.pages);
        drop(current);
        let original_chars = doc.chars.clone();
        // Reproduce precisely the old erroneous fallback for reserved type 2.
        for id in &reserved {
            let mut node = Node {
                name: "underline".into(),
                ..Default::default()
            };
            node.attrs.insert("type".into(), "BOTTOM".into());
            node.attrs
                .insert("style".into(), ((flags[id] >> 4) & 15).to_string());
            node.attrs.insert("color".into(), "#000000".into());
            doc.chars.get_mut(id).unwrap().children.push(node);
        }
        let mut old = Engine::new(&doc, fonts);
        old.render()?;
        ensure!(
            geometry(&old.pages) == geometry(&current_pages),
            "geometry changed {source}"
        );
        let removed = old
            .pages
            .iter()
            .zip(&current_pages)
            .map(|(a, b)| difference(a, b))
            .collect::<Result<Vec<_>>>()?;
        drop(old);
        doc.chars = original_chars;
        // Independent control render: remove valid underline attributes only.
        for c in doc.chars.values_mut() {
            c.children.retain(|n| n.name != "underline");
        }
        let mut plain = Engine::new(&doc, fonts);
        plain.render()?;
        ensure!(
            geometry(&plain.pages) == geometry(&current_pages),
            "underline changes text geometry {source}"
        );
        let preserved = current_pages
            .iter()
            .zip(&plain.pages)
            .map(|(a, b)| difference(a, b))
            .collect::<Result<Vec<_>>>()?;
        let qa_value = serde_json::to_value(audit)?;
        let qa_counts: BTreeMap<_, _> = [
            "outside_page",
            "text_overlaps",
            "border_crossings",
            "foreground_image_overlaps",
        ]
        .into_iter()
        .map(|k| (k, qa_value[k].as_array().unwrap().len()))
        .collect();
        println!(
            "{}: {}p removed={} valid_strokes={} qa={qa_counts:?}",
            path.file_name().unwrap().to_string_lossy(),
            current_pages.len(),
            removed.iter().sum::<usize>(),
            preserved.iter().sum::<usize>()
        );
        results.push(json!({"group":item["group"],"name":item["name"],"pages":current_pages.len(),"reserved_styles":reserved,"removed_strokes_by_page":removed,"valid_strokes_by_page":preserved,"qa":qa_counts,"geometry_unchanged":true,"raw_flags_match":true,"report":report}));
        std::fs::write(out.join("audit.json"), serde_json::to_vec_pretty(&results)?)?;
    }
    Ok(())
}
