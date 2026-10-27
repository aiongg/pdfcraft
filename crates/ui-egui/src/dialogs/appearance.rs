//! Preferences › Appearance: whether PdfCraft follows the suite theme the Craft apps share or uses
//! a theme of its own, a card per theme with a small picture of PdfCraft in it, the appearance
//! mode, importing and exporting theme files, and the suite's notices.
//!
//! Every write goes through [`crate::suite_theme::Suite`]: a card choice is
//! [`PdfCraftApp::set_appearance`] (the suite while following it, the app's own choice
//! otherwise), the notices' buttons are [`PdfCraftApp::answer_suite_notice`], and imports and
//! exports are library requests whose results arrive in a later snapshot.

#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};
use std::sync::Arc;

use craft_theme::{Mode, Selection, SuiteTheme, ThemeId, Variant};
#[cfg(not(target_arch = "wasm32"))]
use craft_theme_store::MAX_THEME_BYTES;
use craft_theme_store::{FileStamp, Op, OpOutput, SelectionChange, Snapshot, StoreError, ThemeRecord};
use egui::{Color32, Rect, RichText, pos2, vec2};

use crate::PdfCraftApp;
use crate::theme::{self, ThemePreference, Tokens};
use crate::widgets;

/// At most this many card themes are read at once, so the library's result list (which keeps
/// [`craft_theme_store::RESULTS_KEPT`]) never drops the answer to an import or export.
const READS_IN_FLIGHT: usize = 4;

/// What the Appearance section keeps between frames: the import or export on its way, how the
/// last one ended, and the library themes read for their cards.
#[derive(Default)]
pub struct Library {
    op: Option<LibraryOp>,
    outcome: Option<Outcome>,
    cards: Vec<(ThemeId, CardTheme)>,
}

enum LibraryOp {
    Import {
        request: u64,
        bytes: Arc<[u8]>,
    },
    #[cfg(not(target_arch = "wasm32"))]
    Export {
        request: u64,
        path: PathBuf,
    },
}

impl LibraryOp {
    fn request(&self) -> u64 {
        match self {
            LibraryOp::Import { request, .. } => *request,
            #[cfg(not(target_arch = "wasm32"))]
            LibraryOp::Export { request, .. } => *request,
        }
    }
}

/// How the last import or export ended, shown under the Import and Export buttons.
enum Outcome {
    Imported(ThemeId),
    /// The library has a theme with this id; "Import as copy" sends the same bytes again.
    IdExists {
        id: ThemeId,
        bytes: Arc<[u8]>,
    },
    #[cfg(not(target_arch = "wasm32"))]
    Exported(PathBuf),
    Failed(String),
}

/// A library theme as its card knows it: read at a file stamp, or on its way.
enum CardTheme {
    Reading {
        request: u64,
        stamp: FileStamp,
    },
    /// `None`: the file couldn't be read or parsed; the card shows no picture.
    Read {
        stamp: FileStamp,
        record: Option<Arc<ThemeRecord>>,
    },
}

impl CardTheme {
    fn stamp(&self) -> FileStamp {
        match self {
            CardTheme::Reading { stamp, .. } | CardTheme::Read { stamp, .. } => *stamp,
        }
    }
}

/// Whether the snapshot's result list has moved past `request` without it (it keeps only the
/// latest results).
fn lost(snapshot: &Snapshot, request: u64) -> bool {
    snapshot.result(request).is_none() && snapshot.results.first().is_some_and(|(first, _)| *first > request)
}

fn mode_label(preference: ThemePreference) -> &'static str {
    match preference {
        ThemePreference::System => "Use system setting",
        ThemePreference::Light => "Light",
        ThemePreference::Dark => "Dark",
    }
}

fn mode_command(preference: ThemePreference) -> &'static str {
    match preference {
        ThemePreference::System => "view.theme.system",
        ThemePreference::Light => "view.theme.light",
        ThemePreference::Dark => "view.theme.dark",
    }
}

