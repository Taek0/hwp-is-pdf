//! Normalize HWPX containers and retain their full child affine transforms.
use super::Node;
use crate::graphics::{IDENTITY, Matrix, compose};
use anyhow::{Result, ensure};
fn child_node<'a>(n: &'a mut Node, name: &str) -> &'a mut Node {
    let i = n
        .children
        .iter()
        .position(|c| c.name == name)
        .unwrap_or_else(|| {
            n.children.push(Node {
                name: name.into(),
                ..Default::default()
            });
            n.children.len() - 1
        });
    &mut n.children[i]
}
fn local_matrix(n: &Node) -> Result<Matrix> {
    let mut m = IDENTITY;
    let mut translated = false;
    if let Some(info) = n.child("renderingInfo") {
        for p in &info.children {
            if !matches!(p.name.as_str(), "transMatrix" | "scaMatrix" | "rotMatrix") {
                continue;
            }
            let next = [
                p.n("e1"),
                p.n("e4"),
                p.n("e2"),
                p.n("e5"),
                p.signed("e3") / 100.,
                p.signed("e6") / 100.,
            ];
            ensure!(next.iter().all(|v| v.is_finite()), "Invalid group matrix");
            translated |= p.name == "transMatrix";
            m = compose(m, next);
        }
    }
    if !translated && let Some(offset) = n.child("offset") {
        m = compose(
            [
                1.,
                0.,
                0.,
                1.,
                offset.signed("x") / 100.,
                offset.signed("y") / 100.,
            ],
            m,
        );
    }
    ensure!(m.iter().all(|v| v.is_finite()), "Group matrix overflow");
    Ok(m)
}
pub(super) fn normalize(n: &mut Node) -> Result<()> {
    for child in &mut n.children {
        normalize(child)?;
    }
    if n.name != "container" {
        return Ok(());
    }
    let w = n
        .child("orgSz")
        .map(|s| s.n("width"))
        .filter(|v| *v > 0.)
        .or_else(|| n.child("sz").map(|s| s.n("width")))
        .unwrap_or(0.);
    let h = n
        .child("orgSz")
        .map(|s| s.n("height"))
        .filter(|v| *v > 0.)
        .or_else(|| n.child("sz").map(|s| s.n("height")))
        .unwrap_or(0.);
    ensure!(
        w > 0. && h > 0. && w.is_finite() && h.is_finite(),
        "Invalid HWPX container extent"
    );
    n.name = "group".into();
    n.attrs.insert("hwpxContainer".into(), "1".into());
    n.attrs.insert("groupWidth".into(), w.to_string());
    n.attrs.insert("groupHeight".into(), h.to_string());
    for child in &mut n.children {
        if !matches!(
            child.name.as_str(),
            "pic"
                | "rect"
                | "line"
                | "ellipse"
                | "polygon"
                | "curve"
                | "arc"
                | "group"
                | "ole"
                | "equation"
                | "chart"
        ) {
            continue;
        }
        let matrix = local_matrix(child)?;
        let explicit_scale = child.child("renderingInfo").is_some_and(|n| {
            n.children
                .iter()
                .filter(|n| n.name == "scaMatrix")
                .any(|n| {
                    (n.n("e1") - 1.).abs() > 1e-9
                        || (n.n("e5") - 1.).abs() > 1e-9
                        || n.n("e2").abs() > 1e-9
                        || n.n("e4").abs() > 1e-9
                })
        });
        let original = child
            .child("orgSz")
            .or_else(|| child.child("sz"))
            .context("Group child size")?;
        let (ow, oh) = (original.n("width"), original.n("height"));
        let current = child.child("curSz");
        let cw = if !explicit_scale {
            current
                .map(|s| s.n("width"))
                .filter(|v| *v > 0.)
                .unwrap_or(ow)
        } else {
            ow
        };
        let ch = if !explicit_scale {
            current
                .map(|s| s.n("height"))
                .filter(|v| *v > 0.)
                .unwrap_or(oh)
        } else {
            oh
        };
        ensure!(
            cw >= 0. && ch >= 0. && cw.is_finite() && ch.is_finite(),
            "Invalid group child extent"
        );
        child
            .attrs
            .insert("groupMatrix".into(), serde_json::to_string(&matrix)?);
        child.attrs.insert("originalWidth".into(), ow.to_string());
        child.attrs.insert("originalHeight".into(), oh.to_string());
        for p in &mut child.children {
            match p.name.as_str() {
                "renderingInfo" => p.name = "sourceRenderingInfo".into(),
                "rotationInfo" => p.name = "sourceRotationInfo".into(),
                "flip" => p.name = "sourceFlip".into(),
                _ => {}
            }
        }
        for size in ["sz", "curSz"] {
            let p = child_node(child, size);
            p.attrs.insert("width".into(), cw.to_string());
            p.attrs.insert("height".into(), ch.to_string());
        }
        let pos = child_node(child, "pos");
        pos.attrs.clear();
        for (key, value) in [
            ("treatAsChar", "0"),
            ("horzRelTo", "PARA"),
            ("vertRelTo", "PARA"),
            ("horzAlign", "LEFT"),
            ("vertAlign", "TOP"),
            ("horzOffset", "0"),
            ("vertOffset", "0"),
        ] {
            pos.attrs.insert(key.into(), value.into());
        }
    }
    Ok(())
}
use anyhow::Context;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn child_affine_order_preserves_translation_and_does_not_scale_twice() {
        let mut root=crate::model::xml(br#"<container><orgSz width="10000" height="5000"/><sz width="20000" height="10000"/><pic><orgSz width="1000" height="2000"/><curSz width="500" height="1000"/><renderingInfo><transMatrix e1="1" e2="0" e3="900" e4="0" e5="1" e6="300"/><scaMatrix e1="0.5" e2="0" e3="100" e4="0" e5="0.5" e6="0"/></renderingInfo></pic></container>"#).unwrap();
        normalize(&mut root).unwrap();
        let child = root.child("pic").unwrap();
        let matrix: Matrix = serde_json::from_str(child.a("groupMatrix")).unwrap();
        assert_eq!(crate::graphics::point(matrix, 0., 0.), (10., 3.));
        assert_eq!(crate::graphics::point(matrix, 10., 20.), (15., 13.));
        assert_eq!(child.child("sz").unwrap().n("width"), 1000.);
        assert!(child.child("renderingInfo").is_none());
        let before = root.clone();
        normalize(&mut root).unwrap();
        assert_eq!(root, before);
    }
}
