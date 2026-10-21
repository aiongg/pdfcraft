//! The interface fonts with and without the optional craft-fonts build input (`CRAFT_FONTS_DIR`)
//! and the faces installed on this machine.

use craft_ui::fonts::{I18nScripts, LanguageScripts, Sample, assert_covers};
use egui::epaint::text::{Fonts, TextOptions};
use egui::{Color32, FontFamily, FontId};
use pdfcraft_ui_egui::i18n::LANGUAGES;
use pdfcraft_ui_egui::theme;

const JAPANESE: &str = "日本語の文字";
const ARABIC: &str = "واحد اثنين";
const TELUGU: &str = "తెలుగు ఫైల్";

fn families() -> Vec<FontId> {
    vec![FontId::proportional(13.0), FontId::monospace(13.0), theme::medium(13.0), theme::semibold(17.0)]
}

/// Lays `text` out in every interface family and returns each galley's width.
fn layout_widths(fonts: &mut Fonts, text: &str) -> Vec<f32> {
    let mut view = fonts.with_pixels_per_point(2.0);
    families().into_iter().map(|id| view.layout_no_wrap(text.to_owned(), id, Color32::BLACK).size().x).collect()
}

fn embedded(script: &str) -> bool {
    theme::craft_faces().iter().any(|f| f.scripts.contains(&script))
}

/// The craft-fonts script tag a language's interface text needs; `None` for the scripts Inter has.
fn craft_script(code: &str) -> Option<&'static str> {
    match code {
        "ja" => Some("Jpan"),
        "zh-hans" => Some("Hans"),
        "zh-hant" => Some("Hant"),
        "ar" => Some("Arab"),
        "te" => Some("Telu"),
        _ => None,
    }
}

/// Every shipped translation and every language's own name (the Preferences menu) draws with the
/// embedded faces alone, no installed fonts. A language in a script Inter lacks is checked when
/// the build has a craft-fonts face for it: run with `CRAFT_FONTS_DIR` to check what a release
/// embeds. craft-fonts has no Chinese or Korean face PdfCraft may embed (Noto Sans CJK and its
/// Noto Sans SC/TC/HK/KR UI cuts are Source Han, AGENTS.md §1.1) and no Telugu face yet, so those
/// interfaces still rely on installed fonts.
#[test]
fn ui_fonts_cover_every_catalog() {
    let checked: Vec<_> = LANGUAGES
        .iter()
        .filter(|l| match craft_script(l.code) {
            Some(script) => embedded(script),
            None => {
                let scripts = I18nScripts.scripts(l.code);
                assert!(scripts.iter().all(|s| ["Latn", "Cyrl", "Grek"].contains(s)), "{}: {scripts:?} needs a craft_script entry", l.code);
                true
            }
        })
        .collect();
    let skipped: Vec<&str> = LANGUAGES.iter().filter(|l| !checked.iter().any(|c| c.code == l.code)).map(|l| l.code).collect();
    if !skipped.is_empty() {
        eprintln!("ui_fonts_cover_every_catalog: not checking {skipped:?}: no embedded face (set CRAFT_FONTS_DIR to check what a release embeds)");
    }
    let mut samples: Vec<Sample<'_>> = Vec::new();
    for language in &checked {
        samples.push(Sample { language: language.code, key: "language name", text: language.name });
        for line in language.source.lines().filter(|l| !l.starts_with('#')) {
            let mut columns = line.split('\t');
            if let (Some(key), Some(text)) = (columns.nth(1), columns.next()) {
                samples.push(Sample { language: language.code, key, text });
            }
        }
    }
    // File names and document titles in scripts the interface isn't translated to.
    for (script, text) in [("Arab", ARABIC), ("Hebr", "קובץ חדש"), ("Thai", "ไฟล์เอกสาร"), ("Telu", TELUGU), ("Jpan", JAPANESE)]
    {
        if embedded(script) {
            samples.push(Sample { language: script, key: "content", text });
        }
    }
    assert!(samples.len() > 1000, "{}", samples.len());
    assert_covers(&theme::font_stack("en", false), samples);
}

/// The interface's CJK faces are the build-time cuts, never the whole faces documents use, and
/// never a Noto CJK face (AGENTS.md §1.1).
#[test]
fn interface_cjk_faces_are_cuts() {
    let cjk = ["Hans", "Hant", "Hant-HK", "Jpan", "Kore", "Hani"];
    let faces: Vec<_> = theme::craft_faces().iter().filter(|f| f.scripts.iter().any(|s| cjk.contains(s))).collect();
    if pdfcraft_fonts::CRAFT_FONTS.iter().any(|f| f.family == "BIZ UDPGothic") {
        assert!(faces.iter().any(|f| f.family == "BIZ UDPGothic"), "no Japanese interface face");
    } else {
        eprintln!("interface_cjk_faces_are_cuts: built without craft-fonts (set CRAFT_FONTS_DIR to run it)");
    }
    for face in faces {
        assert!(!face.family.starts_with("Noto "), "{} is embedded", face.name());
        if let Some(whole) = pdfcraft_fonts::CRAFT_FONTS.iter().find(|d| d.name() == face.name()) {
            assert!(face.bytes.len() < whole.bytes.len() / 4, "{}: {} of {} bytes", face.name(), face.bytes.len(), whole.bytes.len());
        }
    }
}

