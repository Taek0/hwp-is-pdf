use anyhow::Result;
use hwp_engine::model::{Document, Node};
fn dump(n: &Node) {
    if n.name == "tc" {
        println!("text={:?}", hwp_engine::model::text(n));
        let a = n.child("cellAddr").unwrap();
        let mut ls = vec![];
        n.all("lineseg", &mut ls);
        println!(
            "cell r{}c{} size={:?} margin={:?} span={:?} first={:?} last={:?}",
            a.a("rowAddr"),
            a.a("colAddr"),
            n.child("cellSz").map(|n| &n.attrs),
            n.child("cellMargin").map(|n| &n.attrs),
            n.child("cellSpan").map(|n| &n.attrs),
            ls.first().map(|n| &n.attrs),
            ls.last().map(|n| &n.attrs)
        );
    }
    for c in &n.children {
        dump(c)
    }
}
fn main() -> Result<()> {
    let a: Vec<_> = std::env::args().collect();
    let d = Document::open(std::path::Path::new(&a[1]))?;
    let mut tables = vec![];
    for s in &d.sections {
        s.all("tbl", &mut tables)
    }
    for t in tables {
        if a.get(2).is_none_or(|id| t.a("id") == id) {
            println!("TABLE {} {:?}", t.a("id"), t.child("sz").map(|n| &n.attrs));
            dump(t)
        }
    }
    Ok(())
}
