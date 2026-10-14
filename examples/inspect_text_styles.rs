//! List used script, offset and italic styles to explain intentional raster changes.
use hwp_engine::model::{Document,text};
fn main()->anyhow::Result<()> {
 let path=std::env::args().nth(1).unwrap();let d=Document::open(std::path::Path::new(&path))?;
 let mut used=std::collections::BTreeMap::<String,String>::new();
 for s in &d.sections {let mut runs=vec![];s.all("run",&mut runs);for r in runs {used.entry(r.a("charPrIDRef").into()).or_default().push_str(&text(r));}}
 for (id,t) in used {let c=&d.chars[&id];let offset=c.child("offset").is_some_and(|n|n.attrs.values().any(|v|v.parse::<f64>().unwrap_or(0.)!=0.));
 if c.child("supscript").is_some()||c.child("subscript").is_some()||c.child("italic").is_some()||offset {
 println!("style={id} height={} sup={} sub={} italic={} relSz={:?} offset={:?} text={:?}",c.a("height"),c.child("supscript").is_some(),c.child("subscript").is_some(),c.child("italic").is_some(),c.child("relSz").map(|n|&n.attrs),c.child("offset").map(|n|&n.attrs),t.chars().take(160).collect::<String>());
 }}Ok(())
}
