//! Suite themes in PdfCraft: the adapter maps the same core colours as every Craft app, the saved
//! choice migrates without changing an existing look, and the suite is shared through a folder.
//! Every test uses a temporary suite folder or the in-memory suite, never the shared one.

use std::time::{Duration, Instant};

use craft_theme::{AppChoice, CoreRoles, Mode, Selection, ThemeId, Variant};
use craft_theme_store::testing::{SAMPLE_THEME_ID, SAMPLE_THEME_JSON, sample_roles};
use craft_theme_store::{SelectionChange, Store, StoreOptions, SuiteDir};
use egui_kittest::Harness;
use pdfcraft_ui_egui::PdfCraftApp;
use pdfcraft_ui_egui::theme::{ThemeKind, ThemePreference, Tokens};

/// The PdfCraft tokens that carry a core role unchanged (pilot plan §2.7).
fn assert_core(t: &Tokens, roles: &CoreRoles, what: &str) {
    let c = craft_ui::theme::color32;
    let got = [t.text, t.panel, t.card, t.pasteboard, t.accent, t.selected];
    let want = [c(roles.text), c(roles.surface), c(roles.raised), c(roles.background), c(roles.accent), c(roles.selection)];
    assert_eq!(got, want, "{what}: text, panel, card, pasteboard, accent, selected");
}

fn store(dir: &std::path::Path) -> Store {
    let mut s = Store::open(&SuiteDir::at(dir), StoreOptions::new("test"));
    s.load_selection().unwrap();
    s
}

fn app_on(dir: &std::path::Path) -> PdfCraftApp {
    let mut app = PdfCraftApp::new();
    app.set_option("language", "en").unwrap();
    app.open_suite(&SuiteDir::at(dir), || {});
    app
}

