//! The Linux appearance path with a fake desktop source. kittest does not call eframe's
//! `raw_input_hook`, so each step runs `prepare_raw_input` by hand, as production does.

use crate::appearance_source::AppearanceSource;
use crate::theme::{ThemeKind, ThemePreference, Tokens};
use craft_platform::appearance::Theme;
use egui_kittest::Harness;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct Fake {
    theme: Mutex<Option<Theme>>,
    active: Mutex<Vec<bool>>,
    starts: Mutex<usize>,
}

/// Starts nothing, as when the portal is unavailable.
fn failing_harness(fake: &Arc<Fake>) -> Harness<'static, crate::PdfCraftApp> {
    let fake = fake.clone();
    Harness::builder().with_size(egui::vec2(1100.0, 800.0)).build_eframe(move |_cc| {
        let mut app = crate::PdfCraftApp::new();
        app.set_option("language", "en").unwrap();
        app.enable_desktop_theme();
        app.set_appearance_factory(Box::new(move |_ctx| {
            if let Ok(mut starts) = fake.starts.lock() {
                *starts += 1;
            }
            None
        }));
        app
    })
}

struct Source(Arc<Fake>);

impl AppearanceSource for Source {
    fn current(&self) -> Option<Theme> {
        self.0.theme.lock().ok().and_then(|theme| *theme)
    }

    fn set_active(&self, active: bool) {
        if let Ok(mut log) = self.0.active.lock() {
            log.push(active);
        }
    }
}

fn harness(fake: &Arc<Fake>, enabled: bool, preference: ThemePreference) -> Harness<'static, crate::PdfCraftApp> {
    let fake = fake.clone();
    Harness::builder().with_size(egui::vec2(1100.0, 800.0)).build_eframe(move |_cc| {
        let mut app = crate::PdfCraftApp::new();
        app.set_option("language", "en").unwrap();
        app.set_theme_preference(preference);
        if enabled {
            app.enable_desktop_theme();
        }
        app.set_appearance_factory(Box::new(move |_ctx| {
            if let Ok(mut starts) = fake.starts.lock() {
                *starts += 1;
            }
            Some(Box::new(Source(fake.clone())))
        }));
        app
    })
}

fn step(h: &mut Harness<'static, crate::PdfCraftApp>) {
    let ctx = h.ctx.clone();
    let mut raw = std::mem::take(h.input_mut());
    h.state_mut().prepare_raw_input(&ctx, &mut raw);
    *h.input_mut() = raw;
    h.run_steps(3);
}

fn starts(fake: &Fake) -> usize {
    fake.starts.lock().map(|n| *n).unwrap_or(usize::MAX)
}

#[test]
fn the_desktop_answer_replaces_the_toolkit_answer_when_following_the_system() {
    let fake = Arc::new(Fake::default());
    let mut h = harness(&fake, true, ThemePreference::System);
    *fake.theme.lock().unwrap() = Some(Theme::Dark);
    h.input_mut().system_theme = Some(egui::Theme::Light);
    step(&mut h);
    step(&mut h);
    assert_eq!(Tokens::get(&h.ctx).kind, ThemeKind::Dark, "desktop Dark beats winit Light");
    assert_eq!(h.state().theme, ThemeKind::Dark);

    *fake.theme.lock().unwrap() = None;
    h.input_mut().system_theme = Some(egui::Theme::Dark);
    step(&mut h);
    step(&mut h);
    assert_eq!(Tokens::get(&h.ctx).kind, ThemeKind::Light, "no desktop answer clears winit's");

    h.state_mut().set_theme_preference(ThemePreference::Light);
    step(&mut h);
    assert_eq!(fake.active.lock().unwrap().last(), Some(&false), "a fixed mode silences the watcher");
    assert_eq!(starts(&fake), 1);
}

#[test]
fn the_watcher_starts_only_when_enabled_and_following_the_system() {
    let fake = Arc::new(Fake::default());
    let mut disabled = harness(&fake, false, ThemePreference::System);
    step(&mut disabled);
    assert_eq!(starts(&fake), 0, "headless hosts never enable the portal");
    let mut fixed = harness(&fake, true, ThemePreference::Dark);
    step(&mut fixed);
    assert_eq!(starts(&fake), 0, "a fixed mode does not start it");
}

#[test]
fn a_failed_watcher_start_is_not_retried_every_frame() {
    let fake = Arc::new(Fake::default());
    let mut h = failing_harness(&fake);
    for _ in 0..4 {
        step(&mut h);
    }
    assert_eq!(starts(&fake), 1);
    assert_eq!(Tokens::get(&h.ctx).kind, ThemeKind::Light, "the Light fallback applies");
}
