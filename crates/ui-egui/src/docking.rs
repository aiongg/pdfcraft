//! Application panel identities and persistence around the shared docking renderer.
//! Document contents remain owned by their existing view implementations.

use craft_ui::docking::{
    Action, DockArea, DockCommand, DockOperation, DockStyle, FloatSpace, Layout, Location, Node, PanelLimits, PanelState, Permissions, Placement,
    UiDock, Zone,
};
use craft_ui::layout::{SplitAxis, SplitSize};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Panel {
    Canvas,
    Tools,
    Inspector,
}

impl Panel {
    pub const ALL: &[Self] = &[Self::Canvas, Self::Tools, Self::Inspector];
    pub fn id(self) -> &'static str {
        match self {
            Self::Canvas => "canvas",
            Self::Tools => "tools",
            Self::Inspector => "inspector",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Canvas => "Document",
            Self::Tools => "Tools",
            Self::Inspector => "Inspector",
        }
    }
    fn default_side(self) -> Zone {
        match self {
            Self::Canvas => Zone::Right,
            Self::Tools => Zone::Left,
            Self::Inspector => Zone::Right,
        }
    }
}

/// Docked, open and alone in its group, drawn without a tab strip; panels split beside it.
const FIXED: Permissions = Permissions { accepts_splits: true, ..Permissions::PROTECTED };

fn permissions(panel: &Panel) -> Permissions {
    if *panel == Panel::Canvas { FIXED } else { Permissions::default() }
}

fn tabs(panel: Panel) -> Node<Panel> {
    Node::Tabs { panels: vec![panel], active: 0 }
}

fn split(axis: SplitAxis, size: SplitSize, first: Node<Panel>, second: Node<Panel>) -> Node<Panel> {
    Node::Split { axis, size, first: Box::new(first), second: Box::new(second) }
}

/// The saved arrangement. Closed panels keep their last group, side, size or floating rect.
pub type Workspace = craft_ui::docking::Workspace<Panel>;

/// Tools on the left, the document, and the Inspector on the right.
pub fn default_workspace() -> Workspace {
    Workspace::new(Layout {
        root: Some(split(
            SplitAxis::Horizontal,
            SplitSize::FixedFirst(272.0),
            tabs(Panel::Tools),
            split(SplitAxis::Horizontal, SplitSize::FixedSecond(330.0), tabs(Panel::Canvas), tabs(Panel::Inspector)),
        )),
        floating: Vec::new(),
    })
}

fn error(error: craft_ui::docking::DockError) -> String {
    error.to_string()
}

/// Side of the Tools panel, including its saved position while closed.
pub fn tools_on_right(workspace: &Workspace) -> bool {
    let location = workspace
        .layout
        .location(&Panel::Tools)
        .ok()
        .or_else(|| workspace.hidden.iter().find(|(panel, _)| *panel == Panel::Tools).map(|(_, location)| location.clone()));
    location.is_some_and(|location| matches!(location.placement, Placement::Split(Zone::Right)))
}

/// Place Tools at the outer left or right edge while preserving its open state.
pub fn set_tools_on_right(workspace: &mut Workspace, right: bool) -> Result<(), String> {
    let was_open = workspace.layout.contains(&Panel::Tools);
    set_visible(workspace, Panel::Tools, true)?;
    let anchor = if right && workspace.layout.contains(&Panel::Inspector) { Panel::Inspector } else { Panel::Canvas };
    apply(workspace, Action::Move { panel: Panel::Tools, anchor, placement: Placement::Split(if right { Zone::Right } else { Zone::Left }) })?;
    if !was_open {
        set_visible(workspace, Panel::Tools, false)?;
    }
    Ok(())
}