/// A theme file from disk, at most [`MAX_THEME_BYTES`] (an import reads untrusted files).
#[cfg(not(target_arch = "wasm32"))]
fn read_theme_file(path: &Path) -> Result<Vec<u8>, String> {
    use std::io::Read as _;
    let name = path.display().to_string();
    let failed = |e: std::io::Error| crate::i18n::fmt(tl!("Couldn't read {name}: {e}"), &[("name", &name), ("e", &e.to_string())]);
    let too_big = || {
        crate::i18n::fmt(
            tl!("{name} is larger than a theme file can be ({max} KiB)."),
            &[("name", &name), ("max", &(MAX_THEME_BYTES / 1024).to_string())],
        )
    };
    let file = std::fs::File::open(path).map_err(failed)?;
    if file.metadata().is_ok_and(|m| m.len() > MAX_THEME_BYTES as u64) {
        return Err(too_big());
    }
    let mut bytes = Vec::new();
    file.take(MAX_THEME_BYTES as u64 + 1).read_to_end(&mut bytes).map_err(failed)?;
    if bytes.len() > MAX_THEME_BYTES {
        return Err(too_big());
    }
    Ok(bytes)
}

impl PdfCraftApp {
    /// Adds a theme file to the library. A low-contrast theme is imported with its colours as
    /// they are; its card shows the warnings. An id the library has already offers "Import as
    /// copy" (`as_copy` stores it under the next free id).
    pub fn import_theme(&mut self, bytes: Arc<[u8]>, as_copy: bool) {
        let request = self.suite.request(Op::Import { bytes: bytes.clone(), as_copy });
        self.theme_library.op = Some(LibraryOp::Import { request, bytes });
        self.theme_library.outcome = None;
    }

    /// [`Self::import_theme`] from a file.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn import_theme_file(&mut self, path: &Path) {
        match read_theme_file(path) {
            Ok(bytes) => self.import_theme(bytes.into(), false),
            Err(e) => self.theme_library.outcome = Some(Outcome::Failed(e)),
        }
    }

    /// Writes theme `id` (built-in or library) to `path` as a theme file others can import.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn export_theme(&mut self, id: ThemeId, path: PathBuf) {
        let request = self.suite.request(Op::Export(id));
        self.theme_library.op = Some(LibraryOp::Export { request, path });
        self.theme_library.outcome = None;
    }

    /// Takes the results of finished library requests from the suite's snapshot.
    fn poll_theme_library(&mut self) {
        let snapshot = self.suite.snapshot().clone();
        if let Some(op) = &self.theme_library.op {
            let request = op.request();
            let outcome = match (op, snapshot.result(request)) {
                (_, None) if lost(&snapshot, request) => Some(Outcome::Failed(tl!("The theme library didn't answer. Please try again.").to_owned())),
                (_, None) => None,
                (LibraryOp::Import { .. }, Some(Ok(OpOutput::Imported(id)))) => Some(Outcome::Imported(id.clone())),
                (LibraryOp::Import { bytes, .. }, Some(Err(StoreError::IdExists(id)))) => {
                    Some(Outcome::IdExists { id: id.clone(), bytes: bytes.clone() })
                }
                #[cfg(not(target_arch = "wasm32"))]
                (LibraryOp::Export { path, .. }, Some(Ok(OpOutput::Exported(bytes)))) => {
                    Some(match crate::editing::write_atomically(&path.to_string_lossy(), bytes) {
                        Ok(()) => Outcome::Exported(path.clone()),
                        Err(e) => Outcome::Failed(crate::i18n::fmt(
                            tl!("Couldn't save {name}: {e}"),
                            &[("name", &path.display().to_string()), ("e", &e.to_string())],
                        )),
                    })
                }
                (_, Some(Err(e))) => Some(Outcome::Failed(e.to_string())),
                (_, Some(Ok(_))) => Some(Outcome::Failed(tl!("The theme library gave an unexpected answer.").to_owned())),
            };
            if let Some(outcome) = outcome {
                self.theme_library.outcome = Some(outcome);
                self.theme_library.op = None;
            }
        }
        for (_, card) in &mut self.theme_library.cards {
            if let CardTheme::Reading { request, stamp } = *card {
                let record = match snapshot.result(request) {
                    Some(Ok(OpOutput::Exported(bytes))) => craft_theme_store::parse_theme(bytes).ok().map(Arc::new),
                    Some(_) => None,
                    None if lost(&snapshot, request) => None,
                    None => continue,
                };
                *card = CardTheme::Read { stamp, record };
            }
        }
    }

    /// The library theme `id` as read for its card, asking for it when its file is new or
    /// changed. Reads only cards on screen and a few at a time, and none while an import or
    /// export waits for its answer.
    fn card_record(&mut self, id: &ThemeId, stamp: FileStamp, visible: bool) -> Option<Arc<ThemeRecord>> {
        let slot = self.theme_library.cards.iter().position(|(card, _)| card == id);
        if let Some(i) = slot
            && let Some((_, card)) = self.theme_library.cards.get(i)
            && card.stamp() == stamp
        {
            return match card {
                CardTheme::Read { record, .. } => record.clone(),
                CardTheme::Reading { .. } => None,
            };
        }
        let reading = self.theme_library.cards.iter().filter(|(_, c)| matches!(c, CardTheme::Reading { .. })).count();
        if !visible || reading >= READS_IN_FLIGHT || self.theme_library.op.is_some() {
            return None;
        }
        let request = self.suite.request(Op::Export(id.clone()));
        let card = CardTheme::Reading { request, stamp };
        match slot.and_then(|i| self.theme_library.cards.get_mut(i)) {
            Some((_, old)) => *old = card,
            None => self.theme_library.cards.push((id.clone(), card)),
        }
        // In memory the answer is already there.
        self.poll_theme_library();
        None
    }

    /// A theme's name as the cards show it.
    pub(crate) fn theme_name(&self, id: &ThemeId) -> String {
        if id.is_app_default() {
            return tl!("App default").to_owned();
        }
        if let Some(theme) = craft_theme::builtin(id) {
            return tl!(theme.name.as_str()).to_owned();
        }
        self.suite.snapshot().index.iter().find(|e| &e.id == id).map_or_else(|| id.as_str().to_owned(), |e| e.name.clone())
    }
}

