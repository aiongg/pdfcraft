//! Preferences › Appearance. Every suite here is in memory or in a temporary folder, never the
//! shared one.

use std::time::{Duration, Instant};

use craft_theme::AppChoice;
use craft_theme_store::{SelectionChange, Store, StoreOptions, SuiteDir};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

use super::*;

fn app() -> PdfCraftApp {
    let mut app = PdfCraftApp::new();
    app.set_option("language", "en").unwrap();
    app
}

/// The app following the suite in the temporary folder `dir`.
fn following(dir: &std::path::Path) -> PdfCraftApp {
    let mut app = app();
    app.open_suite(&SuiteDir::at(dir), || {});
    assert!(app.set_follow_suite(true));
    app
}

/// The Appearance section on its own, with the app's fonts and file picks.
fn harness(app: PdfCraftApp) -> Harness<'static, PdfCraftApp> {
    let mut fonts = false;
    let mut h = Harness::builder().with_size(egui::vec2(720.0, 900.0)).build_ui_state(
        move |ui, app: &mut PdfCraftApp| {
            // The app's fonts (the semibold headings) take effect from the next frame.
            if !fonts {
                ui.ctx().set_fonts(theme::font_definitions());
                fonts = true;
                return;
            }
            app.process_picked();
            let t = Tokens::get(ui.ctx());
            preferences_section(ui, app, &t);
        },
        app,
    );
    h.run_steps(3);
    h
}

/// The suite selection's theme on disk, as another Craft app reads it.
fn disk_theme(dir: &std::path::Path) -> Option<String> {
    let mut store = Store::open(&SuiteDir::at(dir), StoreOptions::new("test"));
    store.load_selection().unwrap();
    store.accepted().suite.theme.as_ref().map(|t| t.as_str().to_owned())
}

