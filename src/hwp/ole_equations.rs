//! HwpEq5x envelopes and their TAB-delimited prefix expressions.
use super::*;
pub(super) fn script(data: &[u8]) -> Result<Option<String>> {
    if !data.starts_with(b"Hwp 5.0 Equation Editor(HwpEq5x)") {
        return Ok(None);
    }
    ensure!(u32(data, 52)? == 5, "Unsupported HwpEq envelope version");
    let len = u32(data, 68)? as usize;
    ensure!(
        len > 0 && len <= 1_048_576 && len.is_multiple_of(2),
        "Invalid HwpEq script length"
    );
    let raw = bytes(data, 72, len)?;
    let units: Vec<_> = raw
        .as_chunks::<2>()
        .0
        .iter()
        .map(|v| u16::from_le_bytes(*v))
        .collect();
    let s = String::from_utf16(&units)?
        .trim_end_matches('\0')
        .to_string();
    ensure!(!s.contains('\0'), "Interior NUL in HwpEq script");
    let mut parser = Prefix { source: &s, at: 0 };
    let canonical = parser.sequence(false, 0)?;
    ensure!(
        canonical.len() <= 1_048_576,
        "Expanded HwpEq expression too large"
    );
    Ok(Some(canonical))
}
struct Prefix<'a> {
    source: &'a str,
    at: usize,
}
impl Prefix<'_> {
    fn sequence(&mut self, argument: bool, depth: usize) -> Result<String> {
        ensure!(depth < 64, "HwpEq prefix nesting limit");
        let mut out = String::new();
        while self.at < self.source.len() {
            let tail = &self.source[self.at..];
            let c = tail.chars().next().unwrap();
            if c == '"' {
                let end = tail[1..].find('"').context("Unterminated HwpEq literal")? + 2;
                out.push_str(&tail[..end]);
                self.at += end;
                continue;
            }
            if c != '\\' {
                out.push(c);
                self.at += c.len_utf8();
                continue;
            }
            self.at += 1;
            let n = self.source[self.at..]
                .chars()
                .take_while(|c| c.is_ascii_alphabetic())
                .count();
            ensure!(n > 0, "Invalid HwpEq command");
            let command = self.source[self.at..self.at + n].to_ascii_uppercase();
            self.at += n;
            match command.as_str() {
                "TAB" => {
                    ensure!(argument, "Unexpected HwpEq TAB");
                    return Ok(out.trim().into());
                }
                "OVER" => {
                    let a = self.sequence(true, depth + 1)?;
                    let b = self.sequence(true, depth + 1)?;
                    out.push_str(&format!("{{{{{a}}} over {{{b}}}}}"));
                }
                "SQRT" => {
                    let a = self.sequence(true, depth + 1)?;
                    out.push_str(&format!("sqrt {{{a}}}"));
                }
                "ROOT" => {
                    let a = self.sequence(true, depth + 1)?;
                    let b = self.sequence(true, depth + 1)?;
                    out.push_str(&format!("root {{{a}}} of {{{b}}}"));
                }
                "TIMES" | "CDOT" | "PM" | "LEQ" | "GEQ" | "NEQ" | "INF" | "RARROW" | "LARROW"
                | "LRARROW" => {
                    out.push(' ');
                    out.push_str(&command.to_ascii_lowercase());
                    out.push(' ');
                }
                _ => bail!("Unsupported HwpEq prefix command {command}"),
            }
        }
        ensure!(!argument, "Unterminated HwpEq argument");
        Ok(out)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nested_prefix_fractions_preserve_arguments_and_reject_truncation() {
        let mut p = Prefix {
            source: r"N = \OVER 1 \TAB \OVER 2 \TAB 3 \TAB \TAB",
            at: 0,
        };
        assert_eq!(
            p.sequence(false, 0).unwrap(),
            "N = {{1} over {{{2} over {3}}}}"
        );
        assert!(
            Prefix {
                source: r"\OVER 1 \TAB 2",
                at: 0
            }
            .sequence(false, 0)
            .is_err()
        );
        assert!(
            Prefix {
                source: r"\UNKNOWN x",
                at: 0
            }
            .sequence(false, 0)
            .is_err()
        );
    }
}
