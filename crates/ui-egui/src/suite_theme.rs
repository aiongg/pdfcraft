//! The suite theme the Craft apps share, as PdfCraft follows it.
//!
//! [`Suite`] is the app's one seam onto the `craft-theme-store` watcher (callers use
//! `select_theme`, `select_mode`, `choice`, `accepted`, `notice`), so a shared client can replace
//! its internals later. It takes a new snapshot only when the
//! watcher's generation moves, shows the user's suite changes at once (optimistic updates) and
//! reconciles them with the worker's results, keeping a conflict or a failed write as a
//! [`SuiteNotice`] for the Appearance dialog. Only the desktop shell opens the shared folder;
//! everything else (the app constructor, tests, the web build, portable copies) keeps the suite in
//! memory, where it isn't shared and changes go to PdfCraft's own choice instead.

use std::sync::Arc;

use craft_theme::{AppDefaults, Mode, SuiteTheme, ThemeId};
pub use craft_theme_store::SuiteDir;
use craft_theme_store::{Accepted, Op, RESULTS_KEPT, SelectionChange, Snapshot, StoreError, SuiteChoice, SuiteWatch};

/// What PdfCraft shows while it follows the suite and no Craft app has picked a suite theme yet.
pub fn app_defaults() -> AppDefaults {
    AppDefaults { theme: ThemeId::suite_default(), mode: Mode::Dark }
}

/// Why the last suite change didn't stick. Data only: the Appearance dialog shows it.
#[derive(Clone, Debug)]
pub enum SuiteNotice {
    /// Another Craft app changed the same field first. The suite shows `disk`; "Apply mine"
    /// sends `mine` again on top of it.
    Conflict { disk: Accepted, mine: SelectionChange },
    /// The write failed and nothing changed; Retry sends `change` again.
    Failed { error: String, change: SelectionChange },
}

impl SuiteNotice {
    /// A one-line description, for `ui.state` and logs.
    pub fn describe(&self) -> String {
        match self {
            SuiteNotice::Conflict { .. } => "Changed in another Craft app; your choice wasn't applied".to_owned(),
            SuiteNotice::Failed { error, .. } => format!("The suite theme couldn't be saved: {error}"),
        }
    }
}

/// The suite selection and theme library as this app sees them.
pub struct Suite {
    watch: SuiteWatch,
    snapshot: Arc<Snapshot>,
    /// Changes sent to the worker and not yet answered, oldest first, by request id.
    pending: Vec<(u64, SelectionChange)>,
    notice: Option<SuiteNotice>,
    /// The library theme last named with [`Op::AppTheme`].
    app_theme: Option<ThemeId>,
    shared: bool,
}

impl Default for Suite {
    fn default() -> Self {
        Self::memory()
    }
}

impl Suite {
    /// A suite in memory, shared with no other app.
    pub fn memory() -> Self {
        Self::new(SuiteWatch::memory(), false)
    }

    /// `shared`: the watcher reads the folder the other Craft apps use.
    pub fn new(watch: SuiteWatch, shared: bool) -> Self {
        let snapshot = watch.snapshot();
        Self { watch, snapshot, pending: Vec::new(), notice: None, app_theme: None, shared }
    }

    /// Whether other Craft apps see this suite. In memory "Follow suite" has nothing to follow,
    /// so the app's changes go to its own choice.
    pub fn shared(&self) -> bool {
        self.shared
    }

    /// Takes the watcher's new snapshot, if there is one, and settles the answered changes.
    /// One atomic load when nothing changed. Returns whether the snapshot changed.
    pub fn poll(&mut self) -> bool {
        if self.watch.generation() == self.snapshot.generation {
            return false;
        }
        self.snapshot = self.watch.snapshot();
        self.settle();
        true
    }

    fn settle(&mut self) {
        let snapshot = Arc::clone(&self.snapshot);
        let oldest_kept = (snapshot.results.len() >= RESULTS_KEPT).then(|| snapshot.results.first().map(|(id, _)| *id)).flatten();
        let mut notice = None;
        self.pending.retain(|(id, change)| match snapshot.result(*id) {
            // Still queued, unless the snapshot has stopped keeping results this old.
            None => oldest_kept.is_none_or(|oldest| *id > oldest),
            Some(Ok(_)) => false,
            Some(Err(StoreError::Conflict { disk, mine })) => {
                notice = Some(SuiteNotice::Conflict { disk: disk.clone(), mine: mine.clone() });
                false
            }
            Some(Err(e)) => {
                notice = Some(SuiteNotice::Failed { error: e.to_string(), change: change.clone() });
                false
            }
        });
        if notice.is_some() {
            self.notice = notice;
        }
    }

    /// The suite choice to show: the accepted one with the unanswered changes applied.
    pub fn choice(&self) -> SuiteChoice {
        self.pending.iter().fold(self.snapshot.accepted.suite.clone(), |choice, (_, change)| change.applied_to(&choice))
    }

    /// The selection as last read or written, without unanswered changes.
    pub fn accepted(&self) -> &Accepted {
        &self.snapshot.accepted
    }

    /// The user picked a suite theme.
    pub fn select_theme(&mut self, id: ThemeId) {
        self.change(SelectionChange::theme(id));
    }

    /// The user picked a suite mode.
    pub fn select_mode(&mut self, mode: Mode) {
        self.change(SelectionChange::mode(mode));
    }

    /// Sends the fields the user changed. The change shows at once; a conflict or a failed
    /// write returns the suite to the accepted state and leaves a [`SuiteNotice`].
    pub fn change(&mut self, change: SelectionChange) {
        if change.is_empty() {
            return;
        }
        // The base is what the user saw, so quick successive changes don't conflict with each
        // other, only with another app's.
        let base = Accepted { suite: self.choice(), ..self.snapshot.accepted.clone() };
        let id = self.watch.request(Op::Update { base, change: change.clone() });
        self.pending.push((id, change));
        self.notice = None;
        self.poll();
    }

    /// Keeps the library theme the app's own choice uses parsed in the snapshot. Built-ins need
    /// no file.
    pub fn set_app_theme(&mut self, id: Option<&ThemeId>) {
        let id = id.filter(|id| !id.is_builtin()).cloned();
        if self.app_theme != id {
            self.app_theme = id.clone();
            self.watch.request(Op::AppTheme(id));
            self.poll();
        }
    }

    /// A built-in, the suite's active theme or the app's own library theme, for resolution.
    pub fn lookup(&self, id: &ThemeId) -> Option<SuiteTheme> {
        self.snapshot.theme(id)
    }

    /// Whether `id` names a theme a choice may use: App default, a built-in or a library theme.
    pub fn knows(&self, id: &ThemeId) -> bool {
        id.is_app_default() || craft_theme::builtin(id).is_some() || self.snapshot.index.iter().any(|e| &e.id == id)
    }

    /// Runs a library operation (import, export, delete, rescan) on the worker; the result
    /// appears in a later snapshot under the returned id.
    pub fn request(&mut self, op: Op) -> u64 {
        let id = self.watch.request(op);
        self.poll();
        id
    }

    pub fn snapshot(&self) -> &Arc<Snapshot> {
        &self.snapshot
    }

    pub fn generation(&self) -> u64 {
        self.snapshot.generation
    }

    pub fn notice(&self) -> Option<&SuiteNotice> {
        self.notice.as_ref()
    }

    pub fn clear_notice(&mut self) {
        self.notice = None;
    }
}
