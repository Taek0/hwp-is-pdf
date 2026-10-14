use hwp_engine::model::{Document, Node, text};
fn walk(n: &Node, depth: usize) {
    if n.name == "tbl" {
        let cells: Vec<_> = n
            .children_named("tr")
            .flat_map(|r| r.children_named("tc"))
            .collect();
        let mut f = vec![];
        n.all("footNote", &mut f);
        println!(
            "TABLE {} depth={} attrs={:?} pos={:?} size={:?} cells={} footnotes={} maxcell={}",
            n.a("id"),
            depth,
            n.attrs,
            n.child("pos").map(|p| &p.attrs),
            n.child("sz").map(|p| &p.attrs),
            cells.len(),
            f.len(),
            cells
                .iter()
                .filter_map(|c| c.child("cellSz"))
                .map(|c| c.n("height"))
                .fold(0f64, f64::max)
        );
    }
    for c in &n.children {
        walk(c, depth + usize::from(n.name == "tbl"));
    }
}
fn main() -> anyhow::Result<()> {
    let d = Document::open(std::path::Path::new(&std::env::args().nth(1).unwrap()))?;
    for s in &d.sections {
        println!("SECTION {:?}", s.find("secPr"));
        for p in s.children_named("p") {
            let mut foot = vec![];
            p.all("footNote", &mut foot);
            if !foot.is_empty() {
                println!(
                    "FOOTPAR {} count={} {:?} text={:?}",
                    p.a("id"),
                    foot.len(),
                    p.child("linesegarray"),
                    text(p).chars().take(30).collect::<String>()
                );
            }
        }
        walk(s, 0)
    }
    Ok(())
}
