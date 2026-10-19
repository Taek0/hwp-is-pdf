//! A final explicit numerator group does not absorb preceding operators.
pub(super) fn numerator(left: &str) -> (&str, &str) {
    let s = left.trim();
    if !s.ends_with('}') {
        return ("", s);
    }
    let (mut depth, mut quote, mut start) = (0usize, false, None);
    for (i, c) in s.char_indices() {
        if c == '"' {
            quote = !quote;
            continue;
        }
        if quote {
            continue;
        }
        if c == '{' {
            if depth == 0 {
                start = Some(i);
            }
            depth += 1;
        } else if c == '}' {
            if depth == 0 {
                return ("", s);
            }
            depth -= 1;
        }
    }
    let Some(start) = start.filter(|_| depth == 0 && !quote) else {
        return ("", s);
    };
    let prefix = s[..start].trim_end();
    // These groups belong to a unary construction, rather than standing alone.
    let last = prefix
        .split_whitespace()
        .last()
        .unwrap_or("")
        .to_ascii_lowercase();
    if prefix.ends_with(['_', '^'])
        || [
            "sqrt", "root", "bar", "hat", "vec", "tilde", "dot", "ddot", "bold", "rm", "it",
            "matrix", "cases", "pile", "binom",
        ]
        .contains(&last.as_str())
    {
        return ("", s);
    }
    (prefix, &s[start..])
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn numerator_group_keeps_its_prefix_and_unary_owners_separate() {
        assert_eq!(
            numerator("lim _{h -> 0} {f(h)}"),
            ("lim _{h -> 0}", "{f(h)}")
        );
        assert_eq!(numerator("y= {a+b}"), ("y=", "{a+b}"));
        assert_eq!(numerator("sqrt {x}"), ("", "sqrt {x}"));
        assert_eq!(numerator("x^{2}"), ("", "x^{2}"));
    }
}
