use anyhow::{Context, Result, bail};
use std::{collections::BTreeMap, fs::File, io::Read, path::Path};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Node {
    pub name: String,
    pub attrs: BTreeMap<String, String>,
    pub children: Vec<Node>,
    pub text: String,
}
impl Node {
    pub fn a(&self, k: &str) -> &str {
        self.attrs.get(k).map(String::as_str).unwrap_or("")
    }
    pub fn n(&self, k: &str) -> f64 {
        self.a(k).parse::<f64>().unwrap_or(0.)
    }
    pub fn signed(&self, k: &str) -> f64 {
        let v = self.n(k);
        if v > 2147483647. { v - 4294967296. } else { v }
    }
    pub fn u(&self, k: &str) -> usize {
        self.n(k) as usize
    }
    pub fn child(&self, n: &str) -> Option<&Node> {
        self.children.iter().find(|c| c.name == n)
    }
    pub fn find(&self, n: &str) -> Option<&Node> {
        if self.name == n {
            Some(self)
        } else {
            self.children.iter().find_map(|c| c.find(n))
        }
    }
    pub fn all<'a>(&'a self, n: &str, out: &mut Vec<&'a Node>) {
        if self.name == n {
            out.push(self)
        }
        for c in &self.children {
            c.all(n, out)
        }
    }
    pub fn children_named<'a>(&'a self, n: &'a str) -> impl Iterator<Item = &'a Node> {
        self.children.iter().filter(move |c| c.name == n)
    }
}
pub fn xml(bytes: &[u8]) -> Result<Node> {
    let text = std::str::from_utf8(bytes)?;
    let doc = roxmltree::Document::parse(text)?;
    fn own(n: roxmltree::Node) -> Node {
        if n.is_text() {
            return Node {
                name: "#text".into(),
                text: n.text().unwrap_or("").into(),
                ..Default::default()
            };
        }
        Node {
            name: n.tag_name().name().into(),
            attrs: n
                .attributes()
                .map(|a| (a.name().into(), a.value().into()))
                .collect(),
            children: n
                .children()
                .filter(|c| c.is_element() || c.is_text())
                .map(own)
                .collect(),
            text: String::new(),
        }
    }
    Ok(own(doc.root_element()))
}
pub fn text(n: &Node) -> String {
    if n.name == "#text" {
        n.text.clone()
    } else {
        n.children.iter().map(text).collect()
    }
}

pub struct Document {
    pub header: Node,
    pub sections: Vec<Node>,
    pub masters: Vec<Node>,
    pub binaries: BTreeMap<String, Vec<u8>>,
    pub chars: BTreeMap<String, Node>,
    pub paras: BTreeMap<String, Node>,
    pub borders: BTreeMap<String, Node>,
    pub fonts: BTreeMap<(String, String), String>,
}
impl Document {
    pub fn open(path: &Path) -> Result<Self> {
        if path
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|s| s.eq_ignore_ascii_case("hwp"))
        {
            return crate::hwp::open(path);
        }
        if path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase()
            != "hwpx"
        {
            bail!("Supported input extensions: .hwp and .hwpx")
        }
        let mut zip = zip::ZipArchive::new(File::open(path)?)?;
        if zip.len() > 10000 {
            bail!("Excessive ZIP entries")
        }
        let mut files = BTreeMap::new();
        let mut total = 0u64;
        for i in 0..zip.len() {
            let mut f = zip.by_index(i)?;
            if f.is_dir() {
                continue;
            }
            total += f.size();
            if f.size() > 64 * 1024 * 1024 || total > 256 * 1024 * 1024 {
                bail!("HWPX decompression limit exceeded")
            }
            let name = f.name().to_owned();
            let mut b = Vec::new();
            (&mut f).take(64 * 1024 * 1024 + 1).read_to_end(&mut b)?;
            if b.len() > 64 * 1024 * 1024 {
                bail!("ZIP entry limit exceeded")
            }
            if files.insert(name, b).is_some() {
                bail!("Duplicate ZIP entry")
            }
        }
        let mut header = xml(files
            .get("Contents/header.xml")
            .context("Missing header.xml")?)?;
        let manifest = xml(files
            .get("Contents/content.hpf")
            .context("Missing content.hpf")?)?;
        let mut items = Vec::new();
        manifest.all("item", &mut items);
        let mut binaries = BTreeMap::new();
        for item in items {
            let href = item.a("href");
            if let Some(b) = files
                .get(href)
                .or_else(|| files.get(&format!("Contents/{href}")))
            {
                binaries.insert(item.a("id").to_string(), b.clone());
            }
        }
        let mut sections = Vec::new();
        let mut masters = Vec::new();
        let mut section_files: Vec<_> = files
            .iter()
            .filter(|(n, _)| n.starts_with("Contents/section") && n.ends_with(".xml"))
            .collect();
        section_files.sort_by_key(|(n, _)| {
            n.trim_start_matches("Contents/section")
                .trim_end_matches(".xml")
                .parse::<usize>()
                .unwrap_or(0)
        });
        for (_, b) in section_files {
            sections.push(xml(b)?)
        }
        for (n, b) in &files {
            if n.starts_with("Contents/masterpage") && n.ends_with(".xml") {
                masters.push(xml(b)?);
            }
        }
        if sections.is_empty() {
            bail!("No sections")
        }
        crate::image_effects::apply(&mut sections, &mut binaries)?;
        crate::image_effects::apply(&mut masters, &mut binaries)?;
        crate::image_effects::apply(std::slice::from_mut(&mut header), &mut binaries)?;
        let collect = |tag: &str| {
            let mut nodes = Vec::new();
            header.all(tag, &mut nodes);
            nodes
                .into_iter()
                .map(|n| (n.a("id").to_string(), n.clone()))
                .collect()
        };
        let chars = collect("charPr");
        let paras = collect("paraPr");
        let borders = collect("borderFill");
        let mut fonts = BTreeMap::new();
        let mut faces = Vec::new();
        header.all("fontface", &mut faces);
        for face in faces {
            for font in face.children_named("font") {
                fonts.insert(
                    (face.a("lang").to_string(), font.a("id").to_string()),
                    font.a("face").to_string(),
                );
            }
        }
        Ok(Self {
            header,
            sections,
            masters,
            binaries,
            chars,
            paras,
            borders,
            fonts,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unsigned_offsets_become_signed() {
        let n = xml(br#"<pos vertOffset="4294966449"/>"#).unwrap();
        assert_eq!(n.signed("vertOffset"), -847.);
    }
    #[test]
    fn preserves_mixed_text() {
        let n = xml("<t>가<tab/>나</t>".as_bytes()).unwrap();
        assert_eq!(text(&n), "가나");
        assert_eq!(n.children.len(), 3);
    }
}
