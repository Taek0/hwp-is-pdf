use anyhow::Result;
use hwp_engine::model::{Document, Node};
fn dump(n: &Node) {
    if n.name == "tc" {
        println!("text={:?}", hwp_engine::model::text(n));
        println!(
            "CELLATTR {:?} PARAS {:?}",
            n.attrs,
            n.child("subList").map(|l| l
                .children_named("p")
                .map(|p| (p.a("id"), p.child("linesegarray").map(|a| a.children.len())))
                .collect::<Vec<_>>())
        );
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
    if a.get(2).is_some_and(|s| s == "pages") {
        for section in &d.sections {
            let mut previous = None;
            let mut page = 1;
            for p in section.children_named("p") {
                if let Some(ls) = p.child("linesegarray") {
                    for (i, l) in ls.children_named("lineseg").enumerate() {
                        if previous.is_some_and(|y| {
                            l.n("vertpos") < y || (i == 0 && p.a("pageBreak") == "1")
                        }) {
                            page += 1;
                        }
                        if i == 0 {
                            println!(
                                "PAGE {page} {:?} y={} {:?}",
                                p.attrs,
                                l.n("vertpos"),
                                hwp_engine::model::text(p)
                                    .chars()
                                    .take(55)
                                    .collect::<String>()
                            );
                        }
                        previous = Some(l.n("vertpos"));
                    }
                }
            }
        }
        return Ok(());
    }
    if a.get(2).is_some_and(|s| s == "paragraph") {
        let query = a.get(3).map(String::as_str).unwrap_or("");
        for section in &d.sections {
            let mut ps = Vec::new();
            section.all("p", &mut ps);
            for p in ps {
                let plain = p
                    .children_named("run")
                    .flat_map(|r| r.children_named("t"))
                    .map(hwp_engine::model::text)
                    .collect::<String>();
                if plain.contains(query) {
                    println!(
                        "P {:?} TEXT {:?} LINES {:?} STYLE {:?}",
                        p.attrs,
                        plain,
                        p.child("linesegarray"),
                        d.paras.get(p.a("paraPrIDRef"))
                    );
                    for r in p.children_named("run") {
                        println!("CHAR {:?}", d.chars.get(r.a("charPrIDRef")));
                    }
                }
            }
        }
        let mut bullets = Vec::new();
        d.header.all("bullet", &mut bullets);
        println!("BULLETS {bullets:?}");
        return Ok(());
    }
    if let Some(query) = a.get(3) {
        for master in &d.masters {
            let mut boxes = Vec::new();
            master.all("rect", &mut boxes);
            for shape in boxes.into_iter().filter(|s| {
                s.child("drawText")
                    .is_some_and(|n| hwp_engine::model::text(n).contains(query))
            }) {
                println!(
                    "MASTER BOX {:?} POS {:?} SIZE {:?} CURRENT {:?} MARGIN {:?} LIST {:?}",
                    shape.attrs,
                    shape.child("pos"),
                    shape.child("sz"),
                    shape.child("curSz"),
                    shape.child("drawText").and_then(|n| n.child("textMargin")),
                    shape
                        .child("drawText")
                        .and_then(|n| n.child("subList"))
                        .map(|n| &n.attrs)
                );
            }
            let mut ps = Vec::new();
            master.all("p", &mut ps);
            for p in ps {
                let plain = p
                    .children_named("run")
                    .flat_map(|r| r.children_named("t"))
                    .map(hwp_engine::model::text)
                    .collect::<String>();
                if plain.contains(query) {
                    println!(
                        "MASTER {:?} TEXT {:?} LINES {:?} STYLE {:?} CACHE {:?}",
                        master.attrs,
                        plain,
                        p.child("linesegarray"),
                        d.paras.get(p.a("paraPrIDRef")),
                        p.child("linesegarray")
                    );
                    for run in p.children_named("run") {
                        println!("MASTER CHAR {:?}", d.chars.get(run.a("charPrIDRef")));
                    }
                }
            }
        }
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
                    println!("ROOT_STYLE {:?}", d.paras.get(p.a("paraPrIDRef")));
                    for o in p
                        .children_named("run")
                        .flat_map(|r| &r.children)
                        .filter(|n| n.child("pos").is_some())
                    {
                        println!(
                            "OBJECT {:?} POS {:?} SIZE {:?} CAPTION {:?} OUT {:?} IMAGE {:?}",
                            o.attrs,
                            o.child("pos"),
                            o.child("sz"),
                            o.child("caption").map(|c| &c.attrs),
                            o.child("outMargin"),
                            o.child("img")
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
                        "PARA {:?} STYLE {:?} LINES {:?}",
                        hwp_engine::model::text(p),
                        d.paras.get(p.a("paraPrIDRef")),
                        p.child("linesegarray")
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
        if a.get(2).is_none_or(|id| t.a("id") == id || id == "summary") {
            println!("TABLE {} {:?}", t.a("id"), t.child("sz").map(|n| &n.attrs));
            println!(
                "PROPERTIES {:?} POSITION {:?} OUTSIDE {:?} INSIDE {:?}",
                t.attrs,
                t.child("pos").map(|n| &n.attrs),
                t.child("outMargin").map(|n| &n.attrs),
                t.child("inMargin").map(|n| &n.attrs)
            );
            if a.get(2).is_some_and(|s| s == "summary") {
                for p in d
                    .sections
                    .iter()
                    .flat_map(|s| s.children_named("p"))
                    .filter(|p| {
                        p.children_named("run")
                            .flat_map(|r| &r.children)
                            .any(|c| c.name == "tbl" && c.a("id") == t.a("id"))
                    })
                {
                    println!(
                        "PARENT {:?} TEXT {:?} LINES {:?}",
                        p.attrs,
                        p.children_named("run")
                            .flat_map(|r| r.children_named("t"))
                            .map(hwp_engine::model::text)
                            .collect::<String>(),
                        p.child("linesegarray")
                    );
                }
                let mut notes = Vec::new();
                t.all("footNote", &mut notes);
                println!("FOOTNOTES {}", notes.len());
                let all_cells = t
                    .children_named("tr")
                    .flat_map(|r| r.children_named("tc"))
                    .collect::<Vec<_>>();
                println!(
                    "HEADER_CELLS {:?}",
                    all_cells
                        .iter()
                        .filter(|c| c.a("header") == "1")
                        .map(|c| (
                            c.child("cellAddr").unwrap().u("rowAddr"),
                            c.child("cellSpan").unwrap().u("rowSpan")
                        ))
                        .collect::<Vec<_>>()
                );
                for p in all_cells
                    .iter()
                    .filter_map(|c| c.child("subList"))
                    .flat_map(|l| l.children_named("p"))
                {
                    let mut controls = Vec::new();
                    p.all("ctrl", &mut controls);
                    let style = d.paras.get(p.a("paraPrIDRef"));
                    let keep = style
                        .and_then(|s| s.child("breakSetting"))
                        .is_some_and(|b| {
                            ["keepLines", "keepWithNext", "widowOrphan"]
                                .iter()
                                .any(|k| b.a(k) == "1")
                        });
                    let border = style
                        .and_then(|s| s.child("border"))
                        .and_then(|b| d.borders.get(b.a("borderFillIDRef")))
                        .is_some_and(|b| {
                            b.children
                                .iter()
                                .any(|s| s.name.ends_with("Border") && s.a("type") != "NONE")
                                || b.find("winBrush")
                                    .is_some_and(|f| f.a("faceColor") != "none")
                        });
                    if keep || border || !controls.is_empty() {
                        println!(
                            "SPECIAL_PARA {} keep={keep} border={border} controls={:?}",
                            p.a("id"),
                            controls
                                .iter()
                                .flat_map(|n| &n.children)
                                .map(|n| n.name.as_str())
                                .collect::<Vec<_>>()
                        );
                    }
                }
                println!(
                    "BODY_MERGES {:?}",
                    t.children_named("tr")
                        .flat_map(|r| r.children_named("tc"))
                        .filter(|c| c.a("header") != "1"
                            && c.child("cellSpan").is_some_and(|s| s.u("rowSpan") > 1))
                        .map(|c| (
                            c.child("cellAddr").unwrap().attrs.clone(),
                            c.child("cellSpan").unwrap().attrs.clone()
                        ))
                        .collect::<Vec<_>>()
                );
                continue;
            }
            dump(t)
        }
    }
    Ok(())
}