/// craft-ui's checks plus the document view's: saved settings cannot hide, float, group or
/// collapse it, nor send a hidden panel back onto it.
pub fn validate(workspace: &Workspace) -> Result<(), String> {
    workspace.validate().map_err(error)?;
    if !workspace.layout.contains(&Panel::Canvas) {
        return Err("The panel layout must contain its document view".into());
    }
    if workspace.layout.floating.iter().any(|window| window.contains(&Panel::Canvas)) {
        return Err("The document view cannot float".into());
    }
    let mut nodes: Vec<_> = workspace.layout.root.iter().collect();
    while let Some(node) = nodes.pop() {
        match node {
            Node::Split { first, second, .. } => {
                nodes.push(first);
                nodes.push(second);
            }
            Node::Tabs { panels, .. } if panels.contains(&Panel::Canvas) && panels.len() != 1 => {
                return Err("The document view cannot share a tab group".into());
            }
            Node::Stack { entries } if entries.iter().any(|entry| entry.panel == Panel::Canvas) => {
                return Err("The document view cannot be collapsed".into());
            }
            Node::Dock { columns, .. } if columns.iter().flat_map(|column| &column.groups).any(|group| group.panels.contains(&Panel::Canvas)) => {
                return Err("The document view cannot be in a dock".into());
            }
            _ => {}
        }
    }
    let over_canvas = |location: &Location<Panel>| {
        location.anchor == Some(Panel::Canvas) && matches!(location.placement, Placement::Tab { .. } | Placement::Split(Zone::Center))
    };
    if workspace.hidden.iter().any(|(panel, location)| *panel == Panel::Canvas || over_canvas(location)) {
        return Err("A hidden panel cannot restore over the document view".into());
    }
    Ok(())
}

pub fn apply(workspace: &mut Workspace, action: Action<Panel>) -> Result<(), String> {
    let mut next = workspace.clone();
    next.apply(action, permissions).map_err(error)?;
    validate(&next)?;
    *workspace = next;
    Ok(())
}

/// Show a closed panel at its recorded place, else beside the document on its usual side, or
/// close an open one. A panel that already has the asked visibility keeps its tab selection.
pub fn set_visible(workspace: &mut Workspace, panel: Panel, visible: bool) -> Result<(), String> {
    if workspace.layout.contains(&panel) == visible {
        return Ok(());
    }
    if !visible {
        return apply(workspace, Action::Close { panel });
    }
    validate(workspace)?;
    let mut next = workspace.clone();
    if workspace.hidden.iter().any(|(id, _)| *id == panel) && next.set_visible(panel, true, None, permissions).is_ok() && validate(&next).is_ok() {
        *workspace = next;
        return Ok(());
    }
    apply(workspace, Action::OpenAt { panel, anchor: Panel::Canvas, placement: Placement::Split(panel.default_side()) })
}

/// Show a panel and bring its body to the front: select its tab and open a minimized group.
fn reveal(workspace: &mut Workspace, panel: Panel) -> Result<(), String> {
    set_visible(workspace, panel, true)?;
    apply(workspace, Action::Activate { panel })?;
    if workspace.layout.panel_state(&panel) != PanelState::Front {
        // Refused, and the layout kept, where nothing is minimized.
        let _ = apply(workspace, Action::SetMinimized { panel, minimized: false });
    }
    Ok(())
}

/// Whether a panel's body is showing: its tab selected or its stack entry open, and not minimized.
pub(crate) fn exposed(workspace: &Workspace, panel: Panel) -> bool {
    workspace.layout.panel_state(&panel) == PanelState::Front
}

/// `ui.dock` params, `{action}` or `{operation, …}`, in craft-ui's shared command vocabulary.
fn run(workspace: &mut Workspace, params: &Value) -> Result<(), String> {
    validate(workspace)?;
    let mut tagged = params.clone();
    tagged.as_object_mut().ok_or("ui.dock params must be an object")?.insert("command".into(), json!("ui.dock"));
    let command: DockCommand<Panel> = serde_json::from_value(tagged).map_err(|error| error.to_string())?;
    let mut next = workspace.clone();
    match command {
        DockCommand::UiDock(UiDock::Operation(DockOperation::Open { panel } | DockOperation::Activate { panel })) => reveal(&mut next, panel)?,
        command => {
            // `ui.dock` operations carry their own rects; only `window.panel.float` reads the space.
            command.apply(&mut next, false, &FloatSpace::new(egui::Rect::ZERO), permissions).map_err(error)?;
        }
    }
    validate(&next)?;
    *workspace = next;
    Ok(())
}

