//! The suite theme tools: the selection every Craft app shares and the theme library.
//!
//! [`Automation::new`] keeps the suite in memory, so tests and agents never touch the shared
//! folder; `pdfcraft-cli` passes [`SuiteDir::resolve`] (the folder, or `CRAFT_SUITE_DIR`).

use craft_theme::{AppDefaults, Mode, ThemeId};
use craft_theme_store::{Accepted, SelectionChange, Store, StoreError, StoreOptions, SuiteDir, ThemeRecord};

use super::error::{Result, failed};
use super::*;

/// What PdfCraft shows while no Craft app has chosen a suite theme or mode.
fn app_defaults() -> AppDefaults {
    AppDefaults { theme: ThemeId::suite_default(), mode: Mode::Dark }
}

fn mode_name(mode: Mode) -> &'static str {
    match mode {
        Mode::System => "system",
        Mode::Light => "light",
        Mode::Dark => "dark",
    }
}

fn store_error(e: &StoreError) -> ToolError {
    match e {
        StoreError::Conflict { disk, .. } => failed(format!(
            "another Craft app changed the suite theme first (now {} / {}); call suite_get and try again",
            disk.suite.theme.as_ref().map_or("unchosen", ThemeId::as_str),
            disk.suite.mode.map_or("unchosen", mode_name)
        )),
        other => failed(other.to_string()),
    }
}

/// A theme's warnings: the roles filled from Graphite and the advisory contrast findings.
fn warnings(record: &ThemeRecord) -> Value {
    json!({
        "missing": record.missing.iter().map(|m| json!({ "variant": format!("{:?}", m.variant).to_lowercase(), "role": m.role.key() })).collect::<Vec<_>>(),
        "contrast": record
            .diagnostics
            .iter()
            .map(|d| json!({ "variant": format!("{:?}", d.variant).to_lowercase(), "fg": format!("{:?}", d.fg), "bg": format!("{:?}", d.bg), "ratio": (d.ratio * 100.0).round() / 100.0 }))
            .collect::<Vec<_>>(),
    })
}

impl Automation {
    /// Read and write the suite theme in `dir` instead of in memory.
    pub fn with_suite_dir(mut self, dir: SuiteDir) -> Self {
        self.suite_dir = dir;
        self.suite = None;
        self
    }

    /// The store, opened on first use, with `selection.json` re-read (parsed only when it
    /// changed). A damaged or newer file keeps the last state; [`Self::suite_get`] reports it.
    fn suite_store(&mut self) -> (&mut Store, Option<String>) {
        let store = self.suite.get_or_insert_with(|| Store::open(&self.suite_dir, StoreOptions::new(concat!("pdfcraft-automation ", env!("CARGO_PKG_VERSION")))));
        let problem = store.load_selection().err().map(|p| p.to_string());
        (store, problem)
    }

    fn suite_json(&self, accepted: &Accepted, problem: Option<String>) -> Value {
        let shown = accepted.suite.with_defaults(&app_defaults());
        json!({
            "theme": accepted.suite.theme.as_ref().map(ThemeId::as_str),
            "mode": accepted.suite.mode.map(mode_name),
            "effective": { "theme": shown.theme.as_str(), "mode": mode_name(shown.mode) },
            "revision": accepted.revision,
            "library_revision": accepted.library_revision,
            "shared": !self.suite_dir.is_memory(),
            "problem": problem,
        })
    }

    /// `suite_get`: the suite selection (unchosen fields null) and what PdfCraft shows for it.
    pub(crate) fn suite_get(&mut self) -> Result<Value> {
        let (store, problem) = self.suite_store();
        let accepted = store.accepted().clone();
        Ok(self.suite_json(&accepted, problem))
    }