/// Runs frames until `done` holds; the suite watcher works on its own thread.
fn wait(h: &mut Harness<'static, PdfCraftApp>, what: &str, done: impl Fn(&mut Harness<'static, PdfCraftApp>) -> bool) {
    let until = Instant::now() + Duration::from_secs(10);
    while !done(h) {
        assert!(Instant::now() < until, "timed out waiting for {what}");
        h.state_mut().suite_mut().sync();
        h.run_steps(1);
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn own(theme: &str, mode: Mode) -> AppChoice {
    AppChoice::Own(Selection { theme: ThemeId::parse(theme).unwrap(), mode })
}

/// The mode is chosen in Preferences, the cards list ArtCraft, App default and the other
/// built-ins, and the one shown is marked Active.
#[test]
fn preferences_choose_the_appearance_mode_and_list_the_themes() {
    let mut h = harness(app());
    for name in ["ArtCraft", "App default", "Graphite", "High contrast"] {
        assert!(h.query_by_label(name).is_some(), "{name}");
    }
    assert_eq!(h.query_all_by_label("Active").count(), 1);
    if let Ok(path) = std::env::var("PDFCRAFT_APPEARANCE_SHOT") {
        h.render().unwrap().save(path).unwrap();
    }

    // The field-highlight preference is on for new settings, and the box turns it off.
    assert!(h.state().view_defaults.highlight_fields);
    h.get_by_label("Highlight fields").click();
    h.run_steps(2);
    assert!(!h.state().view_defaults.highlight_fields);

    let combo = egui::accesskit::Role::ComboBox;
    h.get_by_role(combo).click();
    h.run_steps(2);
    h.get_by_label("Dark").click();
    h.run_steps(2);
    assert_eq!(h.state().theme_preference(), ThemePreference::Dark);
    h.get_by_role(combo).click();
    h.run_steps(2);
    h.get_by_label("Use system setting").click();
    h.run_steps(2);
    assert_eq!(h.state().theme_preference(), ThemePreference::System);
}

/// Without a shared suite (the web build, portable copies) there is nothing to follow: no
/// Follow suite choice, and a card is the app's own theme.
#[test]
fn without_a_suite_follow_suite_is_hidden_and_a_card_is_the_apps_own_theme() {
    let mut h = harness(app());
    assert!(!h.state().suite_available());
    assert!(h.query_by_label("Follow suite").is_none());
    assert!(h.query_by_label("Use a different theme in this app").is_none());
    // The card's name is not a widget of its own: the click lands on the card.
    h.get_by_label("Graphite").click();
    h.run_steps(2);
    assert_eq!(h.state().appearance(), &own("craft.graphite", Mode::System), "the mode stays");
    assert_eq!(h.query_all_by_label("Active").count(), 1);
}

/// While following the suite a card writes the suite, which every following Craft app shows;
/// with "Use a different theme in this app" it is the app's own choice and the suite stays.
#[test]
fn a_card_writes_the_suite_when_following_and_the_apps_own_choice_otherwise() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = harness(following(dir.path()));
    assert!(h.query_by_label("Changes here apply to every Craft app that follows the suite").is_some());
    h.get_by_label("Graphite").click();
    h.run_steps(2);
    wait(&mut h, "the suite write", |_| disk_theme(dir.path()).as_deref() == Some("craft.graphite"));
    assert_eq!(h.state().appearance(), &AppChoice::FollowSuite);

    h.get_by_label("Use a different theme in this app").click();
    h.run_steps(2);
    assert!(!h.state().follows_suite());
    h.get_by_label("High contrast").click();
    h.run_steps(2);
    assert_eq!(h.state().effective_selection().theme.as_str(), "craft.high-contrast");
    assert!(matches!(h.state().appearance(), AppChoice::Own(s) if s.theme.as_str() == "craft.high-contrast"));
    // Give a stray write time to land: the suite keeps Graphite.
    for _ in 0..5 {
        h.state_mut().suite_mut().sync();
        h.run_steps(1);
        std::thread::sleep(Duration::from_millis(40));
    }
    assert_eq!(disk_theme(dir.path()).as_deref(), Some("craft.graphite"));
    assert_eq!(h.state().suite().accepted().revision, 1);

    // Back to the suite: its theme shows again.
    h.get_by_label("Follow suite").click();
    h.run_steps(2);
    assert_eq!(h.state().effective_selection().theme.as_str(), "craft.graphite");
}

/// A pick another Craft app beat shows a notice; Apply mine writes the pick over it.
#[test]
fn a_conflict_offers_apply_mine_which_writes_the_pick() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = harness(following(dir.path()));
    // Another app picks Graphite; this one hasn't seen it when the user picks High contrast.
    let mut other = Store::open(&SuiteDir::at(dir.path()), StoreOptions::new("other"));
    other.load_selection().unwrap();
    let base = other.accepted().clone();
    other.update(&base, SelectionChange::theme(ThemeId::parse("craft.graphite").unwrap())).unwrap();
    h.get_by_label("High contrast").click();
    h.run_steps(2);
    wait(&mut h, "the conflict notice", |h| h.query_by_label("Apply mine").is_some());
    assert!(h.query_by_label("Dismiss").is_some());
    assert_eq!(h.state().effective_selection().theme.as_str(), "craft.graphite", "the suite's state shows");

    h.get_by_label("Apply mine").click();
    h.run_steps(2);
    wait(&mut h, "apply mine", |_| disk_theme(dir.path()).as_deref() == Some("craft.high-contrast"));
    wait(&mut h, "the notice to go", |h| h.query_by_label("Apply mine").is_none());
    assert_eq!(h.state().effective_selection().theme.as_str(), "craft.high-contrast");
}

/// A suite write that fails shows a notice with Retry, which sends the pick again.
#[cfg(unix)]
#[test]
fn a_failed_suite_write_offers_retry() {
    use std::os::unix::fs::PermissionsExt as _;
    let dir = tempfile::tempdir().unwrap();
    let mode = |m: u32| std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(m)).unwrap();
    let mut h = harness(following(dir.path()));
    mode(0o555);
    h.get_by_label("Graphite").click();
    h.run_steps(2);
    wait(&mut h, "the failure notice", |h| h.query_by_label("Retry").is_some());
    assert!(h.query_by_label("Couldn't save the suite theme").is_some());
    mode(0o755);
    h.get_by_label("Retry").click();
    h.run_steps(2);
    wait(&mut h, "the retry", |_| disk_theme(dir.path()).as_deref() == Some("craft.graphite"));
    wait(&mut h, "the notice to go", |h| h.query_by_label("Retry").is_none());
}