fn sync(workspace: &mut Workspace, app: &crate::PdfCraftApp) -> Result<(), String> {
    set_visible(workspace, Panel::Tools, app.left_open)?;
    set_visible(workspace, Panel::Inspector, app.right.is_some())
}

fn drain_reveals(workspace: &mut Workspace, app: &mut crate::PdfCraftApp) {
    for panel in std::mem::take(&mut app.pending_panel_reveals) {
        if workspace.layout.contains(&panel) {
            let _ = reveal(workspace, panel);
        }
    }
}

fn publish_visibility(app: &mut crate::PdfCraftApp, workspace: &Workspace) {
    app.left_open = workspace.layout.contains(&Panel::Tools);
    if !workspace.layout.contains(&Panel::Inspector) {
        if app.right.is_some() {
            app.choose_right_panel(None);
        }
    } else if app.right.is_none() {
        app.choose_right_panel(Some(crate::RightPanel::Comments));
    }
}

pub fn command(app: &mut crate::PdfCraftApp, reset: bool, params: &Value) -> Result<Value, String> {
    let mut workspace = app.docking.clone();
    if reset {
        workspace = default_workspace();
        set_visible(&mut workspace, Panel::Inspector, app.right.is_some())?;
    } else {
        sync(&mut workspace, app)?;
        run(&mut workspace, params)?;
    }
    publish_visibility(app, &workspace);
    app.docking = workspace;
    Ok(json!({"docking": app.docking}))
}

// Split paths change when a temporarily unavailable pane is pruned. Translate renderer
// paths back to the saved tree before applying geometry changes.
fn saved_split_path(saved: &Layout<Panel>, rendered: &Layout<Panel>, path: &[bool]) -> Option<Vec<bool>> {
    let mut target = rendered.root.as_ref()?;
    for second in path {
        let Node::Split { first, second: other, .. } = target else { return None };
        target = if *second { other } else { first };
    }
    let mut pending = vec![(saved.root.as_ref()?, Vec::new())];
    while let Some((node, original_path)) = pending.pop() {
        if let Node::Split { first, second, .. } = node {
            let mut projection = Layout { root: Some(node.clone()), floating: Vec::new() };
            if projection.contains(&Panel::Inspector) {
                let _ = projection.apply(Action::Close { panel: Panel::Inspector });
            }
            let has_available_panel = |child: &Node<Panel>| {
                Layout { root: Some(child.clone()), floating: Vec::new() }.panels().into_iter().any(|panel| *panel != Panel::Inspector)
            };
            // An ancestor whose Inspector-only side disappeared projects to its surviving
            // child. Its divider is absent, so only the child's split may own this resize.
            if has_available_panel(first) && has_available_panel(second) && projection.root.as_ref() == Some(target) {
                return Some(original_path);
            }
            let mut second_path = original_path.clone();
            second_path.push(true);
            pending.push((second, second_path));
            let mut first_path = original_path;
            first_path.push(false);
            pending.push((first, first_path));
        }
    }
    None
}

