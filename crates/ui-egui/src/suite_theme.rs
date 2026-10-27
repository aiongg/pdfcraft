//! The suite theme the Craft apps share, as PdfCraft follows it.
//!
//! [`Suite`] is the app's one seam onto `craft_theme_store::SuiteClient`, which owns the update
//! protocol: optimistic picks, one change in flight, conflicts and failed writes kept until
//! "Apply mine", Retry or the next pick, and no flash of the app's own palette while a picked
//! library theme loads. Only the desktop shell opens the shared folder; everything else (the app
//! constructor, tests, the web build, portable copies) keeps the suite in memory, where it isn't
//! shared, "Follow suite" is unavailable and the theme is the app's own.

use std::sync::Arc;

use craft_theme::{AppDefaults, Mode, SuiteTheme, ThemeId};
pub use craft_theme_store::SuiteDir;
use craft_theme_store::{Accepted, Op, Problem, SelectionChange, Snapshot, StoreError, SuiteChoice, SuiteClient, SuiteWatch};

/// What PdfCraft shows while it follows the suite and no Craft app has picked a suite theme yet.
pub fn app_defaults() -> AppDefaults {
    AppDefaults { theme: ThemeId::suite_default(), mode: Mode::Dark }
}

/// The suite selection and theme library as this app sees them.
pub struct Suite {
    client: SuiteClient,
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

    /// `shared`: the watch reads the folder the other Craft apps use.
    pub fn new(watch: SuiteWatch, shared: bool) -> Self {
        let mut client = SuiteClient::new(watch);
        client.sync();
        Self { client, shared }
    }

    /// Whether other Craft apps see this suite. In memory "Follow suite" has nothing to follow.
    pub fn shared(&self) -> bool {
        self.shared
    }

    /// Takes a new snapshot when there is one; one atomic load otherwise. Returns whether
    /// anything shown may have changed.
    pub fn sync(&mut self) -> bool {
        self.client.sync()
    }

    /// The user picked a suite theme.
    pub fn select_theme(&mut self, id: ThemeId) {
        self.change(SelectionChange::theme(id));
    }

    /// The user picked a suite mode.
    pub fn select_mode(&mut self, mode: Mode) {
        self.change(SelectionChange::mode(mode));
    }

    /// Sends the fields the user changed; they show at once.
    pub fn change(&mut self, change: SelectionChange) {
        self.client.change(change);
        // In memory the answer is already there.
        self.client.sync();
    }

    /// The suite choice to show, with the changes on their way.
    pub fn choice(&self) -> SuiteChoice {
        self.client.shown()
    }

    /// The selection as last confirmed.
    pub fn accepted(&self) -> &Accepted {
        self.client.accepted()
    }

    /// Keeps the library theme the app's own choice uses loaded.
    pub fn set_app_theme(&mut self, id: Option<&ThemeId>) {
        self.client.set_app_theme(id.cloned());
    }

    /// True while `id` is a library theme picked but not loaded yet: keep the current look.
    pub fn is_loading(&self, id: &ThemeId) -> bool {
        self.client.is_loading(id)
    }

    /// A built-in or a loaded library theme, for resolution.
    pub fn lookup(&self, id: &ThemeId) -> Option<SuiteTheme> {
        self.client.theme(id)
    }

    /// Whether `id` names a theme a choice may use: App default, a built-in or a library theme.
    pub fn knows(&self, id: &ThemeId) -> bool {
        id.is_app_default() || craft_theme::builtin(id).is_some() || self.client.snapshot().index.iter().any(|e| &e.id == id)
    }

    /// The last change another Craft app's choice blocked: the suite's state and the change.
    pub fn conflict(&self) -> Option<&(Accepted, SelectionChange)> {
        self.client.conflict()
    }

    /// The last change that failed to save, with the reason.
    pub fn failure(&self) -> Option<&(SelectionChange, StoreError)> {
        self.client.failure()
    }

    /// Files the suite can't use right now.
    pub fn problems(&self) -> &[Problem] {
        self.client.problems()
    }

    /// Sends the change from the last conflict again, on the suite's current state.
    pub fn apply_mine(&mut self) {
        self.client.apply_mine();
        self.client.sync();
    }

    /// Sends the last failed change again.
    pub fn retry(&mut self) {
        self.client.retry();
        self.client.sync();
    }

    /// Clears the conflict and failure notices.
    pub fn dismiss(&mut self) {
        self.client.dismiss();
    }

    /// A library operation (import, export, delete, rescan); its result appears in a later
    /// snapshot under the returned id.
    pub fn request(&mut self, op: Op) -> u64 {
        let id = self.client.watch().request(op);
        self.client.sync();
        id
    }

    pub fn snapshot(&self) -> &Arc<Snapshot> {
        self.client.snapshot()
    }

    pub fn generation(&self) -> u64 {
        self.client.snapshot().generation
    }

    /// `ui.state.suite`'s notices: a blocked or failed change and unusable files, as text.
    pub fn notices(&self) -> serde_json::Value {
        serde_json::json!({
            "conflict": self.conflict().map(|(disk, mine)| serde_json::json!({
                "suite": { "theme": disk.suite.theme.as_ref().map(ThemeId::as_str), "mode": disk.suite.mode },
                "mine": { "theme": mine.theme.as_ref().map(ThemeId::as_str), "mode": mine.mode },
            })),
            "failure": self.failure().map(|(_, error)| error.to_string()),
            "problems": self.problems().iter().map(ToString::to_string).collect::<Vec<_>>(),
        })
    }
}
