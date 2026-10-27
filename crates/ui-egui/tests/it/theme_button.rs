//! The title bar's theme button: a click cycles System, Light and Dark (a fresh install starts
//! on System, #1248); a right-click opens the menu with the appearance settings.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use pdfcraft_ui_egui::PdfCraftApp;

fn harness() -> Harness<'static, PdfCraftApp> {
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_eframe(|_cc| {
        let mut app = PdfCraftApp::new();
        app.set_option("language", "en").unwrap();
        app
    });
    h.run_steps(6);
    h
}

fn preference(h: &Harness<'static, PdfCraftApp>) -> String {
    format!("{:?}", h.state().theme_preference())
}

#[test]
fn a_click_cycles_the_theme_and_a_right_click_opens_the_menu() {
    let mut h = harness();
    assert_eq!(preference(&h), "System", "a fresh install follows the desktop (#1248)");
    for next in ["Light", "Dark", "System"] {
        h.get_by_label_contains("Display theme:").click();
        h.run_steps(4);
        assert_eq!(preference(&h), next);
    }
    assert!(h.query_by_label("Appearance settings…").is_none(), "a plain click opens no menu");

    h.get_by_label_contains("Display theme:").click_secondary();
    h.run_steps(4);
    h.get_by_label("Appearance settings…").click();
    h.run_steps(4);
    h.run_steps(10);
    assert!(h.query_by_label("App default").is_some(), "Preferences opens on Appearance");
}

/// The tooltip names the effective theme and the mode it shows now.
#[test]
fn the_tooltip_names_the_theme_and_the_mode() {
    let mut h = harness();
    h.get_by_label("Display theme: App default, light (system)");
    h.get_by_label_contains("Display theme:").click();
    h.run_steps(4);
    h.get_by_label("Display theme: App default, light");
    h.get_by_label_contains("Display theme:").click();
    h.run_steps(4);
    h.get_by_label("Display theme: App default, dark");
}