/// PdfCraft's tokens for `theme` (or its own palette for App default) in `variant`; a theme with
/// one variant shows that one.
fn preview_tokens(id: &ThemeId, theme: Option<&SuiteTheme>, variant: Variant) -> Option<Tokens> {
    if id.is_app_default() {
        return Some(Tokens::app_default(variant));
    }
    let theme = theme?;
    let mode = match variant {
        Variant::Light => Mode::Light,
        Variant::Dark => Mode::Dark,
    };
    let resolution = craft_theme::resolve(&Selection { theme: id.clone(), mode }, None, variant, &|_| Some(theme.clone()));
    Some(Tokens::for_resolution(&resolution))
}

/// A miniature PdfCraft window in `p`'s colours: title bar and tabs, the tool bar, the page
/// thumbnails, a page on the pasteboard and the right-hand panel.
fn preview(ui: &egui::Ui, rect: Rect, p: &Tokens) {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, p.chrome);
    let title = Rect::from_min_size(rect.min, vec2(rect.width(), 12.0));
    painter.rect_filled(title, 0.0, p.titlebar);
    // The active tab and the theme button.
    painter.rect_filled(Rect::from_min_size(title.min + vec2(6.0, 3.0), vec2(36.0, 9.0)), 2.0, p.panel);
    painter.rect_filled(Rect::from_min_size(title.min + vec2(9.0, 6.0), vec2(20.0, 2.0)), 1.0, p.text);
    painter.circle_filled(pos2(title.right() - 7.0, title.center().y), 2.5, p.icon);
    // The tool bar, with its first tool active.
    let tools = Rect::from_min_size(pos2(rect.left(), title.bottom()), vec2(rect.width(), 12.0));
    painter.rect_filled(tools, 0.0, p.panel);
    for i in 0..8 {
        let c = if i == 0 { p.accent } else { p.icon };
        painter.rect_filled(Rect::from_min_size(tools.min + vec2(8.0 + i as f32 * 9.0, 3.5), vec2(5.0, 5.0)), 1.0, c);
    }
    let body = Rect::from_min_max(pos2(rect.left(), tools.bottom() + 1.0), rect.max);
    // Page thumbnails on the left, the second one selected.
    let rail = Rect::from_min_size(body.min, vec2((body.width() * 0.16).round(), body.height()));
    painter.rect_filled(rail, 0.0, p.panel);
    // Three thumbnails fill the rail's height, whatever the picture's size.
    let step = ((rail.height() - 4.0) / 3.0).max(1.0);
    for i in 0..3 {
        let thumb = Rect::from_min_size(rail.min + vec2(rail.width() * 0.25, 5.0 + i as f32 * step), vec2(rail.width() * 0.5, (step - 5.0).max(1.0)));
        if i == 1 {
            painter.rect_filled(thumb.expand(2.0), 2.0, p.selection);
        }
        painter.rect_filled(thumb, 1.0, Color32::WHITE);
        painter.rect_stroke(thumb, 1.0, egui::Stroke::new(0.5, p.border), egui::StrokeKind::Outside);
    }
    // The right-hand panel: a heading and a few rows.
    let side = Rect::from_min_max(pos2(body.right() - (body.width() * 0.22).round(), body.top()), body.max);
    painter.rect_filled(side, 0.0, p.panel);
    painter.rect_filled(Rect::from_min_size(side.min + vec2(5.0, 5.0), vec2(side.width() * 0.5, 2.5)), 1.0, p.text);
    for i in 0..4 {
        let y = side.top() + 14.0 + i as f32 * 8.0;
        painter.rect_filled(Rect::from_min_size(pos2(side.left() + 5.0, y), vec2(side.width() * 0.7, 2.0)), 1.0, p.text_muted);
    }
    // The page, white in every theme, with a heading and lines of text.
    let paste = Rect::from_min_max(pos2(rail.right(), body.top()), pos2(side.left(), body.bottom()));
    painter.rect_filled(paste, 0.0, p.pasteboard);
    let page_h = paste.height() - 10.0;
    let page = Rect::from_center_size(paste.center(), vec2(page_h * 0.77, page_h));
    painter.rect_filled(page.translate(vec2(1.0, 1.0)), 1.0, p.page_shadow);
    painter.rect_filled(page, 1.0, Color32::WHITE);
    let ink = Color32::from_gray(0x60);
    painter.rect_filled(Rect::from_min_size(page.min + vec2(5.0, 6.0), vec2(page.width() * 0.5, 3.0)), 1.0, Color32::from_gray(0x30));
    for i in 0..6 {
        let w = if i % 4 == 3 { 0.45 } else { 0.8 };
        painter.rect_filled(Rect::from_min_size(page.min + vec2(5.0, 13.0 + i as f32 * 6.0), vec2(page.width() * w - 5.0, 1.5)), 0.5, ink);
    }
}

