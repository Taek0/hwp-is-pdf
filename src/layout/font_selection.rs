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
            // Supplied Hancom PDFs embed Batang for unavailable HCI Poppy
            // Latin runs; the sans-serif substitute changed line endings.
            "바탕"
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_legacy_latin_face_matches_reference_metrics_without_overriding_installed_faces() {
        let d = Document::open(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/reflow/contour-single.hwpx"),
        )
        .unwrap();
        let mut e = Engine::new(
            &d,
            Path::new(if cfg!(windows) {
                "C:/Windows/Fonts"
            } else {
                "/mnt/c/Windows/Fonts"
            }),
        );
        let installed = e.fonts.family("HCI Poppy", false, false).unwrap();
        let id = e.document_font("HCI Poppy", false, false, None).unwrap();
        if let Some(original) = installed {
            assert_eq!(id, original);
        } else {
            let batang = e.fonts.family("바탕", false, false).unwrap().unwrap();
            assert_eq!(id, batang);
            // Hancom prints 4.512 + 7.140 + 4.512pt at 12pt. Its
            // quantized widths differ from native outlines by about 0.03pt.
            assert!((e.fonts.width(id, "(2)", 12.) - 16.164).abs() < 0.04);
        }
        let requested = e.fonts.get("HANDotum.ttf").unwrap();
        assert_eq!(
            e.document_font("HCI Poppy", false, false, Some("HANDotum.ttf"))
                .unwrap(),
            requested
        );
    }
}
