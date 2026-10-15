//! OWPML number-format glyph sequences, independent of layout and counter state.
fn alphabet(mut n: usize, chars: &str) -> Option<String> {
    if n == 0 {
        return None;
    }
    let letters: Vec<_> = chars.chars().collect();
    let mut out = Vec::new();
    while n > 0 {
        n -= 1;
        out.push(letters[n % letters.len()]);
        n /= letters.len();
    }
    Some(out.into_iter().rev().collect())
}
fn cjk(n: usize, korean: bool) -> Option<String> {
    if n > 9999 {
        return None;
    }
    if n == 0 {
        return Some(if korean { "영" } else { "零" }.into());
    }
    let digits: Vec<_> = if korean {
        "영일이삼사오육칠팔구"
    } else {
        "零一二三四五六七八九"
    }
    .chars()
    .collect();
    let units: Vec<_> = if korean { "천백십" } else { "千百十" }.chars().collect();
    let mut out = String::new();
    let mut zero = false;
    for (i, divisor) in [1000, 100, 10, 1].iter().enumerate() {
        let d = n / divisor % 10;
        if d == 0 {
            if !out.is_empty() {
                zero = true;
            }
            continue;
        }
        if zero && !korean {
            out.push('零');
        }
        zero = false;
        if *divisor == 1 || d > 1 || (!korean && (*divisor != 10 || !out.is_empty())) {
            out.push(digits[d]);
        }
        if i < 3 {
            out.push(units[i]);
        }
    }
    Some(out)
}
pub(crate) fn format(n: usize, kind: &str) -> Option<String> {
    let unicode = |base: u32| char::from_u32(base + n as u32 - 1).map(|c| c.to_string());
    match kind {
        "" | "DIGIT" | "ARABIC" => Some(n.to_string()),
        "ROMAN_CAPITAL" | "ROMAN_SMALL" => {
            if !(1..=3999).contains(&n) {
                return None;
            }
            let mut value = n;
            let mut out = String::new();
            for (v, s) in [
                (1000, "M"),
                (900, "CM"),
                (500, "D"),
                (400, "CD"),
                (100, "C"),
                (90, "XC"),
                (50, "L"),
                (40, "XL"),
                (10, "X"),
                (9, "IX"),
                (5, "V"),
                (4, "IV"),
                (1, "I"),
            ] {
                while value >= v {
                    out.push_str(s);
                    value -= v;
                }
            }
            Some(if kind == "ROMAN_SMALL" {
                out.to_lowercase()
            } else {
                out
            })
        }
        "LATIN_CAPITAL" => alphabet(n, "ABCDEFGHIJKLMNOPQRSTUVWXYZ"),
        "LATIN_SMALL" => alphabet(n, "abcdefghijklmnopqrstuvwxyz"),
        "CIRCLED_DIGIT" if (1..=20).contains(&n) => unicode(0x2460),
        "CIRCLED_DIGIT" if (21..=35).contains(&n) => {
            char::from_u32(0x3251 + n as u32 - 21).map(|c| c.to_string())
        }
        "CIRCLED_DIGIT" if (36..=50).contains(&n) => {
            char::from_u32(0x32b1 + n as u32 - 36).map(|c| c.to_string())
        }
        "CIRCLED_LATIN_CAPITAL" if (1..=26).contains(&n) => unicode(0x24b6),
        "CIRCLED_LATIN_SMALL" if (1..=26).contains(&n) => unicode(0x24d0),
        "HANGUL_SYLLABLE" if (1..=14).contains(&n) => alphabet(n, "가나다라마바사아자차카타파하"),
        "HANGUL_JAMO" if (1..=14).contains(&n) => alphabet(n, "ㄱㄴㄷㄹㅁㅂㅅㅇㅈㅊㅋㅌㅍㅎ"),
        "CIRCLED_HANGUL_SYLLABLE" if (1..=14).contains(&n) => unicode(0x326e),
        "CIRCLED_HANGUL_JAMO" if (1..=14).contains(&n) => unicode(0x3260),
        "HANGUL_PHONETIC" => cjk(n, true),
        "IDEOGRAPH" => cjk(n, false),
        "CIRCLED_IDEOGRAPH" if (1..=10).contains(&n) => unicode(0x3280),
        _ => None,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn numbering_boundaries() {
        for (n, k, s) in [
            (27, "LATIN_CAPITAL", "AA"),
            (49, "ROMAN_SMALL", "xlix"),
            (21, "CIRCLED_DIGIT", "㉑"),
            (36, "CIRCLED_DIGIT", "㊱"),
            (14, "HANGUL_SYLLABLE", "하"),
            (3, "HANGUL_JAMO", "ㄷ"),
            (23, "HANGUL_PHONETIC", "이십삼"),
            (123, "IDEOGRAPH", "一百二十三"),
        ] {
            assert_eq!(format(n, k).as_deref(), Some(s));
        }
        assert!(format(51, "CIRCLED_DIGIT").is_none());
    }
}