/// One theme as the cards list it.
struct Card {
    id: ThemeId,
    name: String,
    /// Library themes only: their file stamp and warning count.
    library: Option<(FileStamp, usize)>,
}

/// The cards in order: ArtCraft, App default, the other built-ins, then the library themes.
fn cards(app: &PdfCraftApp) -> Vec<Card> {
    let suite_default = ThemeId::suite_default();
    let builtin = |id: &ThemeId| Card { id: id.clone(), name: app.theme_name(id), library: None };
    let mut out = vec![builtin(&suite_default), builtin(&ThemeId::app_default())];
    out.extend(craft_theme::builtins().iter().filter(|t| t.id != suite_default).map(|t| builtin(&t.id)));
    out.extend(app.suite.snapshot().index.iter().map(|e| Card { id: e.id.clone(), name: e.name.clone(), library: Some((e.stamp, e.diagnostics)) }));
    out
}

/// The warnings of a library theme, one per line, for the tooltip of its warnings line.
fn warning_details(record: Option<&ThemeRecord>) -> String {
    let Some(record) = record else {
        return tl!("Contrast too low for some text or controls, or colours the file leaves out. The theme's colours are kept as they are.")
            .to_owned();
    };
    let variant = |v: Variant| match v {
        Variant::Light => tl!("Light"),
        Variant::Dark => tl!("Dark"),
    };
    let paint = |p: craft_theme::contrast::Paint| match p {
        craft_theme::contrast::Paint::Role(role) => role.key(),
        craft_theme::contrast::Paint::Derived(name) => name,
    };
    let mut lines: Vec<String> = record
        .missing
        .iter()
        .map(|m| {
            crate::i18n::fmt(
                tl!("{variant}: {role} is missing; Graphite's colour is used"),
                &[("variant", variant(m.variant)), ("role", m.role.key())],
            )
        })
        .collect();
    lines.extend(record.diagnostics.iter().map(|d| {
        crate::i18n::fmt(
            tl!("{variant}: {fg} on {bg} is {ratio}:1, below {need}:1"),
            &[
                ("variant", variant(d.variant)),
                ("fg", paint(d.fg)),
                ("bg", paint(d.bg)),
                ("ratio", &format!("{:.1}", d.ratio)),
                ("need", &format!("{:.1}", d.need.min_ratio())),
            ],
        )
    }));
    lines.push(tl!("The theme's colours are kept as they are.").to_owned());
    lines.join("\n")
}