/// Every family starts with PdfCraft's own face in a Latin, Japanese or Chinese interface: Latin
/// text keeps Inter, and the craft-fonts faces come after it. An Arabic interface with an Arabic
/// face puts that face first in the text families, so its rows fit the marks; code keeps
/// JetBrains Mono.
#[test]
fn app_faces_lead_unless_the_language_needs_taller_rows() {
    let leads = [
        (FontFamily::Proportional, "Inter"),
        (FontFamily::Monospace, "JetBrainsMono"),
        (FontFamily::Name("medium".into()), "Inter-Medium"),
        (FontFamily::Name("semibold".into()), "Inter-SemiBold"),
    ];
    for language in ["en", "ja", "zh-hans", "ru"] {
        let stack = theme::font_stack(language, false);
        for (family, first) in &leads {
            let chain = stack.chain(family);
            assert_eq!(chain.first().map(String::as_str), Some(*first), "{language} {family:?}: {chain:?}");
        }
    }
    let arabic: Vec<String> = pdfcraft_fonts::ui_arabic_fonts().iter().map(|f| f.name()).collect();
    if arabic.is_empty() {
        eprintln!("skipping the Arabic interface: no Arab face bundled (set CRAFT_FONTS_DIR with an Arabic face to run it)");
        return;
    }
    let stack = theme::font_stack("ar", false);
    for (family, first) in &leads {
        let chain = stack.chain(family);
        let lead = chain.first().cloned().unwrap_or_default();
        if *family == FontFamily::Monospace {
            assert_eq!(lead, *first, "{chain:?}");
        } else {
            assert!(arabic.iter().any(|a| lead.starts_with(a.as_str())), "{family:?}: {chain:?}");
            assert_eq!(chain.get(1).map(String::as_str), Some(*first), "{family:?}: {chain:?}");
        }
    }
}

/// Built with a craft-fonts Telugu face, Telugu text is shaped in every family: the conjunct క్ష
/// is narrower than క్ and ష held apart by a zero-width non-joiner.
#[test]
fn telugu_ui_text_is_shaped() {
    if !embedded("Telu") {
        eprintln!("skipping telugu_ui_text_is_shaped: no Telu face bundled (set CRAFT_FONTS_DIR with a Telugu face to run it)");
        return;
    }
    let mut fonts = theme::font_stack("te", false).fonts(false);
    let joined = layout_widths(&mut fonts, "క్ష");
    let apart = layout_widths(&mut fonts, "క్\u{200c}ష");
    for (j, a) in joined.iter().zip(&apart) {
        assert!(j < a, "క్ష is one conjunct, narrower than క్‌ష: {j} vs {a}");
    }
}

/// On a machine with suitable installed fonts, Arabic, Hindi, Telugu (#1531), Sinhala and Tamil
/// text has glyphs in every family, such as a file name or Telugu's name in Preferences. macOS
/// always ships Telugu, Sinhala and Tamil faces, so there those never skip.
#[test]
fn installed_faces_draw_scripts_without_an_embedded_face() {
    let off = std::env::var_os("PDFCRAFT_SYSTEM_FONTS").is_some_and(|v| v == "0");
    let mut fonts = theme::font_stack("en", true).fonts(true);
    for (text, macos) in [(ARABIC, false), ("हिंदी पाठ सम्पादन", false), (TELUGU, true), ("ශ්‍රී ලංකා ගොනුව", true), ("தமிழ் கோப்பு", true)]
    {
        let drawn = families().into_iter().all(|id| fonts.has_glyphs(&id, text));
        if !drawn && (off || !macos || !cfg!(target_os = "macos")) {
            eprintln!("skipping {text}: no installed font has it (or PDFCRAFT_SYSTEM_FONTS=0)");
            continue;
        }
        assert!(drawn, "{text}");
        assert!(layout_widths(&mut fonts, text).iter().all(|w| w.is_finite() && *w > 0.0));
    }
}

/// The Han characters of a catalog's translations.
fn catalog_han(catalog: &str) -> String {
    let mut han: Vec<char> = catalog
        .lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| l.split('\t').nth(2))
        .flat_map(str::chars)
        .filter(|c| matches!(*c, '\u{3400}'..='\u{9fff}' | '\u{f900}'..='\u{faff}'))
        .collect();
    han.sort_unstable();
    han.dedup();
    han.into_iter().collect()
}

