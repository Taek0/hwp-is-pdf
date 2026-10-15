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
    if let Some(query) = a.get(3) {
        for section in &d.sections {
            for p in section.children_named("p") {
                let plain = p
                    .children_named("run")
                    .flat_map(|r| &r.children)
                    .filter(|n| n.name == "t")
                    .map(hwp_engine::model::text)
                    .collect::<String>();
                if plain.contains(query) {
                    println!(
                        "ROOT {:?} TEXT {:?} LINES {:?}",
                        p.attrs,
                        plain,
                        p.child("linesegarray")
                    );
                    for o in p
                        .children_named("run")
                        .flat_map(|r| &r.children)
                        .filter(|n| n.child("pos").is_some())
                    {
                        println!(
                            "OBJECT {:?} POS {:?} SIZE {:?} CAPTION {:?}",
                            o.attrs,
                            o.child("pos"),
                            o.child("sz"),
                            o.child("caption").map(|c| &c.attrs)
                        );
                    }
                }
            }
            let mut cells = Vec::new();
            section.all("tc", &mut cells);
            for cell in cells
                .into_iter()
                .filter(|c| hwp_engine::model::text(c).contains(query))
            {
                println!(
                    "CELL {:?} LIST {:?}",
                    cell.child("cellSz"),
                    cell.child("subList").map(|n| &n.attrs)
                );
                let mut paras = Vec::new();
                cell.all("p", &mut paras);
                for p in paras {
                    println!(
                        "PARA {:?} STYLE {:?}",
                        hwp_engine::model::text(p),
                        d.paras.get(p.a("paraPrIDRef"))
                    );
                    let mut seen = std::collections::BTreeSet::new();
                    for r in p.children_named("run") {
                        if seen.insert(r.a("charPrIDRef")) {
                            println!("CHAR {:?}", d.chars.get(r.a("charPrIDRef")));
                        }
                    }
                }
            }
        }
        println!("FONTS {:?}", d.fonts);
        return Ok(());
    }
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
