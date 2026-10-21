//! Embeds the fonts of the optional craft-fonts build input (`CRAFT_FONTS_DIR`) as `CRAFT_FONTS`.
//! The recipe is craft-fonts' `docs/integration.md`; unset, `CRAFT_FONTS` is empty. It only reads
//! the local checkout (no network). `CRAFT_FONTS_REQUIRED=1` (release builds) turns a missing or
//! unusable checkout into a build error. The faces are whole: they draw text written into PDFs and
//! the interface's non-CJK scripts. The interface's CJK faces are cuts that `crates/ui-egui/build.rs`
//! embeds. On wasm32 it embeds only BIZ UDPGothic Regular and any `Arab`, `Telu`, `Hebr` or `Thai`
//! face, to keep the web build within hosting limits (Cloudflare Pages: 25 MiB per file).

use std::fmt::Write as _;
use std::path::PathBuf;

fn main() {
    println!("cargo::rerun-if-env-changed=CRAFT_FONTS_DIR");
    println!("cargo::rerun-if-env-changed=CRAFT_FONTS_REQUIRED");
    let required = std::env::var_os("CRAFT_FONTS_REQUIRED").is_some_and(|v| v == "1");
    let mut src = String::from("pub static CRAFT_FONTS: &[CraftFont] = &[\n");
    match std::env::var_os("CRAFT_FONTS_DIR").filter(|d| !d.is_empty()).map(PathBuf::from) {
        Some(dir) => match craft_fonts(&dir) {
            Ok(entries) => src.push_str(&entries),
            Err(e) if required => println!("cargo::error=CRAFT_FONTS_DIR={}: {e}", dir.display()),
            Err(e) => println!("cargo::warning=building without craft-fonts: CRAFT_FONTS_DIR={}: {e}", dir.display()),
        },
        None if required => {
            println!("cargo::error=CRAFT_FONTS_REQUIRED=1 but CRAFT_FONTS_DIR is not set: point it at a craft-fonts checkout (absolute path)");
        }
        None => {}
    }
    src.push_str("];\n");
    let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap_or_default()).join("craft_fonts.rs");
    if let Err(e) = std::fs::write(&out, src) {
        println!("cargo::error=writing {}: {e}", out.display());
    }
}

include!("src/barred.rs");

/// One `CraftFont { .. }` initialiser per manifest line.
fn craft_fonts(dir: &std::path::Path) -> Result<String, String> {
    let manifest = dir.join("fonts/manifest.txt");
    println!("cargo::rerun-if-changed={}", manifest.display());
    let text = std::fs::read_to_string(&manifest).map_err(|e| format!("{}: {e}", manifest.display()))?;
    let wasm = std::env::var("CARGO_CFG_TARGET_ARCH").is_ok_and(|a| a == "wasm32");
    let mut out = String::new();
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
        let f: Vec<&str> = line.split(" | ").map(str::trim).collect();
        let [family, style, file, scripts, ..] = f.as_slice() else {
            return Err(format!("malformed manifest line: {line}"));
        };
        // A checkout that carries a barred face (craft-fonts has Noto Sans CJK and the Noto Sans
        // SC/TC/HK/KR UI cuts of it) never embeds it.
        if barred(family, &scripts.split(',').map(str::trim).collect::<Vec<_>>()) {
            println!("cargo::warning=craft-fonts: not embedding {family} {style} (AGENTS.md §1.1)");
            continue;
        }
        // Arabic interface faces are small, so the web build keeps them too.
        let arabic = scripts.split(',').any(|s| s.trim() == "Arab");
        // Telugu faces too, for the Telugu interface.
        let telugu = scripts.split(',').any(|s| s.trim() == "Telu");
        // And the Hebrew ones (small), for Hebrew text and Edit text on the web.
        let hebrew = scripts.split(',').any(|s| s.trim() == "Hebr");
        let web_face =
            (*family == "BIZ UDPGothic" && *style == "Regular") || arabic || telugu || hebrew || scripts.split(',').any(|s| s.trim() == "Thai");
        if wasm && !web_face {
            continue;
        }
        let path = dir.join(file).canonicalize().map_err(|e| format!("{file}: {e}"))?;
        println!("cargo::rerun-if-changed={}", path.display());
        let scripts: Vec<String> = scripts.split(',').map(|s| format!("{:?}", s.trim())).collect();
        let _ = writeln!(
            out,
            "    CraftFont {{ family: {family:?}, style: {style:?}, scripts: &[{}], bytes: include_bytes!({:?}) }},",
            scripts.join(", "),
            path.display().to_string(),
        );
    }
    Ok(out)
}