/// One theme card: a picture of PdfCraft in the theme, its name, Active when it is the theme
/// shown, and for a library theme its warnings. Returns true when the card was clicked.
fn card(ui: &mut egui::Ui, app: &mut PdfCraftApp, t: &Tokens, card: &Card, active: bool, variant: Variant) -> bool {
    let width = ui.available_width();
    // The picture: the theme as this app would show it, read from the library if need be.
    let (stamp_warnings, record) = match card.library {
        Some((stamp, warnings)) => {
            let visible = ui.is_rect_visible(Rect::from_min_size(ui.cursor().min, vec2(width, 120.0)));
            (Some(warnings), app.card_record(&card.id, stamp, visible))
        }
        None => (None, None),
    };
    let theme = app.suite.lookup(&card.id).or_else(|| record.as_ref().map(|r| r.theme.clone()));
    let tokens = preview_tokens(&card.id, theme.as_ref(), variant);
    let inner = egui::Frame::new()
        .fill(t.card)
        .stroke(egui::Stroke::new(if active { 2.0 } else { 1.0 }, if active { t.accent } else { t.border }))
        .corner_radius(6.0)
        .inner_margin(egui::Margin::same(6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 84.0), egui::Sense::hover());
            match &tokens {
                Some(p) => preview(ui, rect, p),
                // Not read yet, or unreadable: an empty frame of the same size.
                None => {
                    ui.painter().rect_stroke(rect, 3.0, egui::Stroke::new(1.0, t.divider), egui::StrokeKind::Inside);
                }
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add(egui::Label::new(RichText::new(&card.name).font(theme::semibold(12.5)).color(t.text)).truncate());
                if active {
                    ui.label(RichText::new(tl!("Active")).color(t.accent_text).small());
                }
            });
        });
    // The whole card picks its theme, not just its name.
    let clicked = inner.response.interact(egui::Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).clicked();
    if let Some(warnings) = stamp_warnings.filter(|n| *n > 0) {
        let text = crate::i18n::trn(crate::i18n::current(), warnings as u64, "{n} warning", "{n} warnings");
        let details = warning_details(record.as_deref());
        theme::status_label(ui, t.warn, t.text, RichText::new(text).small()).on_hover_text(details);
    }
    clicked
}

/// A notice box: a short status heading in `status` (one of the tokens' bad, warn or good
/// colours), the message, and buttons under it. Returns the index of the button clicked.
fn notice(ui: &mut egui::Ui, t: &Tokens, status: Color32, heading: &str, message: &str, buttons: &[(&str, bool)]) -> Option<usize> {
    egui::Frame::new()
        .fill(t.card)
        .stroke(egui::Stroke::new(1.0, status))
        .corner_radius(6.0)
        .inner_margin(egui::Margin::same(8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            theme::status_label(ui, status, t.text, RichText::new(heading).font(theme::semibold(12.5)));
            if !message.is_empty() {
                ui.label(RichText::new(message).color(t.text));
            }
            let mut clicked = None;
            if !buttons.is_empty() {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    for (i, (label, primary)) in buttons.iter().enumerate() {
                        if widgets::pill_button(ui, label, *primary).clicked() {
                            clicked = Some(i);
                        }
                    }
                });
            }
            clicked
        })
        .inner
}