/// A theme whose text barely stands out from its backgrounds, in both variants.
const LOW_CONTRAST: &str = r##"{
  "schema": "craft-theme", "schema_version": 1, "id": "test.low-contrast", "name": "Low contrast", "version": 1,
  "light": {
    "background": "#7A7A7A", "surface": "#808080", "raised": "#858585",
    "text": "#8A8A8A", "text_muted": "#888888", "line": "#7D7D7D",
    "accent": "#828282", "on_accent": "#8C8C8C", "selection": "#838383",
    "focus": "#848484", "good": "#7E7E7E", "warn": "#7F7F7F", "bad": "#818181"
  },
  "dark": {
    "background": "#7A7A7A", "surface": "#808080", "raised": "#858585",
    "text": "#8A8A8A", "text_muted": "#888888", "line": "#7D7D7D",
    "accent": "#828282", "on_accent": "#8C8C8C", "selection": "#838383",
    "focus": "#848484", "good": "#7E7E7E", "warn": "#7F7F7F", "bad": "#818181"
  }
}"##;

/// A low-contrast theme imports with its warnings shown and its colours kept, and once picked
/// PdfCraft paints with them unchanged. The same id again offers Import as copy.
#[test]
fn a_low_contrast_import_shows_warnings_and_keeps_its_colours() {
    let files = tempfile::tempdir().unwrap();
    let path = files.path().join("low.json");
    std::fs::write(&path, LOW_CONTRAST).unwrap();
    // The whole app with Preferences open, so the pick installs its styles.
    let mut app = app();
    app.dialog = Some(crate::Dialog::Preferences);
    app.pick_override = Some(vec![path.to_string_lossy().into_owned()]);
    let mut slot = Some(app);
    let mut h = Harness::builder().with_size(egui::vec2(1100.0, 900.0)).build_eframe(move |_cc| slot.take().unwrap_or_else(PdfCraftApp::new));
    h.run_steps(6);
    let import = |h: &mut Harness<'static, PdfCraftApp>| {
        h.get_by_label("Import…").scroll_to_me();
        // The scroll area eases there over a few frames.
        h.run_steps(8);
        h.get_by_label("Import…").click();
        h.run_steps(4);
    };
    import(&mut h);
    let id = ThemeId::parse("test.low-contrast").unwrap();
    let entry = h.state().suite().snapshot().index.iter().find(|e| e.id == id).cloned().expect("imported");
    assert!(entry.diagnostics > 0);
    assert!(h.query_by_label("Imported Low contrast").is_some());
    assert!(h.query_by_label_contains("Its colours are kept as they are").is_some());
    let warnings = format!("{} warnings", entry.diagnostics);
    assert!(h.query_by_label(&warnings).is_some(), "the card's warnings line");

    h.get_by_label("Low contrast").scroll_to_me();
    h.run_steps(8);
    h.get_by_label("Low contrast").click();
    h.run_steps(6);
    assert_eq!(h.state().effective_selection().theme, id);
    let t = Tokens::get(&h.ctx);
    assert_eq!(
        (t.text, t.pasteboard, t.panel),
        (Color32::from_gray(0x8A), Color32::from_gray(0x7A), Color32::from_gray(0x80)),
        "the file's colours, unchanged"
    );

    // The same file again: the id is taken, and Import as copy keeps both.
    import(&mut h);
    assert!(h.query_by_label("Theme already installed").is_some());
    h.get_by_label("Import as copy").click();
    h.run_steps(4);
    assert_eq!(h.state().suite().snapshot().index.len(), 2);
    assert!(h.query_by_label("Theme already installed").is_none());
}
