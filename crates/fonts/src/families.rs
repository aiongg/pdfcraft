// The craft-fonts families PdfCraft embeds whole, named as the manifest spells them. Both build
// scripts `include!` this file: `crates/fonts/build.rs` embeds the document families,
// `crates/ui-egui/build.rs` the interface ones, and each tells craft-fonts-build the other's are
// embedded elsewhere. The interface's CJK faces are cuts, not listed here.

/// The families documents need: Japanese text written into PDFs and the renderer's Japanese
/// substitutes (Shippori Mincho, BIZ UDMincho, BIZ UDPGothic), and Arabic and Thai text written
/// into PDFs. The interface draws Arabic and Thai with them too. Noto Sans Hebrew and a Telugu face
/// belong here once craft-fonts has them.
const DOCUMENT_FAMILIES: &[&str] = &["Shippori Mincho", "BIZ UDMincho", "BIZ UDPGothic", "Noto Sans Arabic", "Noto Sans Thai"];

/// The families only the interface draws with: scripts its catalogs, file names and document
/// titles use beyond the document families, symbols and emoji.
const INTERFACE_FAMILIES: &[&str] = &[
    "Noto Sans",
    "Noto Sans Georgian",
    "Noto Sans Armenian",
    "Noto Sans Ethiopic",
    "Noto Sans Symbols",
    "Noto Sans Symbols 2",
    "Noto Sans Math",
    "Noto Color Emoji",
    "Noto Emoji",
];