/// The suite's notices: a change another Craft app's pick blocked, a change that couldn't be
/// saved, and the suite files that can't be used.
fn suite_notices(ui: &mut egui::Ui, app: &mut PdfCraftApp, t: &Tokens) {
    if let Some((_, mine)) = app.suite.conflict() {
        let message = match (&mine.theme, mine.mode) {
            (Some(id), _) => crate::i18n::fmt(
                tl!("Another Craft app changed the suite theme at the same time, so {theme} wasn't applied."),
                &[("theme", &app.theme_name(id))],
            ),
            (None, _) => tl!("Another Craft app changed the suite theme at the same time, so your choice wasn't applied.").to_owned(),
        };
        let buttons = [(tl!("Apply mine"), true), (tl!("Dismiss"), false)];
        match notice(ui, t, t.warn, tl!("Not applied"), &message, &buttons) {
            Some(0) => answer(app, "apply_mine"),
            Some(_) => answer(app, "dismiss"),
            None => {}
        }
        ui.add_space(6.0);
    }
    if let Some((_, error)) = app.suite.failure() {
        let message = error.to_string();
        let buttons = [(tl!("Retry"), true), (tl!("Dismiss"), false)];
        match notice(ui, t, t.bad, tl!("Couldn't save the suite theme"), &message, &buttons) {
            Some(0) => answer(app, "retry"),
            Some(_) => answer(app, "dismiss"),
            None => {}
        }
        ui.add_space(6.0);
    }
    let problems = app.suite.problems();
    if !problems.is_empty() {
        let message = problems.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n");
        notice(ui, t, t.warn, tl!("Some suite theme files can't be used"), &message, &[]);
        ui.add_space(6.0);
    }
}

fn answer(app: &mut PdfCraftApp, action: &str) {
    if let Err(e) = app.answer_suite_notice(action) {
        app.notify(e);
    }
}

