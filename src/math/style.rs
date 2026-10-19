//! Lexically scoped equation font style; variables are italic by default.
use crate::pdf::Fonts;
use anyhow::Result;
#[derive(Clone, Copy, Default)]
pub(crate) struct Style {
    pub italic: Option<bool>,
    pub bold: bool,
    pub color: [f64; 3],
}
impl Style {
    pub fn consume<'a>(&mut self, s: &'a str) -> Option<&'a str> {
        if s.get(..5).is_some_and(|v| v.eq_ignore_ascii_case("color"))
            && s[5..].trim_start().starts_with('{')
        {
            let (rgb, rest) = super::group(&s[5..]);
            let values: Vec<_> = rgb.split(',').map(|n| n.trim().parse::<u8>()).collect();
            if values.len() == 3 && values.iter().all(Result::is_ok) {
                self.color = std::array::from_fn(|i| *values[i].as_ref().unwrap() as f64 / 255.);
                return Some(rest);
            }
        }
        for key in ["bold", "rm", "it"] {
            if s.get(..key.len())
                .is_some_and(|p| p.eq_ignore_ascii_case(key))
                && (key != "it"
                    || s[key.len()..]
                        .chars()
                        .next()
                        .is_none_or(|c| c.is_whitespace() || c == '{'))
            {
                match key {
                    "bold" => self.bold = true,
                    "rm" => self.italic = Some(false),
                    _ => self.italic = Some(true),
                }
                return Some(&s[key.len()..]);
            }
        }
        None
    }
    pub fn font(self, fonts: &mut Fonts, c: char) -> Result<usize> {
        let italic = self
            .italic
            .unwrap_or(c.is_ascii_alphabetic() || matches!(c as u32, 0x3b1..=0x3c9));
        fonts.get(match (self.bold, italic) {
            (true, true) => "timesbi.ttf",
            (true, false) => "timesbd.ttf",
            (false, true) => "timesi.ttf",
            _ => "times.ttf",
        })
    }
}