pub fn show(app: &mut crate::PdfCraftApp, ui: &mut egui::Ui) {
    let t = crate::theme::Tokens::get(ui.ctx());
    let mut workspace = std::mem::replace(&mut app.docking, default_workspace());
    if let Err(error) = validate(&workspace).and_then(|()| sync(&mut workspace, app)) {
        app.notify_fmt("Panel layout could not be restored: {error}", &[("error", &error)]);
        workspace = default_workspace();
        let _ = sync(&mut workspace, app);
    }
    let mut style = DockStyle::from_ui(ui);
    style.min_pane = 96.0;
    // Panels arrange in a split tree around the document; no side docks.
    style.new_dock_zones = false;
    style.float_label = crate::i18n::t("Float panel").into();
    style.close_label = crate::i18n::t("Close panel").into();
    style.move_panel_to_label = crate::i18n::t("Move to group").into();
    style.panels_label = crate::i18n::t("Panels").into();
    style.resize_label = crate::i18n::t("Resize panels").into();
    style.resize_window_label = crate::i18n::t("Resize panel window").into();
    style.background = t.panel;
    style.tab_background = t.chrome;
    style.active_background = t.selected;
    style.text = t.text;
    style.inactive_text = t.text_muted;
    style.border = egui::Stroke::new(1.0, t.divider);
    style.accent = t.accent;
    style.font = crate::theme::medium(12.0);
    drain_reveals(&mut workspace, app);
    let mut rendered = workspace.layout.clone();
    if app.active.is_none() && rendered.contains(&Panel::Inspector) {
        // Availability is a render projection: the saved group, selection and width survive Home.
        let _ = rendered.apply(Action::Close { panel: Panel::Inspector });
    }
    // Tools docked on its own is drawn bare, without a tab strip, like Acrobat's tool pane. Grouped
    // with another panel, or floating, it keeps its tab so it can be dragged away again; moving it
    // to the other side stays a preference and a command.
    let bare_tools = rendered.location(&Panel::Tools).is_ok_and(|l| l.floating.is_none() && matches!(l.placement, Placement::Split(_)));
    let render_permissions = move |panel: &Panel| if bare_tools && *panel == Panel::Tools { FIXED } else { permissions(panel) };
    let output = DockArea::new(egui::Id::new("pdfcraft-docking")).show_with_limits(
        ui,
        &rendered,
        &style,
        |panel| crate::i18n::t(panel.label()).into(),
        render_permissions,
        panel_limits,
        |ui, panel| match panel {
            Panel::Canvas => {
                ui.painter().rect_filled(ui.max_rect(), 0.0, t.pasteboard);
                match app.active {
                    None if app.combine_showing() => crate::combine_ui::page(app, ui),
                    None => crate::home::show(app, ui),
                    Some(index) => match crate::compare_ui::side_by_side(app, index) {
                        Some((old, new)) => crate::compare_ui::compare_area(app, old, new, ui),
                        None => crate::canvas::document_area(app, index, ui),
                    },
                }
            }
            Panel::Tools => crate::panels::left_panel(app, ui),
            Panel::Inspector => {
                ui.data_mut(|data| data.insert_temp(egui::Id::new("pdfcraft-inspector-body"), ui.max_rect()));
                crate::panels::right_panel(app, ui);
            }
        },
    );
    let _ = sync(&mut workspace, app);
    drain_reveals(&mut workspace, app);
    publish_visibility(app, &workspace);
    app.docking = workspace;
    for mut action in output.actions {
        if app.active.is_none()
            && let Action::ResizeSplit { path, .. } = &mut action
        {
            let Some(saved_path) = saved_split_path(&app.docking.layout, &rendered, path) else { continue };
            *path = saved_path;
        }
        if let Err(error) =
            serde_json::to_value(action).map_err(|error| error.to_string()).and_then(|action| command(app, false, &json!({"action": action})))
        {
            app.notify_fmt("Panel move was not applied: {error}", &[("error", &error)]);
        }
    }
}