/// Polls the app's suite until `done` holds; the watcher works on its own thread.
fn wait(app: &mut PdfCraftApp, what: &str, done: impl Fn(&PdfCraftApp) -> bool) {
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        app.suite_mut().sync();
        if done(app) {
            return;
        }
        assert!(Instant::now() < until, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn shown(app: &PdfCraftApp) -> (String, Mode) {
    let s = app.effective_selection();
    (s.theme.as_str().to_owned(), s.mode)
}

fn harness_with(app: PdfCraftApp) -> Harness<'static, PdfCraftApp> {
    let mut slot = Some(app);
    let mut h = Harness::builder().with_size(egui::vec2(1100.0, 800.0)).build_eframe(move |_cc| slot.take().unwrap_or_else(PdfCraftApp::new));
    h.run_steps(2);
    h
}

#[test]
fn the_sample_theme_and_every_built_in_give_the_shared_core_colours() {
    // The sample theme through a suite folder, followed, in Follow system: each slot its variant.
    let dir = tempfile::tempdir().unwrap();
    let mut s = store(dir.path());
    s.import(SAMPLE_THEME_JSON.as_bytes(), false).unwrap();
    let base = s.accepted().clone();
    s.update(&base, SelectionChange::whole(Selection { theme: ThemeId::parse(SAMPLE_THEME_ID).unwrap(), mode: Mode::System })).unwrap();
    let mut app = app_on(dir.path());
    app.set_follow_suite(true);
    wait(&mut app, "the sample theme", |a| a.suite().lookup(&ThemeId::parse(SAMPLE_THEME_ID).unwrap()).is_some() && shown(a).0 == SAMPLE_THEME_ID);
    let h = harness_with(app);
    assert_core(&Tokens::of(&h.ctx, egui::Theme::Light), &sample_roles(Variant::Light), "sample light");
    assert_core(&Tokens::of(&h.ctx, egui::Theme::Dark), &sample_roles(Variant::Dark), "sample dark");
    assert!(h.state().theme_fallback().is_none());
    drop(h);

    // Every built-in as the app's own choice.
    for theme in craft_theme::builtins() {
        let mut app = PdfCraftApp::new();
        app.set_appearance_theme(theme.id.as_str()).unwrap();
        let h = harness_with(app);
        for (slot, variant) in [(egui::Theme::Light, Variant::Light), (egui::Theme::Dark, Variant::Dark)] {
            let roles = theme.variant(variant).or_else(|| theme.variants().next().map(|(_, r)| r)).unwrap();
            assert_core(&Tokens::of(&h.ctx, slot), roles, &format!("{} {variant:?}", theme.id.as_str()));
        }
    }

    // A fresh profile follows the suite and, with nothing chosen, shows ArtCraft dark.
    let mut fresh = PdfCraftApp::new();
    fresh.restore("{}");
    assert!(fresh.follows_suite());
    let h = harness_with(fresh);
    let artcraft = craft_theme::builtin(&ThemeId::suite_default()).unwrap();
    assert_eq!(h.state().theme, ThemeKind::Dark);
    assert_core(&Tokens::get(&h.ctx), artcraft.variant(Variant::Dark).unwrap(), "fresh profile");
}

#[test]
fn saved_choices_migrate_as_the_pilot_plan_says() {
    let own = |theme: &str, mode| AppChoice::Own(Selection { theme: ThemeId::parse(theme).unwrap(), mode });
    for (saved, expected) in [
        (r#"{"theme": "Dark"}"#, own("craft.app-default", Mode::Dark)),
        (r#"{"theme": "System"}"#, own("craft.app-default", Mode::System)),
        ("{}", AppChoice::FollowSuite),
        (r#"{"recent": []}"#, AppChoice::FollowSuite),
        (
            r#"{"theme": "Dark", "appearance": {"follow_suite": false, "theme_id": "craft.graphite", "mode": "light"}}"#,
            own("craft.graphite", Mode::Light),
        ),
        (r#"{"theme": "Dark", "appearance": {"follow_suite": true, "theme_id": "craft.graphite", "mode": "light"}}"#, AppChoice::FollowSuite),
        // Unreadable fields fall back to the app's own palette and the legacy mode.
        (r#"{"theme": "Light", "appearance": {"follow_suite": false, "theme_id": "Not An Id", "mode": 7}}"#, own("craft.app-default", Mode::Light)),
    ] {
        let mut app = PdfCraftApp::new();
        app.set_follow_suite(false);
        app.set_theme_preference(ThemePreference::Light);
        app.restore(saved);
        assert_eq!(app.appearance(), &expected, "{saved}");
    }
    // Garbage keeps the current choice.
    let mut app = PdfCraftApp::new();
    app.set_theme_preference(ThemePreference::Dark);
    app.restore(r#"{"theme": "Purple"}"#);
    assert_eq!(app.appearance(), &own("craft.app-default", Mode::Dark));

    // A legacy profile looks exactly as before: the same tokens in both slots.
    let legacy = {
        let mut app = PdfCraftApp::new();
        app.restore(r#"{"theme": "Dark"}"#);
        harness_with(app)
    };
    let before = {
        let mut app = PdfCraftApp::new();
        app.set_theme_preference(ThemePreference::Dark);
        harness_with(app)
    };
    for slot in [egui::Theme::Light, egui::Theme::Dark] {
        assert_eq!(format!("{:?}", Tokens::of(&legacy.ctx, slot)), format!("{:?}", Tokens::of(&before.ctx, slot)));
        assert_eq!(format!("{:?}", legacy.ctx.style_of(slot)), format!("{:?}", before.ctx.style_of(slot)));
    }
}

#[test]
fn the_choice_and_the_suite_survive_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let mut a = app_on(dir.path());
    a.set_follow_suite(true);
    a.set_appearance_theme("craft.graphite").unwrap();
    a.set_theme_preference(ThemePreference::Light);
    assert_eq!(shown(&a), ("craft.graphite".into(), Mode::Light), "a pick shows at once");
    wait(&mut a, "the write", |a| a.suite().accepted().suite.mode == Some(Mode::Light) && a.suite().accepted().suite.theme.is_some());
    let saved = a.persist();
    drop(a);

    let mut b = PdfCraftApp::new();
    b.restore(&saved);
    assert!(b.follows_suite());
    b.open_suite(&SuiteDir::at(dir.path()), || {});
    // The saved suite state shows before the folder has been read, then the folder agrees.
    assert_eq!(shown(&b), ("craft.graphite".into(), Mode::Light));
    wait(&mut b, "the folder", |b| b.suite().accepted().revision >= 1);
    assert_eq!(shown(&b), ("craft.graphite".into(), Mode::Light));

    // The app's own choice round-trips through the preferences alone.
    b.set_follow_suite(false);
    b.set_appearance_theme("craft.high-contrast").unwrap();
    let mut c = PdfCraftApp::new();
    c.restore(&b.persist());
    assert_eq!(shown(&c), ("craft.high-contrast".into(), Mode::Light));
    assert!(!c.follows_suite());
}

#[test]
fn an_own_theme_ignores_suite_changes_until_follow_suite_is_back_on() {
    let dir = tempfile::tempdir().unwrap();
    let mut a = app_on(dir.path());
    let mut b = app_on(dir.path());
    a.set_follow_suite(true);
    b.set_option("follow_suite", "false").unwrap();
    b.set_option("theme_id", "craft.graphite").unwrap();
    assert!(b.set_option("theme_id", "user.not-there").is_err());
    assert!(b.set_option("follow_suite", "maybe").is_err());

    a.set_appearance_theme("craft.high-contrast").unwrap();
    wait(&mut a, "a's write", |a| a.suite().accepted().suite.theme.as_ref().map(ThemeId::as_str) == Some("craft.high-contrast"));
    wait(&mut b, "b to see the suite", |b| b.suite().accepted().suite.theme.as_ref().map(ThemeId::as_str) == Some("craft.high-contrast"));
    assert_eq!(shown(&b).0, "craft.graphite", "the app's own theme stays");

    b.set_option("follow_suite", "true").unwrap();
    assert_eq!(shown(&b).0, "craft.high-contrast");
    // In memory there is no suite to share: a pick while following goes to the app's own choice.
    let mut alone = PdfCraftApp::new();
    alone.set_follow_suite(true);
    alone.set_appearance_theme("craft.graphite").unwrap();
    assert_eq!(alone.appearance(), &AppChoice::Own(Selection { theme: ThemeId::parse("craft.graphite").unwrap(), mode: Mode::Dark }));
}

#[test]
fn a_pick_another_app_beat_shows_the_suite_and_keeps_a_conflict() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app_on(dir.path());
    app.set_follow_suite(true);
    // Another app picks Graphite; this one hasn't seen it when the user picks High contrast.
    let mut other = store(dir.path());
    let base = other.accepted().clone();
    other.update(&base, SelectionChange::theme(ThemeId::parse("craft.graphite").unwrap())).unwrap();
    app.set_appearance_theme("craft.high-contrast").unwrap();
    wait(&mut app, "the conflict", |a| a.suite().conflict().is_some());
    assert_eq!(shown(&app).0, "craft.graphite", "the suite's state shows, not the refused pick");
    let (_, mine) = app.suite().conflict().unwrap();
    assert_eq!(mine.theme.as_ref().map(ThemeId::as_str), Some("craft.high-contrast"));

    app.suite_mut().apply_mine();
    wait(&mut app, "apply mine", |a| a.suite().accepted().suite.theme.as_ref().map(ThemeId::as_str) == Some("craft.high-contrast"));
    assert!(app.suite().conflict().is_none());
    assert_eq!(shown(&app).0, "craft.high-contrast");
}
