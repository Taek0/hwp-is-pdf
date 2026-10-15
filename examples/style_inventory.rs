//! Used source font/paragraph definitions with short text samples.
use hwp_engine::model::{Document, text};
fn main() -> anyhow::Result<()> {
    let d = Document::open(std::path::Path::new(&std::env::args().nth(1).unwrap()))?;
    let mut chars = std::collections::BTreeMap::<String, String>::new();
    let mut paras = std::collections::BTreeMap::<String, String>::new();
    for s in &d.sections {
        let mut nodes = Vec::new();
        s.all("run", &mut nodes);
        for r in nodes {
            chars
                .entry(r.a("charPrIDRef").into())
                .or_default()
                .push_str(&text(r));
        }
        let mut nodes = Vec::new();
        s.all("p", &mut nodes);
        for p in nodes {
            paras
                .entry(p.a("paraPrIDRef").into())
                .or_default()
                .push_str(&text(p));
        }
    }
    println!("FONTS {:?}", d.fonts);
    for (id, t) in chars {
        let c = &d.chars[&id];
        println!(
            "CHAR {id} size={} bold={} italic={} fonts={:?} text={:?}",
            c.a("height"),
            c.child("bold").is_some(),
            c.child("italic").is_some(),
            c.child("fontRef").map(|n| &n.attrs),
            t.chars().take(80).collect::<String>()
        );
    }
    for (id, t) in paras {
        let p = &d.paras[&id];
        println!(
            "PARA {id} border={:?} margin={:?} spacing={:?} text={:?}",
            p.child("border"),
            p.child("margin"),
            p.child("lineSpacing"),
            t.chars().take(40).collect::<String>()
        );
    }
    println!("BORDERS {:?}", d.borders);
    for s in &d.sections {
        for p in s.children_named("p").take(40) {
            println!(
                "ROOT {} style={} first={:?} text={:?}",
                p.a("id"),
                p.a("paraPrIDRef"),
                p.child("linesegarray")
                    .and_then(|l| l.children.first())
                    .map(|l| &l.attrs),
                text(p).chars().take(20).collect::<String>()
            );
        }
    }
    Ok(())
}
