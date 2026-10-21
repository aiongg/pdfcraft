// The faces AGENTS.md §1.1 bars whatever their licence. One rule for both tables that embed
// craft-fonts faces: `crates/fonts/build.rs` (the document table) and `crates/ui-egui/build.rs`
// (the interface cuts) `include!` this file, and the fonts crate's tests check the result.

/// Family-name prefixes of Adobe's type designs.
const ADOBE_FAMILIES: [&str; 3] = ["Source Han", "Source Serif", "Source Sans"];

/// Script tags of CJK faces, with their region forms.
const CJK_SCRIPTS: [&str; 6] = ["Hans", "Hant", "Hant-HK", "Jpan", "Kore", "Hani"];

/// Whether AGENTS.md §1.1 bars a face: Adobe's Source families, and every Noto face for a CJK
/// script, which is Source Han under another name (Noto Sans CJK and its per-region cuts such as
/// Noto Sans SC UI).
pub(crate) fn barred(family: &str, scripts: &[&str]) -> bool {
    ADOBE_FAMILIES.iter().any(|adobe| family.starts_with(adobe)) || (family.starts_with("Noto ") && scripts.iter().any(|s| CJK_SCRIPTS.contains(s)))
}
