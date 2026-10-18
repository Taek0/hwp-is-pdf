//! ToUnicode fallbacks for shaped glyphs; ActualText remains the logical span.
use super::*;

pub(super) fn mappings(
    face: &ttf_parser::Face<'_>,
    text: &str,
    glyphs: &[(u16, usize)],
) -> BTreeMap<u16, String> {
    let mut nominal = BTreeMap::new();
    for c in text.chars() {
        if let Some(id) = crate::pdf::glyph_id(face, c) {
            nominal.entry(id.0).or_insert_with(|| c.to_string());
        }
    }
    cluster_mappings(text, glyphs, &nominal)
}

fn cluster_mappings(
    text: &str,
    glyphs: &[(u16, usize)],
    nominal: &BTreeMap<u16, String>,
) -> BTreeMap<u16, String> {
    let mut groups = BTreeMap::<usize, Vec<u16>>::new();
    for (id, cluster) in glyphs {
        if *cluster < text.len() && text.is_char_boundary(*cluster) {
            groups.entry(*cluster).or_default().push(*id);
        }
    }
    let starts: Vec<_> = groups.keys().copied().collect();
    let mut out = BTreeMap::new();
    for (index, start) in starts.iter().enumerate() {
        let end = starts.get(index + 1).copied().unwrap_or(text.len());
        let ids = &groups[start];
        if ids.len() == 1 {
            // One glyph may represent several source characters (ligatures or
            // composed marks). PDF ToUnicode supports a UTF-16 sequence.
            out.entry(ids[0])
                .or_insert_with(|| text[*start..end].to_owned());
        } else {
            // Do not repeat an entire cluster for every combining-mark glyph.
            // Nominal mappings are unambiguous; contextual multi-glyph clusters
            // without them retain the outer logical ActualText span.
            for id in ids {
                if let Some(value) = nominal.get(id) {
                    out.entry(*id).or_insert_with(|| value.clone());
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ligatures_and_reversed_visual_runs_keep_logical_cluster_text() {
        let map = cluster_mappings(
            "office",
            &[(1, 0), (2, 1), (3, 4), (4, 5)],
            &BTreeMap::new(),
        );
        assert_eq!(map[&2], "ffi");
        let map = cluster_mappings("اب", &[(11, 2), (12, 0)], &BTreeMap::new());
        assert_eq!(map[&11], "ب");
        assert_eq!(map[&12], "ا");
    }
    #[test]
    fn decomposed_marks_do_not_duplicate_the_cluster_and_bad_offsets_are_ignored() {
        let nominal = [(1, "a".into()), (2, "\u{301}".into())]
            .into_iter()
            .collect();
        let map = cluster_mappings("a\u{301}", &[(1, 0), (2, 0), (9, 2), (10, 99)], &nominal);
        assert_eq!(map.len(), 2);
        assert_eq!(map[&1], "a");
        assert_eq!(map[&2], "\u{301}");
    }
}
