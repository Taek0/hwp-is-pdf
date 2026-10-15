//! Resolve document family names against the explicitly configured font directory.
use super::*;
use std::sync::{Arc, Mutex, OnceLock};
#[derive(Clone)]
struct Face {
    file: String,
    index: u32,
    weight: u16,
    width: u16,
    italic: bool,
}
type Catalog = BTreeMap<String, Vec<Face>>;
type Cache = Mutex<BTreeMap<PathBuf, Arc<Catalog>>>;
static CATALOGS: OnceLock<Cache> = OnceLock::new();
fn key(name: &str) -> String {
    name.chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}
fn catalog(root: &Path) -> Result<Arc<Catalog>> {
    let mut caches = CATALOGS
        .get_or_init(|| Mutex::new(BTreeMap::new()))
        .lock()
        .map_err(|_| anyhow::anyhow!("Font catalog lock poisoned"))?;
    if let Some(c) = caches.get(root) {
        return Ok(c.clone());
    }
    let mut paths: Vec<_> = std::fs::read_dir(root)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .collect();
    paths.sort();
    let mut out = Catalog::new();
    for path in paths {
        if !path.extension().is_some_and(|e| {
            matches!(
                e.to_string_lossy().to_ascii_lowercase().as_str(),
                "ttf" | "ttc"
            )
        }) {
            continue;
        }
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        for index in 0..ttf_parser::fonts_in_collection(&bytes)
            .unwrap_or(1)
            .min(256)
        {
            let Ok(face) = ttf_parser::Face::parse(&bytes, index) else {
                continue;
            };
            if face.tables().glyf.is_none() {
                continue;
            }
            let item = Face {
                file: path.file_name().unwrap().to_string_lossy().into(),
                index,
                weight: face.weight().to_number(),
                width: face.width().to_number(),
                italic: face.is_italic(),
            };
            let names: std::collections::BTreeSet<_> = face
                .names()
                .into_iter()
                .filter(|n| matches!(n.name_id, 1 | 4 | 6 | 16))
                .filter_map(|n| n.to_string())
                .map(|s| key(&s))
                .collect();
            for name in names {
                out.entry(name).or_default().push(item.clone());
            }
        }
    }
    let out = Arc::new(out);
    caches.insert(root.into(), out.clone());
    Ok(out)
}
impl Fonts {
    pub fn family(&mut self, name: &str, bold: bool, italic: bool) -> Result<Option<usize>> {
        let catalog = catalog(&self.root)?;
        let Some(faces) = catalog.get(&key(name)) else {
            return Ok(None);
        };
        let target = if bold { 700 } else { 400 };
        let face = faces
            .iter()
            .min_by_key(|f| {
                (
                    u32::from(f.italic != italic) * 1000 + f.weight.abs_diff(target) as u32,
                    u32::from(f.width.abs_diff(5)),
                    f.index,
                )
            })
            .unwrap();
        let name = if face.index == 0 {
            face.file.clone()
        } else {
            format!("{}#{}", face.file, face.index)
        };
        let id = self.get(&name)?;
        if bold && face.weight < 600 {
            Ok(Some(self.embolden(id)))
        } else {
            Ok(Some(id))
        }
    }
    fn embolden(&mut self, id: usize) -> usize {
        let name = format!("{}:synthetic-bold", self.list[id].name);
        if let Some(i) = self.list.iter().position(|f| f.name == name) {
            return i;
        }
        let bytes = self.list[id].bytes.clone();
        let i = self.list.len();
        self.list.push(Font {
            bytes,
            name,
            used: BTreeMap::new(),
            synthetic_bold: true,
        });
        i
    }
    pub fn ink_padding(&self, id: usize, size: f64) -> f64 {
        if self.list[id].synthetic_bold {
            size * 0.0125
        } else {
            0.
        }
    }
}