    /// `suite_set`: change the suite's theme, mode or both, as every following Craft app shows it.
    pub(crate) fn suite_set(&mut self, a: &Args) -> Result<Value> {
        let mut change = SelectionChange::default();
        if let Some(id) = a.opt_str("theme")? {
            let id = ThemeId::parse(id).map_err(|e| ToolError::InvalidArgs(format!("theme {id:?}: {e}")))?;
            if id.is_app_default() {
                return Err(ToolError::InvalidArgs("craft.app-default is each app's own palette, not a suite theme".into()));
            }
            change.theme = Some(id);
        }
        if let Some(mode) = a.opt_str("mode")? {
            change.mode = Some(match mode {
                "system" => Mode::System,
                "light" => Mode::Light,
                "dark" => Mode::Dark,
                m => return Err(ToolError::InvalidArgs(format!("unknown mode {m:?} (system, light, dark)"))),
            });
        }
        if change.is_empty() {
            return Err(ToolError::InvalidArgs("suite_set: give theme, mode or both".into()));
        }
        let (store, _) = self.suite_store();
        if let Some(id) = &change.theme
            && id.is_builtin()
            && craft_theme::builtin(id).is_none()
        {
            return Err(failed(format!("no built-in theme has the id {id}")));
        }
        let base = store.accepted().clone();
        let accepted = store.update(&base, change).map_err(|e| store_error(&e))?;
        Ok(self.suite_json(&accepted, None))
    }

    /// `theme_list`: App default, the built-ins and the library, with the suite's theme marked.
    pub(crate) fn theme_list(&mut self) -> Result<Value> {
        let (store, problem) = self.suite_store();
        let active = store.accepted().suite.with_defaults(&app_defaults()).theme;
        let library = store.library();
        let mut themes = vec![json!({ "id": craft_theme::APP_DEFAULT, "name": "App default", "origin": "built-in", "variants": ["light", "dark"], "warnings": 0, "suite": false })];
        for t in craft_theme::builtins() {
            let variants: Vec<&str> = t.variants().map(|(v, _)| if v == craft_theme::Variant::Light { "light" } else { "dark" }).collect();
            themes.push(json!({ "id": t.id.as_str(), "name": t.name, "origin": "built-in", "variants": variants, "warnings": 0, "suite": t.id == active }));
        }
        for e in &library.entries {
            let variants: Vec<&str> = [(e.has_light, "light"), (e.has_dark, "dark")].into_iter().filter(|(has, _)| *has).map(|(_, v)| v).collect();
            themes.push(json!({ "id": e.id.as_str(), "name": e.name, "origin": "library", "version": e.version, "variants": variants, "warnings": e.diagnostics, "suite": e.id == active }));
        }
        let problems: Vec<String> = library.problems.iter().map(ToString::to_string).chain(problem).collect();
        Ok(json!({ "count": themes.len(), "themes": themes, "problems": problems }))
    }

    /// `theme_import`: add a theme file to the library, its bytes kept exactly. Contrast
    /// warnings never block it.
    pub(crate) fn theme_import(&mut self, a: &Args) -> Result<Value> {
        let path = self.resolve(a.str("path")?, false)?;
        let bytes = read_input(&path)?;
        let as_copy = a.opt_bool("as_copy")?.unwrap_or(false);
        let (store, _) = self.suite_store();
        let imported = store.import(&bytes, as_copy).map_err(|e| match e {
            StoreError::IdExists(id) => failed(format!("the library already has a theme with the id {id}; pass as_copy: true to keep both")),
            other => store_error(&other),
        })?;
        let t = &imported.record.theme;
        Ok(json!({
            "id": t.id.as_str(),
            "name": t.name,
            "library_revision": imported.accepted.library_revision,
            "warnings": warnings(&imported.record),
        }))
    }

    /// `theme_export`: write a library or built-in theme as a theme file.
    pub(crate) fn theme_export(&mut self, a: &Args) -> Result<Value> {
        let path = self.resolve(a.str("path")?, true)?;
        let id = a.str("id")?;
        let id = ThemeId::parse(id).map_err(|e| ToolError::InvalidArgs(format!("id {id:?}: {e}")))?;
        let (store, _) = self.suite_store();
        let bytes = store.export(&id).map_err(|e| store_error(&e))?;
        write_atomic(&path, &bytes)?;
        Ok(json!({ "id": id.as_str(), "path": path.to_string_lossy(), "bytes": bytes.len() }))
    }
}