/// In a Chinese interface built without a Chinese craft face (every release so far, #826), a
/// Chinese face installed on this machine goes right after the app's faces, before the Japanese
/// faces, which lack hundreds of the catalog's characters, and draws every Han character of both
/// Chinese catalogs. Other interfaces, and the stack without installed faces, don't take it.
#[test]
fn chinese_ui_uses_an_installed_chinese_face() {
    assert!(!theme::font_stack("zh-hans", false).definitions.font_data.contains_key(theme::SYSTEM_HAN));
    assert!(!theme::font_stack("ja", true).definitions.font_data.contains_key(theme::SYSTEM_HAN));
    if embedded("Hans") || embedded("Hant") {
        eprintln!("skipping chinese_ui_uses_an_installed_chinese_face: a Chinese face is bundled");
        return;
    }
    for (code, catalog) in [("zh-hans", include_str!("../src/i18n/zh-hans.tsv")), ("zh-hant", include_str!("../src/i18n/zh-hant.tsv"))] {
        let stack = theme::font_stack(code, true);
        if !stack.definitions.font_data.contains_key(theme::SYSTEM_HAN) {
            eprintln!("skipping chinese_ui_uses_an_installed_chinese_face: no installed Chinese face (or PDFCRAFT_SYSTEM_FONTS=0)");
            return;
        }
        for (family, chain) in &stack.definitions.families {
            let system = chain.iter().position(|n| n == theme::SYSTEM_HAN).expect("in every family");
            assert_eq!(chain.iter().filter(|n| *n == theme::SYSTEM_HAN).count(), 1, "{family:?}");
            assert!(chain[..system].iter().all(|n| n.starts_with("Inter") || n == "JetBrainsMono"), "{family:?}: {chain:?}");
        }
        let han = catalog_han(catalog);
        assert!(han.contains('欢') || han.contains('說'), "{code}");
        let mut fonts = stack.fonts(false);
        for id in families() {
            let missing: String = han.chars().filter(|c| !fonts.has_glyphs(&id, &c.to_string())).collect();
            assert!(missing.is_empty(), "{code} {id:?} lacks {missing}");
        }
    }
}

/// Latin widgets keep egui's 20 px height under PdfCraft's 5 px button padding, whether the theme
/// is applied before or after the fonts are installed, and after a language switch; an Arabic
/// interface with an Arabic face makes them taller to fit its rows, and switching back undoes it.
#[test]
fn language_switch_keeps_the_widget_height() {
    let installed_first = egui::Context::default();
    theme::install_fonts(&installed_first);
    theme::apply(&installed_first, theme::ThemeKind::default());
    let applied_first = egui::Context::default();
    theme::apply(&applied_first, theme::ThemeKind::default());
    theme::install_fonts(&applied_first);
    for ctx in [installed_first, applied_first] {
        ctx.run_ui(egui::RawInput::default(), |_| {}).textures_delta.clear();
        let height = || ctx.global_style().spacing.interact_size.y;
        assert_eq!(ctx.global_style().spacing.button_padding.y, 5.0);
        assert_eq!(height(), 20.0);
        theme::install_fonts_for(&ctx, "de");
        assert_eq!(height(), 20.0);
        if embedded("Arab") {
            theme::install_fonts_for(&ctx, "ar");
            assert!(height() > 20.0, "{}", height());
            theme::install_fonts_for(&ctx, "de");
            assert_eq!(height(), 20.0);
        }
    }
}

/// Without craft-fonts the interface fonts still install and lay out any text (Japanese falls
/// back to egui's replacement glyph) without panicking; Latin text is unaffected.
#[test]
fn ui_fonts_work_without_craft_fonts() {
    let mut fonts = Fonts::new(TextOptions::default(), theme::font_definitions());
    for id in families() {
        assert!(fonts.has_glyphs(&id, "PdfCraft"), "{id:?}");
        assert_eq!(fonts.has_glyphs(&id, JAPANESE), embedded("Jpan"), "{id:?}");
    }
    assert!(layout_widths(&mut fonts, JAPANESE).iter().all(|w| w.is_finite() && *w > 0.0));
    assert!(layout_widths(&mut fonts, "PdfCraft").iter().all(|w| *w > 20.0));
    let ctx = egui::Context::default();
    for language in &LANGUAGES {
        theme::install_fonts_for(&ctx, language.code);
    }
    theme::install_fonts(&ctx);
}

