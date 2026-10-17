//! Named installed faces first; explicit fallbacks for unavailable legacy families.
use super::*;
impl<'a> Engine<'a> {
    pub(super) fn document_font(
        &mut self,
        name: &str,
        bold: bool,
        italic: bool,
        override_font: Option<&str>,
    ) -> Result<usize> {
        if let Some(file) = override_font {
            return self.fonts.get(file);
        }
        if let Some(id) = self.fonts.family(name, bold, italic)? {
            return Ok(id);
        }
        let lower = name.to_ascii_lowercase();
        let bold = bold || lower.ends_with(" bold") || name.ends_with(" B");
        let fallback = if name == "휴먼명조" {
            "함초롬바탕"
        } else if name.contains("명조") || name.contains("바탕") {
            "바탕"
        } else if name.contains("고딕") || name.contains("돋움") {
            "돋움"
        } else if name.contains("굴림") {
            "굴림"
        } else if lower.contains("times") {
            "Times New Roman"
        } else if lower.contains("arial") || lower.contains("helvetica") {
            "Arial"
        } else if name == "HCI Poppy" {
            "함초롬돋움"
        } else {
            "함초롬바탕"
        };
        if let Some(id) = self.fonts.family(fallback, bold, italic)? {
            self.report.warnings.insert(format!(
                "Font substitution: {name} -> {fallback} ({})",
                self.fonts.list[id].name
            ));
            return Ok(id);
        }
        let file = if bold { "malgunbd.ttf" } else { "malgun.ttf" };
        self.report
            .warnings
            .insert(format!("Font substitution: {name} -> {file}"));
        self.fonts.get(file)
    }
}
