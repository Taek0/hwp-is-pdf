use hwp_engine::model::{Document, Node, text};
fn walk(n: &Node) {
    if n.name == "p" {
        let mut pics = vec![];
        n.all("pic", &mut pics);
        for pic in pics {
            println!(
                "PARA {} text={:?} PIC {} wrap={} pos={:?} size={:?} lines={:?}",
                n.a("id"),
                text(n).chars().take(45).collect::<String>(),
                pic.a("id"),
                pic.a("textWrap"),
                pic.child("pos"),
                pic.child("sz"),
                n.child("linesegarray")
            );
        }
    }
    for c in &n.children {
        walk(c)
    }
}
fn main() -> anyhow::Result<()> {
    let doc = Document::open(std::path::Path::new(&std::env::args().nth(1).unwrap()))?;
    for s in doc.sections {
        walk(&s)
    }
    Ok(())
}