/// Import… and Export…, and how the last import or export ended.
fn library_row(ui: &mut egui::Ui, app: &mut PdfCraftApp, t: &Tokens) {
    #[cfg(not(target_arch = "wasm32"))]
    ui.horizontal(|ui| {
        let busy = app.theme_library.op.is_some();
        // Importing installs into the shared suite, so without one there is nothing to import into.
        if app.suite_available()
            && ui
                .add_enabled(!busy, egui::Button::new(tl!("Import…")))
                .on_hover_text(tl!("Add a theme file to the themes every Craft app can use"))
                .clicked()
        {
            let dialog = rfd::AsyncFileDialog::new().set_title(tl!("Import a theme")).add_filter(tl!("Craft theme"), &["json"]);
            app.ask_one(crate::pickers::Ask::File(dialog), None, |app, path| app.import_theme_file(&path));
        }
        let shown = app.effective_selection().theme;
        let exportable = !shown.is_app_default();
        let export = ui.add_enabled(!busy && exportable, egui::Button::new(tl!("Export…")));
        let export = if exportable {
            export.on_hover_text(tl!("Save the theme in use as a file to share"))
        } else {
            export.on_disabled_hover_text(tl!("App default is PdfCraft's own palette, not a theme file"))
        };
        if export.clicked() {
            let dialog = rfd::AsyncFileDialog::new()
                .set_title(tl!("Export the theme"))
                .add_filter(tl!("Craft theme"), &["json"])
                .set_file_name(format!("{}.json", shown.as_str()));
            app.ask_one(crate::pickers::Ask::Save(dialog), None, move |app, path| app.export_theme(shown, path));
        }
    });
    let mut clear = false;
    let mut again = None;
    match &app.theme_library.outcome {
        None => {}
        Some(Outcome::Imported(id)) => {
            let name = app.theme_name(id);
            let warnings = app.suite.snapshot().index.iter().find(|e| &e.id == id).map_or(0, |e| e.diagnostics);
            let message = if warnings == 0 {
                String::new()
            } else {
                crate::i18n::trn(
                    crate::i18n::current(),
                    warnings as u64,
                    "It has {n} warning. Its colours are kept as they are.",
                    "It has {n} warnings. Its colours are kept as they are.",
                )
            };
            let heading = crate::i18n::fmt(tl!("Imported {name}"), &[("name", &name)]);
            clear = notice(ui, t, if warnings == 0 { t.good } else { t.warn }, &heading, &message, &[(tl!("OK"), false)]).is_some();
        }
        Some(Outcome::IdExists { id, bytes }) => {
            let message = crate::i18n::fmt(
                tl!("A theme with the id {id} is installed already. Import this one as a copy to keep both."),
                &[("id", id.as_str())],
            );
            match notice(ui, t, t.warn, tl!("Theme already installed"), &message, &[(tl!("Import as copy"), true), (tl!("Cancel"), false)]) {
                Some(0) => again = Some(bytes.clone()),
                Some(_) => clear = true,
                None => {}
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        Some(Outcome::Exported(path)) => {
            let heading = crate::i18n::fmt(tl!("Exported to {name}"), &[("name", &path.display().to_string())]);
            clear = notice(ui, t, t.good, &heading, "", &[(tl!("OK"), false)]).is_some();
        }
        Some(Outcome::Failed(message)) => {
            clear = notice(ui, t, t.bad, tl!("Couldn't import or export the theme"), message, &[(tl!("Dismiss"), false)]).is_some();
        }
    }
    if let Some(bytes) = again {
        app.import_theme(bytes, true);
    } else if clear {
        app.theme_library.outcome = None;
    }
}

/// The Appearance section of Preferences.
pub fn preferences_section(ui: &mut egui::Ui, app: &mut PdfCraftApp, t: &Tokens) {
    app.suite.sync();
    app.poll_theme_library();
    if app.theme_library.op.is_some() || app.theme_library.cards.iter().any(|(_, c)| matches!(c, CardTheme::Reading { .. })) {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(50));
    }
    ui.label(RichText::new(tl_ctx!("theme", "Appearance")).font(theme::semibold(13.0)));
    // Without a shared suite (the web build, portable copies) the theme is always the app's own.
    if app.suite_available() {
        let follows = app.follows_suite();
        if ui.add(widgets::radio(follows, tl!("Follow suite"))).clicked() && !follows {
            app.execute("view.theme.follow_suite");
        }
        ui.indent("follow-suite-caption", |ui| {
            ui.label(RichText::new(tl!("Changes here apply to every Craft app that follows the suite")).small().color(t.text_muted));
        });
        if ui.add(widgets::radio(!follows, tl!("Use a different theme in this app"))).clicked() && follows {
            app.execute("view.theme.this_app");
        }
        ui.add_space(4.0);
    }
    suite_notices(ui, app, t);
    ui.horizontal(|ui| {
        let label = ui.label(tl!("Appearance mode"));
        let combo = egui::ComboBox::from_id_salt("appearance-mode").selected_text(tl!(mode_label(app.theme_preference()))).show_ui(ui, |ui| {
            for preference in [ThemePreference::System, ThemePreference::Light, ThemePreference::Dark] {
                if ui.selectable_label(app.theme_preference() == preference, tl!(mode_label(preference))).clicked() {
                    app.execute(mode_command(preference));
                    ui.close();
                }
            }
        });
        // The visible label names the combo box for assistive technology and tests.
        combo.response.labelled_by(label.id);
    });
    ui.add_space(4.0);
    let shown = app.effective_selection().theme;
    let variant = Variant::from(app.theme);
    let all = cards(app);
    let columns = ((ui.available_width() / 180.0).floor() as usize).clamp(1, 4);
    let mut picked = None;
    for row in all.chunks(columns) {
        ui.columns(columns, |cols| {
            for (col, c) in cols.iter_mut().zip(row) {
                if card(col, app, t, c, c.id == shown, variant) {
                    picked = Some(c.id.clone());
                }
            }
        });
        ui.add_space(6.0);
    }
    if let Some(id) = picked {
        // The suite while following it, otherwise the app's own choice.
        app.set_appearance(SelectionChange::theme(id));
    }
    library_row(ui, app, t);
    ui.add_space(4.0);
    // Acrobat's "Highlight existing fields": shared with the toggle in the form bar.
    ui.add(crate::widgets::checkbox(&mut app.view_defaults.highlight_fields, tl!("Highlight fields")));
}

#[cfg(test)]
mod tests;
