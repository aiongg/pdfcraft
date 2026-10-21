//! Embeds the document fonts of the optional craft-fonts build input (`CRAFT_FONTS_DIR`) as
//! `CRAFT_FONTS`: the document families `src/families.rs` names, whole, through craft-fonts-build.
//! Unset, `CRAFT_FONTS` is empty. It only reads the local checkout (no network).
//! `CRAFT_FONTS_REQUIRED=1` (release builds) turns a missing or unusable checkout, or a family the
//! manifest lacks, into a build error. These faces draw text written into PDFs, the renderer's
//! Japanese substitutes and the interface's Arabic and Thai text; the interface's own faces (the
//! CJK cuts, the other scripts, symbols and emoji) are embedded by `crates/ui-egui/build.rs`. On
//! wasm32 it embeds only BIZ UDPGothic Regular and any `Arab`, `Telu`, `Hebr` or `Thai` face, to
//! keep the web build within hosting limits (Cloudflare Pages: 25 MiB per file).

include!("src/families.rs");

fn main() {
    let wasm = craft_fonts_build::target_is_wasm();
    let written = craft_fonts_build::Embed::documents(DOCUMENT_FAMILIES)
        .filter(move |face| {
            // The web build keeps the small faces of the scripts it writes and its interface draws.
            let web_face =
                (face.family == "BIZ UDPGothic" && face.style == "Regular") || ["Arab", "Telu", "Hebr", "Thai"].iter().any(|s| face.has_script(s));
            !wasm || web_face
        })
        .embedded_elsewhere(INTERFACE_FAMILIES)
        .write("craft_fonts.rs");
    if let Err(e) = written {
        println!("cargo::error={e}");
    }
}
