//! The desktop appearance as the app reads it: the XDG portal watcher in production, a fake in
//! tests.

use craft_platform::appearance::{Theme, Watcher};

/// What the app needs from a desktop appearance subscription.
pub(crate) trait AppearanceSource {
    /// The cached desktop preference; `None` when there is no known one.
    fn current(&self) -> Option<Theme>;
    /// Enable or suppress change notifications.
    fn set_active(&self, active: bool);
}

impl AppearanceSource for Watcher {
    fn current(&self) -> Option<Theme> {
        Watcher::current(self)
    }

    fn set_active(&self, active: bool) {
        Watcher::set_active(self, active);
    }
}

/// Starts a subscription whose changes repaint the given context.
pub(crate) type AppearanceFactory = Box<dyn Fn(&egui::Context) -> Option<Box<dyn AppearanceSource>>>;

/// The production factory: the portal watcher.
pub(crate) fn portal_source(ctx: &egui::Context) -> Option<Box<dyn AppearanceSource>> {
    let ctx = ctx.clone();
    let watcher = Watcher::start_active(true, move || ctx.request_repaint())?;
    Some(Box::new(watcher))
}
