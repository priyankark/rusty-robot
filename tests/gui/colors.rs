/// Tagged macOS screenshots round-trip through the display profile and sRGB.
/// Allow the same <=3/channel quantization bound used by capture-quality checks.
/// This is only for comparing CSS literals to capture; crop/template comparisons
/// continue to compare captured pixels exactly.
pub fn matches_srgb(actual: &str, expected: &str) -> bool {
    if actual.len() != 6 || expected.len() != 6 {
        return false;
    }
    let tolerance = if cfg!(target_os = "macos") { 3 } else { 0 };
    (0..6).step_by(2).all(|i| {
        match (
            actual
                .get(i..i + 2)
                .and_then(|s| u8::from_str_radix(s, 16).ok()),
            expected
                .get(i..i + 2)
                .and_then(|s| u8::from_str_radix(s, 16).ok()),
        ) {
            (Some(a), Some(b)) => a.abs_diff(b) <= tolerance,
            _ => false,
        }
    })
}
