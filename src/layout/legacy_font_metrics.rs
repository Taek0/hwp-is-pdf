//! Advance-only compatibility for a missing classic Haansoft Batang face.
//! These 1024-em ASCII advances were checked against the local viewer's
//! font-metric cache and its supplied PDF. No font outlines are included.
//! HCR Batang is a visual substitute, not a metrically identical family.
pub(super) fn advance(family: &str, c: char, size: f64) -> Option<f64> {
    if !matches!(family, "한컴바탕" | "Haansoft Batang") || !(' '..='~').contains(&c) {
        return None;
    }
    const WIDTHS: [u16; 95] = [
        341, 426, 426, 853, 640, 938, 853, 256, 512, 512, 512, 853, 298, 853, 298, 341, 597, 597,
        597, 597, 597, 597, 597, 597, 597, 597, 341, 341, 853, 853, 853, 512, 1024, 768, 682, 682,
        725, 682, 640, 725, 768, 384, 469, 768, 640, 938, 768, 725, 640, 725, 682, 640, 768, 768,
        725, 981, 682, 682, 640, 512, 341, 512, 1024, 512, 597, 512, 554, 512, 554, 554, 384, 554,
        554, 298, 298, 554, 298, 853, 554, 554, 554, 554, 426, 512, 384, 554, 554, 810, 597, 597,
        469, 597, 597, 597, 810,
    ];
    Some(f64::from(WIDTHS[c as usize - 32]) * size / 1024.)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn classic_family_is_distinct_from_hcr_and_non_ascii_is_not_guessed() {
        assert_eq!(advance("한컴바탕", 'Y', 12.), Some(7.9921875));
        assert_eq!(advance("Haansoft Batang", ' ', 12.), Some(3.99609375));
        assert_eq!(advance("함초롬바탕", 'Y', 12.), None);
        assert_eq!(advance("한컴바탕", '가', 12.), None);
    }
}
