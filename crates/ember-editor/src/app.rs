//! The editor window: a node tree like any game (M4.3a panel split).
//!
//! ```text
//! EditorApp                 root: owns the Document, draws the preview
//! ├── Toolbar               Save … Rename, status; class picker, quit dialog
//! ├── Outline               a Button per scene node, Up / Down / In / Out
//! ├── Inspector             a typed editor row per property (scrolls)
//! └── UiRoot                every panel's Control subtree
//! ```
//!
//! Each panel node builds its own `Control` subtree under the `UiRoot`
//! and turns the UI signals it owns (`ButtonClicked`, `TextSubmitted`)
//! into editor signals ([`NodeSelected`], [`PropertyEdited`],
//! [`CommandRequested`]; ADR-036: each carries its sender).
//! `EditorApp` is the one owner of the [`Document`]: it queues those
//! signals, applies them in `process`, then pushes the document state
//! back into the panels. The preview ([`preview`]) is drawn by
//! `EditorApp` and picks nodes by clicking (ADR-040 camera).

pub mod inspector;
pub mod outline;
pub mod preview;
pub mod toolbar;

pub use inspector::{Inspector, Row, RowEditor};
pub use outline::Outline;
pub use preview::{node_rect, pick, preview_camera, preview_rect};
pub use toolbar::Toolbar;

use crate::{Document, Shift};
use ember_core::Variant;
use ember_input::{Input, Key};
use ember_scene::{DrawContext, Node, NodeContext, NodeId, NodeName, NodeTree, Servers};
use ember_servers::{LinearPhysics, MockAudio};
use ember_ui::prelude::*;
use glam::Vec2;

pub const WINDOW_W: f32 = 1100.0;
pub const WINDOW_H: f32 = 700.0;
pub(crate) const TOOLBAR_H: f32 = 40.0;
pub(crate) const OUTLINE_W: f32 = 240.0;
pub(crate) const INSPECTOR_W: f32 = 320.0;

// --- Editor signals (ADR-036: they carry their sender) ------------------

/// The outline asked to select `node`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeSelected {
    pub sender: NodeId,
    pub node: NodeId,
}

/// The inspector asked to change `node.key`.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyEdited {
    pub sender: NodeId,
    pub node: NodeId,
    pub key: &'static str,
    pub edit: Edit,
}

/// One inspector edit; each becomes one undoable `SetProperty`.
#[derive(Debug, Clone, PartialEq)]
pub enum Edit {
    /// A whole value (toggle, enum cycle).
    Set(Variant),
    /// Part `index` of the value from text (`Vec2` x/y, `Color` r/g/b/a;
    /// index 0 of any other kind is the whole value).
    Part { index: usize, text: String },
}

