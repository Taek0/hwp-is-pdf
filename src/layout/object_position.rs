//! Shared vertical reference semantics for saved and rebuilt object anchors.
pub(super) fn outside_inset(align: &str, leading: f64, trailing: f64) -> f64 {
    match align {
        "CENTER" => (leading - trailing) / 2.,
        "RIGHT" | "BOTTOM" => -trailing,
        _ => leading,
    }
}
pub(super) fn vertical(
    origin: f64,
    extent: f64,
    height: f64,
    reference: &str,
    align: &str,
    offset: f64,
) -> f64 {
    // Paragraph TOP/CENTER/BOTTOM all refer to the anchor line, not page height.
    if !matches!(reference, "PAGE" | "PAPER") {
        return origin + offset;
    }
    origin
        + match align {
            "CENTER" => (extent - height) / 2. + offset,
            "BOTTOM" => extent - height - offset,
            _ => offset,
        }
}
