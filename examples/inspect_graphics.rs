use hwp_engine::{
    layout::Engine,
    model::{Document, Node},
    pdf::Op,
};
fn walk(n: &Node) {
    if n.name == "pic" || n.name == "rect" || n.name == "line" {
        println!(
            "{} {} size={:?} current={:?} rotation={:?} clip={:?} dim={:?}",
            n.name,
            n.a("id"),
            n.child("sz"),
            n.child("curSz"),
            n.child("rotationInfo"),
            n.child("imgClip"),
            n.child("imgDim")
        );
    }
    for c in &n.children {
        walk(c)
    }
}
fn main() -> anyhow::Result<()> {
    let doc = Document::open(std::path::Path::new(&std::env::args().nth(1).unwrap()))?;
    for s in &doc.sections {
        walk(s)
    }
    for s in &doc.masters {
        walk(s)
    }
    let fonts = std::path::Path::new(if cfg!(windows) {
        "C:/Windows/Fonts"
    } else {
        "/mnt/c/Windows/Fonts"
    });
    let mut e = Engine::new(&doc, fonts);
    e.render()?;
    for p in e.pages {
        for o in p.ops {
            if matches!(o, Op::ImageClip { .. } | Op::TransformBegin { .. }) {
                println!("{o:?}");
            }
        }
    }
    Ok(())
}