/// A toolbar, outline or dialog action on the document.
#[derive(Debug, Clone, PartialEq)]
pub struct CommandRequested {
    pub sender: NodeId,
    pub command: EditorCommand,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EditorCommand {
    Save,
    Undo,
    Redo,
    Delete,
    /// Open or close the class picker.
    ToggleClasses,
    Add(&'static str),
    Rename(String),
    Shift(Shift),
    SaveAndQuit,
    QuitWithoutSaving,
    CancelQuit,
}

/// What `EditorApp` queued from the editor signals.
#[derive(Debug, Clone, PartialEq)]
enum Request {
    Select(NodeId),
    Edit {
        node: NodeId,
        key: &'static str,
        edit: Edit,
    },
    Command(EditorCommand),
}

/// The panel nodes and the `UiRoot`.
#[derive(Debug, Clone, Copy)]
pub struct Panels {
    pub ui_root: NodeId,
    pub toolbar: NodeId,
    pub outline: NodeId,
    pub inspector: NodeId,
}

pub struct EditorApp {
    name: String,
    pub doc: Document,
    pub selected: Option<NodeId>,
    pub status: String,
    pending: Vec<Request>,
    inspector_stale: bool,
    outline_stale: bool,
    quit_armed: bool,
    panels: Option<Panels>,
    /// The game's resolution, framed in the preview (ADR-039).
    pub viewport: Option<Vec2>,
    // The preview's own servers for the scene's `ready`: silent audio,
    // no physics shared with the editor, no input.
    preview_audio: MockAudio,
    preview_physics: LinearPhysics,
    preview_input: Input,
}

impl EditorApp {
    pub fn new(doc: Document) -> Self {
        Self {
            name: "Editor".into(),
            doc,
            selected: None,
            status: "Click a node to inspect it".into(),
            pending: Vec::new(),
            inspector_stale: true,
            outline_stale: true,
            quit_armed: false,
            panels: None,
            viewport: None,
            preview_audio: MockAudio::new(),
            preview_physics: LinearPhysics::default(),
            preview_input: Input::default(),
        }
    }

    /// Frame the game's resolution in the preview.
    pub fn with_viewport(mut self, viewport: Option<Vec2>) -> Self {
        self.viewport = viewport;
        self
    }

    pub fn panels(&self) -> Option<Panels> {
        self.panels
    }

    pub fn select(&mut self, id: Option<NodeId>) {
        if self.selected != id {
            self.selected = id;
            self.inspector_stale = true;
        }
    }

    fn save(&mut self) -> bool {
        match self.doc.save() {
            Ok(()) => {
                self.status = format!("Saved {}", self.file_name());
                true
            }
            Err(e) => {
                self.status = format!("Save failed: {e}");
                false
            }
        }
    }

    /// The document changed: refresh the outline and inspector, and
    /// drop a selection that was removed.
    fn changed(&mut self) {
        self.outline_stale = true;
        self.inspector_stale = true;
        if self.selected.is_some_and(|id| !self.doc.is_live(id)) {
            self.selected = None;
        }
    }

    /// Report an edit's result in the status line.
    fn report(&mut self, done: &str, r: Result<(), crate::EditorError>) {
        match r {
            Ok(()) => {
                self.status = done.to_string();
                self.changed();
            }
            Err(e) => self.status = e.to_string(),
        }
    }

    pub fn undo(&mut self) {
        match self.doc.undo() {
            Ok(true) => self.report("Undone", Ok(())),
            Ok(false) => self.status = "Nothing to undo".into(),
            Err(e) => self.status = e.to_string(),
        }
    }

    pub fn redo(&mut self) {
        match self.doc.redo() {
            Ok(true) => self.report("Redone", Ok(())),
            Ok(false) => self.status = "Nothing to redo".into(),
            Err(e) => self.status = e.to_string(),
        }
    }

    pub fn delete_selected(&mut self) {
        let Some(id) = self.selected else {
            self.status = "Select a node to delete".into();
            return;
        };
        let parent = self.doc.tree().parent(id);
        let r = self.doc.remove_node(id);
        let ok = r.is_ok();
        self.report("Deleted (Ctrl+Z to undo)", r);
        if ok {
            self.select(parent);
        }
    }

    fn rename_selected(&mut self, name: &str) {
        let Some(id) = self.selected else {
            self.status = "Select a node to rename".into();
            return;
        };
        let r = self.doc.rename_node(id, name.trim());
        self.report(&format!("Renamed to {}", name.trim()), r);
    }

    fn shift_selected(&mut self, shift: Shift) {
        let Some(id) = self.selected else {
            self.status = "Select a node to move".into();
            return;
        };
        let r = self.doc.shift(id, shift);
        self.report("Moved (Ctrl+Z to undo)", r);
    }

    fn add_class(&mut self, class: &str) {
        let parent = self
            .selected
            .filter(|&id| self.doc.is_live(id))
            .unwrap_or_else(|| self.doc.root());
        match self.doc.add_node(parent, class) {
            Ok(id) => {
                self.report(&format!("Added a {class}"), Ok(()));
                self.select(Some(id));
            }
            Err(e) => self.status = e.to_string(),
        }
    }

    fn edit(&mut self, node: NodeId, key: &'static str, edit: Edit) {
        let r = match edit {
            Edit::Set(v) => self.doc.set_property(node, key, &v),
            Edit::Part { index, text } => self.doc.set_property_component(node, key, index, &text),
        };
        self.status = match r {
            Ok(()) => format!(
                "{key} = {}",
                self.doc.property_text(node, key).unwrap_or_default()
            ),
            Err(e) => e.to_string(),
        };
        // Show the value as stored (normalised, or unchanged on error).
        self.inspector_stale = true;
    }

    /// Apply one request; `false` means quit.
    fn apply(&mut self, request: Request, tree: &mut NodeTree, panels: Panels) -> bool {
        let toolbar = |tree: &mut NodeTree| node_mut::<Toolbar>(tree, panels.toolbar).cloned();
        match request {
            Request::Select(node) => {
                self.select(Some(node));
                self.status = "Edit a value, then press Enter".into();
            }
            Request::Edit { node, key, edit } => self.edit(node, key, edit),
            Request::Command(command) => match command {
                EditorCommand::Save => {
                    self.save();
                }
                EditorCommand::Undo => self.undo(),
                EditorCommand::Redo => self.redo(),
                EditorCommand::Delete => self.delete_selected(),
                EditorCommand::Rename(name) => self.rename_selected(&name),
                EditorCommand::Shift(shift) => self.shift_selected(shift),
                EditorCommand::ToggleClasses => {
                    if let Some(t) = toolbar(tree) {
                        let open = !t.picker_open(tree);
                        t.show_picker(tree, open);
                        self.status = if open {
                            "Pick a class to add under the selection".into()
                        } else {
                            String::new()
                        };
                    }
                }
                EditorCommand::Add(class) => {
                    if let Some(t) = toolbar(tree) {
                        t.show_picker(tree, false);
                    }
                    self.add_class(class);
                }
                EditorCommand::SaveAndQuit => {
                    if let Some(t) = toolbar(tree) {
                        t.show_dialog(tree, false);
                    }
                    return !self.save();
                }
                EditorCommand::QuitWithoutSaving => return false,
                EditorCommand::CancelQuit => {
                    if let Some(t) = toolbar(tree) {
                        t.show_dialog(tree, false);
                    }
                    self.status = "Kept editing".into();
                }
            },
        }
        true
    }

    fn file_name(&self) -> String {
        self.doc
            .path()
            .and_then(|p| p.file_name())
            .map_or_else(|| "(unsaved)".into(), |n| n.to_string_lossy().into_owned())
    }

    /// Push document state into the panels.
    fn sync(&mut self, ctx: &mut NodeContext) {
        let Some(p) = self.panels else { return };
        let dirty = if self.doc.is_dirty() { " *" } else { "" };
        let status = format!("{}{dirty} — {}", self.file_name(), self.status);
        let (doc, selected) = (&self.doc, self.selected);
        if let Some(t) = node_mut::<Toolbar>(ctx.tree(), p.toolbar).cloned() {
            t.sync(ctx.tree(), doc, &status);
        }
        let rebuild = std::mem::take(&mut self.outline_stale);
        with_panel::<Outline>(ctx, p.outline, |o, ctx| {
            if rebuild {
                o.rebuild(ctx, doc, selected);
            } else {
                o.refresh(ctx.tree(), doc, selected);
            }
        });
        if std::mem::take(&mut self.inspector_stale) {
            with_panel::<Inspector>(ctx, p.inspector, |i, ctx| i.show(ctx, doc, selected));
            if let Some(t) = node_mut::<Toolbar>(ctx.tree(), p.toolbar).cloned() {
                let name = selected
                    .and_then(|id| doc.tree().get(id))
                    .map(|n| n.name().to_string())
                    .unwrap_or_default();
                t.set_name_text(ctx.tree(), name);
            }
        }
    }

    /// A left click in the preview selects the topmost node under it
    /// (or clears the selection), unless a popup covers the preview.
    fn preview_click(&mut self, input: &Input, tree: &mut NodeTree, panels: Panels) {
        if !input.mouse_left_pressed || !preview_rect().contains(input.mouse_pos) {
            return;
        }
        let covered = node_mut::<Toolbar>(tree, panels.toolbar)
            .cloned()
            .is_some_and(|t| t.popup_open(tree));
        if covered {
            return;
        }
        let hit = pick(&self.doc, self.viewport, input.mouse_pos);
        self.select(hit);
        self.status = match hit {
            Some(_) => "Edit a value, then press Enter".into(),
            None => "Click a node to inspect it".into(),
        };
    }
}

impl ember_core::Object for EditorApp {
    fn class_name() -> &'static str {
        "EditorApp"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl NodeName for EditorApp {
    fn name(&self) -> &str {
        &self.name
    }
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

impl Node for EditorApp {
    fn ready(&mut self, ctx: &mut NodeContext) {
        let me = ctx.current_id();
        ctx.connect::<NodeSelected>(me, |ctx, ev| {
            queue(ctx, Request::Select(ev.node));
        });
        ctx.connect::<PropertyEdited>(me, |ctx, ev| {
            queue(
                ctx,
                Request::Edit {
                    node: ev.node,
                    key: ev.key,
                    edit: ev.edit.clone(),
                },
            );
        });
        ctx.connect::<CommandRequested>(me, |ctx, ev| {
            queue(ctx, Request::Command(ev.command.clone()));
        });
        let mut servers = Servers {
            rendering: ctx.render(),
            audio: &mut self.preview_audio,
            physics: &mut self.preview_physics,
            input: &self.preview_input,
        };
        self.doc.ready(&mut servers);
        self.sync(ctx);
    }

    fn process(&mut self, _dt: f32, ctx: &mut NodeContext) {
        let Some(panels) = self.panels else { return };
        let input = ctx.input().clone();
        let down = |keys: &[Key]| keys.iter().any(|&k| input.is_key_down(k));
        let ctrl = down(&[
            Key::LeftControl,
            Key::RightControl,
            Key::LeftSuper,
            Key::RightSuper,
        ]);
        let shift = down(&[Key::LeftShift, Key::RightShift]);
        let pressed = |k| input.is_key_pressed(k);
        let (save, undo, redo) = (
            ctrl && pressed(Key::S),
            ctrl && pressed(Key::Z) && !shift,
            ctrl && (pressed(Key::Y) || (pressed(Key::Z) && shift)),
        );
        let delete = pressed(Key::Delete);
        let close = pressed(Key::Escape) || input.close_requested;
        let tree = ctx.tree();
        let typing = node_mut::<UiRoot>(tree, panels.ui_root)
            .and_then(|r| r.focused())
            .is_some_and(|f| tree.get(f).is_some_and(|n| n.as_any().is::<TextInput>()));

        if close {
            if !self.doc.is_dirty() || self.quit_armed {
                ctx.quit();
                return;
            }
            self.quit_armed = true;
            if let Some(t) = node_mut::<Toolbar>(tree, panels.toolbar).cloned() {
                t.show_dialog(tree, true);
            }
            self.status = "Unsaved changes: Escape again to quit without saving".into();
        }
        if save {
            self.save();
        }
        if undo {
            self.undo();
        }
        if redo {
            self.redo();
        }
        if delete && !typing {
            self.delete_selected();
        }
        self.preview_click(&input, tree, panels);
        for request in std::mem::take(&mut self.pending) {
            self.quit_armed = false;
            if !self.apply(request, tree, panels) {
                ctx.quit();
                return;
            }
        }
        self.sync(ctx);
    }

    fn draw(&self, ctx: &mut DrawContext) {
        preview::draw(ctx, &self.doc, self.selected, self.viewport);
    }
}

/// Queue a request on the `EditorApp` the handler runs on.
fn queue(ctx: &mut NodeContext, request: Request) {
    let me = ctx.current_id();
    if let Some(app) = node_mut::<EditorApp>(ctx.tree(), me) {
        app.pending.push(request);
    }
}

// --- Tree helpers ------------------------------------------------------

pub(crate) fn node_mut<T: 'static>(tree: &mut NodeTree, id: NodeId) -> Option<&mut T> {
    tree.get_mut(id)?.as_any_mut().downcast_mut::<T>()
}

/// Run `f` on a copy of panel `P`, then store it back (the panel is a
/// node of the same tree `f` edits).
pub(crate) fn with_panel<P: Clone + 'static>(
    ctx: &mut NodeContext,
    id: NodeId,
    f: impl FnOnce(&mut P, &mut NodeContext),
) {
    let Some(mut panel) = node_mut::<P>(ctx.tree(), id).cloned() else {
        return;
    };
    f(&mut panel, ctx);
    if let Some(slot) = node_mut::<P>(ctx.tree(), id) {
        *slot = panel;
    }
}

pub(crate) fn set_label(tree: &mut NodeTree, id: NodeId, text: impl Into<String>) {
    if let Some(label) = node_mut::<Label>(tree, id) {
        let text = text.into();
        if label.text != text {
            label.set_text(text);
        }
    }
}

pub(crate) fn set_button(tree: &mut NodeTree, id: NodeId, text: impl Into<String>) {
    if let Some(b) = node_mut::<Button>(tree, id) {
        let text = text.into();
        if b.text != text {
            b.set_text(text);
        }
    }
}

pub(crate) fn set_field(tree: &mut NodeTree, id: NodeId, text: impl Into<String>) {
    if let Some(f) = node_mut::<TextInput>(tree, id) {
        let text = text.into();
        if f.text() != text {
            f.set_text(text);
        }
    }
}

pub(crate) fn set_visible(tree: &mut NodeTree, id: NodeId, visible: bool) {
    if let Some(row) = node_mut::<HBox>(tree, id) {
        row.base_mut().visible = visible;
    } else if let Some(panel) = node_mut::<Panel>(tree, id) {
        panel.base_mut().visible = visible;
    }
}

pub(crate) fn is_shown(tree: &NodeTree, id: NodeId) -> bool {
    tree.get(id).is_some_and(|n| n.visible())
}

pub(crate) fn add(tree: &mut NodeTree, parent: NodeId, node: impl Node) -> NodeId {
    let id = tree.insert(node);
    tree.attach(id, parent).expect("attach");
    id
}

/// The panel node of type `P` (a child of the editor's root), for tests
/// and tools.
pub fn panel<P: 'static>(tree: &NodeTree) -> Option<&P> {
    let root = tree.root()?;
    tree.children(root)
        .iter()
        .find_map(|&c| tree.get(c)?.as_any().downcast_ref::<P>())
}

/// The editor's node tree for `doc`.
pub fn build_tree(doc: Document) -> NodeTree {
    build_app_tree(EditorApp::new(doc))
}

/// Like [`build_tree`], for an `EditorApp` already configured
/// (for example [`EditorApp::with_viewport`]).
pub fn build_app_tree(app: EditorApp) -> NodeTree {
    let classes = app.doc.addable_classes();
    let mut tree = NodeTree::with_root(app);
    let root = tree.root().expect("root");
    let ui_root = add(&mut tree, root, UiRoot::new(Theme::dark()));
    let theme = Theme::dark();
    let mut toolbar = Toolbar::build(&mut tree, ui_root);
    let outline = Outline::build(&mut tree, ui_root);
    let inspector = Inspector::build(&mut tree, ui_root, &theme);
    // Popups go last so they draw over the panels.
    toolbar.build_popups(&mut tree, ui_root, &theme, &classes);
    let panels = Panels {
        ui_root,
        toolbar: add(&mut tree, root, toolbar),
        outline: add(&mut tree, root, outline),
        inspector: add(&mut tree, root, inspector),
    };
    if let Some(app) = node_mut::<EditorApp>(&mut tree, root) {
        app.panels = Some(panels);
    }
    tree
}
