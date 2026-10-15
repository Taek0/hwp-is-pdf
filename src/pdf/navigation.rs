//! PDF link annotations, named destinations and hierarchical document outlines.
use super::*;
use crate::graphics::{self, Matrix};
pub(super) struct Navigation {
    pub annotations: Vec<Vec<usize>>,
    pub names: Option<usize>,
    pub outlines: Option<usize>,
}
struct Destination {
    name: String,
    title: String,
    level: usize,
    page: usize,
    x: f64,
    y: f64,
}
struct Link {
    target: String,
    rect: [f64; 4],
}
fn hex(s: &str) -> String {
    format!(
        "<FEFF{}>",
        s.encode_utf16()
            .map(|c| format!("{c:04X}"))
            .collect::<String>()
    )
}
pub(crate) fn supported_uri(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();
    ["https://", "http://", "mailto:"]
        .iter()
        .any(|p| lower.starts_with(p) && s.len() > p.len())
}
fn uri_string(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'\\' | b'(' | b')' => {
                out.push('\\');
                out.push(b as char);
            }
            0..=32 | 127..=255 => out.push_str(&format!("%{b:02X}")),
            _ => out.push(b as char),
        }
    }
    format!("({out})")
}
pub(super) fn install(w: &mut Writer, pages: &[Page], page_ids: &[usize]) -> Navigation {
    let mut destinations = BTreeMap::new();
    let mut outline_order = Vec::new();
    let mut page_links = Vec::new();
    for (page_index, page) in pages.iter().enumerate() {
        let mut matrix = graphics::IDENTITY;
        let mut stack: Vec<Matrix> = Vec::new();
        let mut links: Vec<Link> = Vec::new();
        for op in &page.ops {
            match op {
                Op::TransformBegin { matrix: m } => {
                    stack.push(matrix);
                    matrix = graphics::compose(matrix, *m);
                }
                Op::TransformEnd => matrix = stack.pop().unwrap_or(graphics::IDENTITY),
                Op::Destination {
                    name,
                    title,
                    level,
                    x,
                    y,
                    ..
                } if !name.is_empty() => {
                    let (x, y) = graphics::point(matrix, *x, *y);
                    if x.is_finite() && y.is_finite() && !destinations.contains_key(name) {
                        if !title.is_empty() {
                            outline_order.push(name.clone());
                        }
                        destinations.insert(
                            name.clone(),
                            Destination {
                                name: name.clone(),
                                title: title.clone(),
                                level: *level,
                                page: page_index,
                                x: x.clamp(0., page.width),
                                y: y.clamp(0., page.height),
                            },
                        );
                    }
                }
                Op::Link { target, rect, .. } => {
                    let mut r = graphics::rect(matrix, *rect);
                    for i in [0, 2] {
                        r[i] = r[i].clamp(0., page.width);
                    }
                    for i in [1, 3] {
                        r[i] = r[i].clamp(0., page.height);
                    }
                    if r.iter().all(|v| v.is_finite()) && r[2] - r[0] > 0.01 && r[3] - r[1] > 0.01 {
                        if let Some(last) = links.last_mut().filter(|l| {
                            l.target == *target
                                && (l.rect[1] - r[1]).abs() < 0.5
                                && (l.rect[3] - r[3]).abs() < 0.5
                                && r[0] >= l.rect[0] - 0.5
                                && r[0] <= l.rect[2] + 0.6
                        }) {
                            last.rect[2] = last.rect[2].max(r[2]);
                        } else {
                            links.push(Link {
                                target: target.clone(),
                                rect: r,
                            });
                        }
                    }
                }
                _ => {}
            }
        }
        page_links.push(links);
    }
    let mut annotations = Vec::new();
    for (page, links) in pages.iter().zip(page_links) {
        let mut ids = Vec::new();
        for link in links {
            let action = if supported_uri(&link.target) {
                format!("/A << /S /URI /URI {} >>", uri_string(&link.target))
            } else if destinations.contains_key(link.target.trim_start_matches('#')) {
                format!("/Dest {}", hex(link.target.trim_start_matches('#')))
            } else {
                continue;
            };
            let r = link.rect;
            ids.push(w.add(format!("<< /Type /Annot /Subtype /Link /Rect [{:0.4} {:0.4} {:0.4} {:0.4}] /Border [0 0 0] {action} >>",r[0],page.height-r[3],r[2],page.height-r[1])));
        }
        annotations.push(ids);
    }
    let names = if destinations.is_empty() {
        None
    } else {
        let mut entries: Vec<_> = destinations
            .values()
            .map(|d| {
                (
                    hex(&d.name),
                    format!(
                        "[{} 0 R /XYZ {:0.4} {:0.4} null]",
                        page_ids[d.page],
                        d.x,
                        pages[d.page].height - d.y
                    ),
                )
            })
            .collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        Some(w.add(format!(
                "<< /Dests << /Names [{}] >> >>",
                entries
                    .iter()
                    .map(|(key, value)| format!("{key} {value} "))
                    .collect::<String>()
            )))
    };
    let outlines = if outline_order.is_empty() {
        None
    } else {
        let root = w.add(Vec::new());
        let ids: Vec<_> = outline_order.iter().map(|_| w.add(Vec::new())).collect();
        let mut parents: Vec<Option<usize>> = Vec::new();
        let mut stack: Vec<usize> = Vec::new();
        for (i, name) in outline_order.iter().enumerate() {
            let level = destinations[name].level;
            if level == usize::MAX {
                parents.push(None);
                continue;
            }
            while stack
                .last()
                .is_some_and(|j| destinations[&outline_order[*j]].level >= level)
            {
                stack.pop();
            }
            parents.push(stack.last().copied());
            stack.push(i);
        }
        let children = |parent: Option<usize>| {
            parents
                .iter()
                .enumerate()
                .filter_map(|(i, p)| (*p == parent).then_some(i))
                .collect::<Vec<_>>()
        };
        let roots = children(None);
        w.objects[root - 1] = format!(
            "<< /Type /Outlines /First {} 0 R /Last {} 0 R /Count {} >>",
            ids[roots[0]],
            ids[*roots.last().unwrap()],
            ids.len()
        )
        .into_bytes();
        for (i, name) in outline_order.iter().enumerate() {
            let d = &destinations[name];
            let siblings = children(parents[i]);
            let at = siblings.iter().position(|j| *j == i).unwrap();
            let mut s = format!(
                "<< /Title {} /Parent {} 0 R /Dest {}",
                hex(&d.title),
                parents[i].map(|p| ids[p]).unwrap_or(root),
                hex(name)
            );
            if at > 0 {
                s += &format!(" /Prev {} 0 R", ids[siblings[at - 1]]);
            }
            if at + 1 < siblings.len() {
                s += &format!(" /Next {} 0 R", ids[siblings[at + 1]]);
            }
            let child = children(Some(i));
            if !child.is_empty() {
                let descendants = (0..ids.len())
                    .filter(|j| {
                        let mut p = parents[*j];
                        while let Some(k) = p {
                            if k == i {
                                return true;
                            }
                            p = parents[k];
                        }
                        false
                    })
                    .count();
                s += &format!(
                    " /First {} 0 R /Last {} 0 R /Count {descendants}",
                    ids[child[0]],
                    ids[*child.last().unwrap()]
                );
            }
            s += " >>";
            w.objects[ids[i] - 1] = s.into_bytes();
        }
        Some(root)
    };
    Navigation {
        annotations,
        names,
        outlines,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn annotations_follow_transforms_and_discard_missing_destinations() {
        let page = Page {
            width: 200.,
            height: 300.,
            ops: vec![
                Op::TransformBegin {
                    matrix: [1., 0., 0., 1., 20., 30.],
                },
                Op::Destination {
                    name: "target".into(),
                    title: "제목".into(),
                    level: 0,
                    x: 10.,
                    y: 40.,
                    anchor_y: 40.,
                    page_base: 1,
                },
                Op::Link {
                    target: "#target".into(),
                    rect: [10., 40., 50., 60.],
                    anchor_y: 50.,
                },
                Op::TransformEnd,
                Op::Link {
                    target: "missing".into(),
                    rect: [0., 0., 10., 10.],
                    anchor_y: 5.,
                },
                Op::Link {
                    target: "javascript:alert(1)".into(),
                    rect: [0., 0., 10., 10.],
                    anchor_y: 5.,
                },
            ],
        };
        let mut w = Writer {
            objects: vec![Vec::new()],
        };
        let nav = install(&mut w, &[page], &[1]);
        assert_eq!(nav.annotations[0].len(), 1);
        let annot = String::from_utf8_lossy(&w.objects[nav.annotations[0][0] - 1]);
        assert!(
            annot.contains("/Rect [30.0000 210.0000 70.0000 230.0000]"),
            "{annot}"
        );
        let names = String::from_utf8_lossy(&w.objects[nav.names.unwrap() - 1]);
        assert!(names.contains("[1 0 R /XYZ 30.0000 230.0000 null]"));
        assert!(nav.outlines.is_some());
    }
}