fn panel_limits(panel: &Panel) -> PanelLimits {
    let (min, max_width) = match panel {
        Panel::Canvas => (egui::vec2(160.0, 160.0), 1_000_000.0),
        Panel::Tools => (egui::vec2(240.0, 120.0), 520.0),
        Panel::Inspector => (egui::vec2(260.0, 120.0), 520.0),
    };
    PanelLimits { min, max: egui::vec2(max_width, 1_000_000.0) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tools_side_moves_to_both_edges_and_survives_closed_restore() {
        let mut workspace = default_workspace();
        assert!(!tools_on_right(&workspace));
        set_tools_on_right(&mut workspace, true).unwrap();
        assert!(tools_on_right(&workspace));
        set_visible(&mut workspace, Panel::Tools, false).unwrap();
        assert!(tools_on_right(&workspace));
        set_tools_on_right(&mut workspace, false).unwrap();
        assert!(!tools_on_right(&workspace));
        assert!(!workspace.layout.contains(&Panel::Tools));
        set_visible(&mut workspace, Panel::Tools, true).unwrap();
        assert!(!tools_on_right(&workspace));
        validate(&workspace).unwrap();
    }

    #[test]
    fn exposed_respects_collapsed_stack_and_inactive_floating_tabs() {
        let mut workspace = Workspace::new(Layout {
            root: Some(split(
                SplitAxis::Horizontal,
                SplitSize::FixedFirst(272.0),
                Node::Stack {
                    entries: vec![
                        craft_ui::docking::StackEntry { panel: Panel::Tools, open: true, height: None },
                        craft_ui::docking::StackEntry { panel: Panel::Inspector, open: false, height: None },
                    ],
                },
                tabs(Panel::Canvas),
            )),
            floating: Vec::new(),
        });
        validate(&workspace).unwrap();
        assert!(exposed(&workspace, Panel::Tools));
        assert!(!exposed(&workspace, Panel::Inspector));
        apply(&mut workspace, Action::Activate { panel: Panel::Inspector }).unwrap();
        assert!(exposed(&workspace, Panel::Inspector));
        apply(&mut workspace, Action::Float { panel: Panel::Tools, rect: [10.0, 20.0, 300.0, 400.0] }).unwrap();
        apply(&mut workspace, Action::Move { panel: Panel::Inspector, anchor: Panel::Tools, placement: Placement::Split(Zone::Center) }).unwrap();
        assert!(!exposed(&workspace, Panel::Tools));
        assert!(exposed(&workspace, Panel::Inspector));
    }

    #[test]
    fn unavailable_inspector_split_resize_targets_saved_tools_split() {
        let mut workspace = default_workspace();
        // Inspector at the outer edge makes the visible Tools split move to the root.
        apply(&mut workspace, Action::Move { panel: Panel::Inspector, anchor: Panel::Tools, placement: Placement::Split(Zone::Left) }).unwrap();
        let mut rendered = workspace.layout.clone();
        rendered.apply(Action::Close { panel: Panel::Inspector }).unwrap();
        let path = saved_split_path(&workspace.layout, &rendered, &[]).unwrap();
        apply(&mut workspace, Action::ResizeSplit { path, size: SplitSize::FixedFirst(290.0) }).unwrap();
        let mut resized = workspace.layout.clone();
        resized.apply(Action::Close { panel: Panel::Inspector }).unwrap();
        assert!(matches!(resized.root, Some(Node::Split { size: SplitSize::FixedFirst(290.0), .. })));
    }

    #[test]
    fn unavailable_outer_inspector_does_not_capture_its_childs_resize() {
        let mut workspace = Workspace::new(Layout {
            root: Some(split(
                SplitAxis::Horizontal,
                SplitSize::FixedSecond(330.0),
                split(SplitAxis::Horizontal, SplitSize::FixedFirst(272.0), tabs(Panel::Tools), tabs(Panel::Canvas)),
                tabs(Panel::Inspector),
            )),
            floating: Vec::new(),
        });
        validate(&workspace).unwrap();
        let mut rendered = workspace.layout.clone();
        rendered.apply(Action::Close { panel: Panel::Inspector }).unwrap();
        let path = saved_split_path(&workspace.layout, &rendered, &[]).unwrap();
        assert_eq!(path, vec![false]);
        apply(&mut workspace, Action::ResizeSplit { path, size: SplitSize::FixedFirst(290.0) }).unwrap();
        assert!(matches!(workspace.layout.root, Some(Node::Split { size: SplitSize::FixedSecond(330.0), .. })));
        let mut resized = workspace.layout.clone();
        resized.apply(Action::Close { panel: Panel::Inspector }).unwrap();
        assert!(matches!(resized.root, Some(Node::Split { size: SplitSize::FixedFirst(290.0), .. })));
    }
}
