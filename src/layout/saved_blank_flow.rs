//! Logical continuation of saved columns containing boundary blank paragraphs.
use super::*;
pub(super) struct BlankColumnFlow {
    previous: Option<(f64, f64)>,
    hidden: usize,
}
impl BlankColumnFlow {
    pub(super) fn new() -> Self {
        Self {
            previous: None,
            hidden: 2,
        }
    }
    pub(super) fn gap(&self, y: f64) -> f64 {
        self.previous
            .filter(|(old, _)| y > *old)
            .map(|(old, advance)| (y - old - advance).max(0.))
            .unwrap_or(0.)
    }
    pub(super) fn observe(&mut self, y: f64, advance: f64) {
        self.previous = Some((y, advance));
    }
    pub(super) fn break_column(&mut self, automatic: bool) {
        self.previous = None;
        self.hidden = if automatic { 2 } else { 0 };
    }
    pub(super) fn hide(&mut self, empty: bool) -> bool {
        if !empty {
            self.hidden = 0;
        }
        if empty && self.hidden > 0 {
            self.hidden -= 1;
            true
        } else {
            false
        }
    }
}
impl<'a> Engine<'a> {
    pub(super) fn recover_blank_columns(&self, section: &Node, cols: usize) -> bool {
        let Some(sec) = section.find("secPr") else {
            return false;
        };
        if cols < 2
            || sec.a("binaryHwp") == "1"
            || !sec
                .child("visibility")
                .is_some_and(|v| v.a("hideFirstEmptyLine") == "1")
            || section.find("footNote").is_some()
            || section_columns(section).is_some_and(|c| {
                !matches!(c.a("type"), "" | "NEWSPAPER")
                    || c.a("sameSz") == "0"
                    || c.a("layout") == "RIGHT"
            })
        {
            return false;
        }
        if section.children_named("p").any(|p| {
            lines(p)
                .iter()
                .any(|l| l.n("vertsize") <= 0. || l.n("vertsize") + l.n("spacing") <= 0.)
                || self
                    .doc
                    .paras
                    .get(p.a("paraPrIDRef"))
                    .and_then(|s| s.child("breakSetting"))
                    .is_some_and(|b| {
                        ["keepWithNext", "keepLines", "widowOrphan"]
                            .iter()
                            .any(|name| matches!(b.a(name), "1" | "true"))
                    })
        }) {
            return false;
        }
        let mut previous: Option<(f64, bool)> = None;
        for p in section.children_named("p") {
            let ls = lines(p);
            if ls.is_empty() {
                return false;
            }
            let empty = Self::items(p)
                .iter()
                .all(|i| matches!(&i.kind,ItemKind::Char(c) if c.is_whitespace()));
            for l in ls {
                let y = l.n("vertpos");
                if previous.is_some_and(|(old, blank)| blank && empty && y <= old) {
                    return true;
                }
                previous = Some((y, empty));
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hide_is_bounded_and_does_not_swallow_content_or_explicit_breaks() {
        let mut f = BlankColumnFlow::new();
        assert!(f.hide(true));
        assert!(f.hide(true));
        assert!(!f.hide(true));
        f.break_column(true);
        assert!(!f.hide(false));
        assert!(!f.hide(true));
        f.break_column(false);
        assert!(!f.hide(true));
        f.observe(20., 16.);
        assert_eq!(f.gap(40.), 4.);
        assert_eq!(f.gap(0.), 0.);
    }
}