/// Latin- and Cyrillic-script catalogs (Czech, Brazilian Portuguese, German, Spanish, French, Russian, Bulgarian, Hungarian, Ukrainian, Italian, Slovene) are drawn entirely by the app's own faces
/// (Inter, JetBrains Mono with Inter fallback): no letter falls through to egui's defaults or a CJK fallback. (egui's
/// `has_glyphs` can't answer this: with only the primary face it is also the replacement face.)
#[test]
fn primary_ui_fonts_cover_latin_and_cyrillic_catalogs() {
    use skrifa::MetadataProvider as _;
    let defs = theme::font_definitions();
    assert_eq!(&defs.families[&FontFamily::Monospace][..2], ["JetBrainsMono", "Inter"]);
    let fallback = &defs.font_data["Inter"];
    let fallback_font = skrifa::FontRef::from_index((*fallback.font).as_ref(), fallback.index).unwrap();
    let fallback_charmap = fallback_font.charmap();
    for (code, catalog) in [
        ("cs", include_str!("../src/i18n/cs.tsv")),
        ("el", include_str!("../src/i18n/el.tsv")),
        ("pt-br", include_str!("../src/i18n/pt-br.tsv")),
        ("de", include_str!("../src/i18n/de.tsv")),
        ("es", include_str!("../src/i18n/es.tsv")),
        ("fr", include_str!("../src/i18n/fr.tsv")),
        ("ru", include_str!("../src/i18n/ru.tsv")),
        ("kk", include_str!("../src/i18n/kk.tsv")),
        ("bg", include_str!("../src/i18n/bg.tsv")),
        ("hu", include_str!("../src/i18n/hu.tsv")),
        ("uk", include_str!("../src/i18n/uk.tsv")),
        ("it", include_str!("../src/i18n/it.tsv")),
        ("tr", include_str!("../src/i18n/tr.tsv")),
        ("sl", include_str!("../src/i18n/sl.tsv")),
        ("lt", include_str!("../src/i18n/lt.tsv")),
        ("vi", include_str!("../src/i18n/vi.tsv")),
    ] {
        let mut text = String::from("áčďéěíňóřšťúůýžÁČĎÉĚÍŇÓŘŠŤÚŮÝŽãõçâêôàÃÕÇÂÊÔÀñÑüÜäÄöÖőűß„“¿¡«»…ґҐєЄіІїЇәғқңөұүһӘҒҚҢӨҰҮҺ");
        for line in catalog.lines().filter(|l| !l.starts_with('#')) {
            if let Some(translation) = line.split('\t').nth(2) {
                text.extend(translation.chars().filter(|c| !c.is_whitespace()));
            }
        }
        for name in ["Inter", "Inter-Medium", "Inter-SemiBold", "JetBrainsMono"] {
            let data = &defs.font_data[name];
            let font = skrifa::FontRef::from_index((*data.font).as_ref(), data.index).unwrap();
            let charmap = font.charmap();
            let mut missing: Vec<char> =
                text.chars().filter(|c| charmap.map(*c).is_none() && (name != "JetBrainsMono" || fallback_charmap.map(*c).is_none())).collect();
            missing.sort_unstable();
            missing.dedup();
            assert!(missing.is_empty(), "{code}: {name} lacks {missing:?}");
        }
    }
}

/// Characters no catalog uses, typed into a text field: a rare Han character (𠮷, U+20BB7), a
/// Vietnamese letter (ẫ) and a key symbol (⌘). They draw without a panic, and none is a box while
/// an embedded face has it: Inter has ẫ and ⌘. The Han character is a box here: the interface's CJK
/// faces are cut to the catalogs, and on desktop an installed face draws it.
#[test]
fn characters_outside_the_catalogs_draw_in_a_text_field() {
    use egui_kittest::kittest::Queryable as _;
    use skrifa::MetadataProvider as _;
    const TYPED: &str = "𠮷ẫ⌘";
    let stack = theme::font_stack("en", false);
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(320.0, 60.0)).build_ui_state(
        |ui, text: &mut String| {
            ui.add(egui::TextEdit::singleline(text)).accessible_name("Field");
        },
        String::new(),
    );
    stack.install(&h.ctx);
    h.ctx.set_missing_glyph_policy(egui::MissingGlyphPolicy::Tofu);
    h.run();
    h.get_by_label("Field").click();
    h.run();
    h.event(egui::Event::Text(TYPED.into()));
    h.run();
    assert_eq!(h.state().as_str(), TYPED);

    let mut fonts = stack.fonts(false);
    let font_id = FontId::proportional(13.0);
    for c in TYPED.chars() {
        let embedded = stack
            .definitions
            .font_data
            .values()
            .any(|d| skrifa::FontRef::from_index((*d.font).as_ref(), d.index).is_ok_and(|font| font.charmap().map(c).is_some()));
        if embedded {
            assert!(fonts.has_glyph(&font_id, c), "{c} (U+{:04X}) is a box although an embedded face has it", u32::from(c));
        }
    }
    assert!(fonts.has_glyph(&font_id, 'ẫ') && fonts.has_glyph(&font_id, '⌘'));
}
