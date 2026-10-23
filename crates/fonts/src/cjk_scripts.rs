//! Which manifest script tags `build.rs` counts as CJK when it decides what to embed whole. The
//! build script includes this file with `#[path]` and the crate's tests use it as a module, so
//! the two can't drift.

/// ISO 15924 codes of the Han, Japanese and Korean scripts.
const CJK_SCRIPTS: [&str; 5] = ["Hani", "Hans", "Hant", "Jpan", "Kore"];

/// Whether a craft-fonts manifest script tag is CJK. A tag can carry a region after the code
/// (craft-fonts tags Noto Sans HK UI `Hant-HK`), so only the part before `-` is compared.
pub fn is_cjk_script(tag: &str) -> bool {
    let code = tag.trim().split('-').next().unwrap_or_default();
    CJK_SCRIPTS.contains(&code)
}
