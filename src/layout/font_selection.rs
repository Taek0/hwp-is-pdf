//! Named installed faces first; explicit fallbacks for unavailable legacy families.
use super::*;
impl<'a> Engine<'a> {
    pub(super) fn document_character_font(
        &mut self,
        name: &str,
        script: &str,
        bold: bool,
        italic: bool,
        override_font: Option<&str>,
    ) -> Result<usize> {
        // The supplied native reference realizes missing HFT Junggothic
        // symbols with Batang, while its Hangul/Latin runs use Dotum.
        if script == "symbol"
            && name == "한양중고딕"
            && override_font.is_none()
            && self.fonts.family(name, bold, italic)?.is_none()
        {
            let id = self.document_font("바탕", bold, italic, None)?;
            self.report.warnings.insert(format!(
                "Font substitution: {name} symbols -> 바탕 ({})",
                self.fonts.list[id].name
            ));
            return Ok(id);
        }
        self.document_font(name, bold, italic, override_font)
    }
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
    fn cjk_brackets_select_symbol_faces_and_preserve_installed_halfwidth_note_fonts() {
        let mut d = Document::open(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("samples/tables-notes/issue1937_rowbreak_footnote_overpagination.hwp"),
        )
        .unwrap();
        let mut note = d.chars["12"].clone();
        node_mut(&mut note, "fontRef")
            .attrs
            .insert("symbol".into(), "2".into());
        note.attrs.insert("height".into(), "900".into());
        for v in node_mut(&mut note, "ratio").attrs.values_mut() {
            *v = "100".into();
        }
        for v in node_mut(&mut note, "spacing").attrs.values_mut() {
            *v = "0".into();
        }
        let mut missing = note.clone();
        node_mut(&mut missing, "fontRef")
            .attrs
            .insert("other".into(), "5".into());
        d.chars.insert("missing-dot".into(), missing);
        d.chars.insert("quoted-note".into(), note);
        let mut e = Engine::new(
            &d,
            Path::new(if cfg!(windows) {
                "C:/Windows/Fonts"
            } else {
                "/mnt/c/Windows/Fonts"
            }),
        );
        for c in ['「', '」'] {
            assert_eq!(text_metrics::language(c), ("symbol", "SYMBOL"));
            let g = e.glyph(c, "12").unwrap();
            if e.fonts
                .family("한양중고딕", false, false)
                .unwrap()
                .is_none()
            {
                assert_eq!(
                    g.font,
                    e.fonts.family("바탕", false, false).unwrap().unwrap()
                );
                assert!((g.width - g.tracking - 9.4).abs() < 0.01);
            }
            let n = e.glyph(c, "quoted-note").unwrap();
            assert_eq!(
                n.font,
                e.fonts.family("함초롬바탕", false, false).unwrap().unwrap()
            );
            assert!((n.width - 4.5).abs() < 0.01);
        }
        let dot = e.glyph('․', "missing-dot").unwrap();
        assert_eq!(
            dot.font,
            e.fonts.family("함초롬바탕", false, false).unwrap().unwrap()
        );
        assert_eq!(text_metrics::language('․'), ("other", "OTHER"));
        assert_eq!(text_metrics::language('あ'), ("japanese", "JAPANESE"));
        assert_eq!(text_metrics::language('漢'), ("hanja", "HANJA"));
    }
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
