//! Inspect cached paragraph origins and break settings (including notes with --notes).
use hwp_engine::model::{Document, Node, text};
fn dump(p: &Node, d: &Document) {
    let ls: Vec<_> = p
        .child("linesegarray")
        .into_iter()
        .flat_map(|a| a.children_named("lineseg"))
        .map(|l| &l.attrs)
        .collect();
    println!(
        "{} {:?} style={:?} lines={:?} text={:?}",
        p.a("id"),
        p.attrs,
        d.paras
            .get(p.a("paraPrIDRef"))
            .and_then(|n| n.child("breakSetting")),
        ls,
        text(p).chars().take(65).collect::<String>()
    );
}
fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let d = Document::open(std::path::Path::new(
        args.first()
            .ok_or_else(|| anyhow::anyhow!("INPUT.hwp|hwpx [--notes]"))?,
    ))?;
    for s in &d.sections {
        for p in s.children_named("p") {
            dump(p, &d);
        }
        if args.iter().any(|a| a == "--notes") {
            let mut notes = Vec::new();
            s.all("footNote", &mut notes);
            for note in notes {
                println!("NOTE {}", note.a("number"));
                if let Some(l) = note.child("subList") {
                    for p in l.children_named("p") {
                        dump(p, &d);
                    }
                }
            }
        }
    }
    Ok(())
}
