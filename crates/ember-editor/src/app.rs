//! The editor window (session 6.2): a node tree like any game.
//!
//! ```text
//! EditorApp                 root: owns the Document, draws the preview
//! └── UiRoot
//!     ├── toolbar           Save button, status line
//!     ├── outline           a Button per scene node (in a ScrollContainer)
//!     └── inspector         header + one Label/TextInput row per property
//! ```
//!
//! UI signals (`ButtonClicked`, `TextSubmitted`, ADR-036) only queue an
//! [`Intent`]; `EditorApp::process` applies it with the whole tree at
//! hand and then pushes the document state back into the widgets.

use crate::Document;
use crate::value;
use ember_core::Color;
use ember_core::math::Rect;
use ember_input::{Input, Key};
use ember_scene::{DrawContext, Node, NodeContext, NodeId, NodeName, NodeTree, Servers};
use ember_servers::{Camera2d, LinearPhysics, MockAudio};
use ember_ui::prelude::*;
use glam::Vec2;

pub const WINDOW_W: f32 = 1100.0;
pub const WINDOW_H: f32 = 700.0;
const TOOLBAR_H: f32 = 40.0;
const OUTLINE_W: f32 = 240.0;
const INSPECTOR_W: f32 = 320.0;
const KEY_W: f32 = 84.0;
const NAME_W: f32 = 140.0;
const PICKER_W: f32 = 180.0;
const DIALOG_W: f32 = 420.0;
/// Rows the inspector can show; extra properties are listed as "+N more".
pub const MAX_PROPERTIES: usize = 16;
/// Scene origin inside the preview.
const PREVIEW_MARGIN: f32 = 20.0;

/// The camera that fits a scene into the preview (ADR-040). With the
/// game's `viewport`, the whole resolution is scaled down to fit (never
/// up) and centred; without, the scene's origin sits near the top-left
/// at zoom 1.
pub fn preview_camera(viewport: Option<Vec2>) -> Camera2d {
    let area = preview_rect();
    let corner = Vec2::new(area.x, area.y);
    let Some(size) = viewport.filter(|s| s.x > 0.0 && s.y > 0.0) else {
        return Camera2d::placing_origin_at(corner + Vec2::splat(PREVIEW_MARGIN), 1.0);
    };
    let room = Vec2::new(area.w, area.h) - Vec2::splat(2.0 * PREVIEW_MARGIN);
    let zoom = (room.x / size.x).min(room.y / size.y).min(1.0);
    let origin = corner + (Vec2::new(area.w, area.h) - size * zoom) / 2.0;
    Camera2d::placing_origin_at(origin, zoom)
}

/// The preview area: between the outline and the inspector.
pub fn preview_rect() -> Rect {
    Rect::new(
        OUTLINE_W,
        TOOLBAR_H,
        WINDOW_W - OUTLINE_W - INSPECTOR_W,
        WINDOW_H - TOOLBAR_H,
    )
}

/// Something a UI signal asked for; applied in `EditorApp::process`.
#[derive(Debug, Clone, PartialEq)]
pub enum Intent {
    Click(NodeId),
    Submit { input: NodeId, text: String },
}

/// Ids of the widgets `EditorApp` updates.
#[derive(Debug, Clone)]
pub struct Widgets {
    pub ui_root: NodeId,
    pub save: NodeId,
    pub undo: NodeId,
    pub redo: NodeId,
    pub add: NodeId,
    pub delete: NodeId,
    /// The selected node's name; Enter (or `rename`) renames it.
    pub name: NodeId,
    pub rename: NodeId,
    pub status: NodeId,
    /// The class picker `add` opens: the panel and `(button, class)`.
    pub classes: NodeId,
    pub class_buttons: Vec<(NodeId, &'static str)>,
    /// The unsaved-changes dialog and its buttons.
    pub dialog: NodeId,
    pub save_quit: NodeId,
    pub discard_quit: NodeId,
    pub cancel_quit: NodeId,
    /// The outline's button list, rebuilt after structural edits.
    pub outline_list: NodeId,
    /// `(button, scene node)` per outline row, in outline order.
    pub outline: Vec<(NodeId, NodeId)>,
    pub header: NodeId,
    pub more: NodeId,
    /// `(row, key label, value input)`.
    pub rows: Vec<(NodeId, NodeId, NodeId)>,
}

pub struct EditorApp {
    name: String,
    pub doc: Document,
    pub selected: Option<NodeId>,
    /// Property key shown in each inspector row.
    row_keys: Vec<Option<&'static str>>,
    pub status: String,
    pending: Vec<Intent>,
    inspector_stale: bool,
    outline_stale: bool,
    quit_armed: bool,
    ui: Option<Widgets>,
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
            row_keys: vec![None; MAX_PROPERTIES],
            status: "Click a node to inspect it".into(),
            pending: Vec::new(),
            inspector_stale: true,
            outline_stale: false,
            quit_armed: false,
            ui: None,
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

    pub fn widgets(&self) -> Option<&Widgets> {
        self.ui.as_ref()
    }

    /// Queue an intent (what the signal handlers do).
    pub fn push(&mut self, intent: Intent) {
        self.pending.push(intent);
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

    fn apply(&mut self, intent: Intent, tree: &mut NodeTree) -> bool {
        let Some(ui) = self.ui.clone() else {
            return true;
        };
        match intent {
            Intent::Click(button) if button == ui.save => {
                self.save();
            }
            Intent::Click(button) if button == ui.undo => self.undo(),
            Intent::Click(button) if button == ui.redo => self.redo(),
            Intent::Click(button) if button == ui.delete => self.delete_selected(),
            Intent::Click(button) if button == ui.add => {
                let open = !is_shown(tree, ui.classes);
                set_visible(tree, ui.classes, open);
                self.status = if open {
                    "Pick a class to add under the selection".into()
                } else {
                    String::new()
                };
            }
            Intent::Click(button) if button == ui.rename => {
                let text = node_mut::<TextInput>(tree, ui.name)
                    .map(|t| t.text().to_string())
                    .unwrap_or_default();
                self.rename_selected(&text);
            }
            Intent::Click(button) if button == ui.save_quit => {
                set_visible(tree, ui.dialog, false);
                return !self.save();
            }
            Intent::Click(button) if button == ui.discard_quit => return false,
            Intent::Click(button) if button == ui.cancel_quit => {
                set_visible(tree, ui.dialog, false);
                self.status = "Kept editing".into();
            }
            Intent::Click(button) if ui.class_buttons.iter().any(|&(b, _)| b == button) => {
                let class = ui.class_buttons.iter().find(|&&(b, _)| b == button);
                set_visible(tree, ui.classes, false);
                if let Some(&(_, class)) = class {
                    self.add_class(class);
                }
            }
            Intent::Submit { input, text } if input == ui.name => self.rename_selected(&text),
            Intent::Click(button) => {
                if let Some(&(_, node)) = ui.outline.iter().find(|(b, _)| *b == button) {
                    self.select(Some(node));
                    self.status = "Edit a value, then press Enter".into();
                }
            }
            Intent::Submit { input, text } => {
                let Some(row) = ui.rows.iter().position(|&(_, _, i)| i == input) else {
                    return true;
                };
                let (Some(node), Some(key)) = (self.selected, self.row_keys[row]) else {
                    return true;
                };
                self.status = match self.doc.set_property_text(node, key, &text) {
                    Ok(()) => format!(
                        "{key} = {}",
                        self.doc.property_text(node, key).unwrap_or_default()
                    ),
                    Err(e) => e.to_string(),
                };
                // Show the value as stored (normalised, or unchanged on error).
                if let (Ok(shown), Some(field)) = (
                    self.doc.property_text(node, key),
                    node_mut::<TextInput>(tree, input),
                ) {
                    field.set_text(shown);
                }
            }
        }
        true
    }

    fn outline_text(&self, node: NodeId, depth: usize, name: &str) -> String {
        let mark = if self.selected == Some(node) {
            "> "
        } else {
            "  "
        };
        format!("{mark}{}{name}", "    ".repeat(depth))
    }

    /// Replace the outline's buttons with one per live node.
    fn rebuild_outline(&mut self, ctx: &mut NodeContext, ui: &Widgets) {
        for &(button, _) in &ui.outline {
            ctx.despawn(button);
        }
        let outline = self
            .doc
            .outline()
            .iter()
            .map(|row| {
                let text = self.outline_text(row.id, row.depth, &row.name);
                let b = ctx.spawn_child(
                    ui.outline_list,
                    Button::new(text).with_align(Alignment::Left),
                );
                (b, row.id)
            })
            .collect();
        if let Some(w) = self.ui.as_mut() {
            w.outline = outline;
        }
    }

    fn file_name(&self) -> String {
        self.doc
            .path()
            .and_then(|p| p.file_name())
            .map_or_else(|| "(unsaved)".into(), |n| n.to_string_lossy().into_owned())
    }

    /// Push document state into the widgets.
    fn sync(&mut self, tree: &mut NodeTree) {
        let Some(ui) = self.ui.clone() else { return };
        let dirty = if self.doc.is_dirty() { " *" } else { "" };
        set_label(
            tree,
            ui.status,
            format!("{}{dirty} — {}", self.file_name(), self.status),
        );
        if let Some(b) = node_mut::<Button>(tree, ui.save) {
            b.set_text(if self.doc.is_dirty() {
                "Save *"
            } else {
                "Save"
            });
        }
        let (can_undo, can_redo) = (self.doc.can_undo(), self.doc.can_redo());
        for (id, on) in [(ui.undo, can_undo), (ui.redo, can_redo)] {
            if let Some(b) = node_mut::<Button>(tree, id)
                && b.is_disabled() == on
            {
                b.set_disabled(!on);
            }
        }
        let outline = self.doc.outline();
        for (&(button, node), row) in ui.outline.iter().zip(&outline) {
            let text = self.outline_text(node, row.depth, &row.name);
            if let Some(b) = node_mut::<Button>(tree, button)
                && b.text != text
            {
                b.set_text(text);
            }
        }
        if self.inspector_stale {
            self.inspector_stale = false;
            self.fill_inspector(tree, &ui);
        }
    }

    fn fill_inspector(&mut self, tree: &mut NodeTree, ui: &Widgets) {
        let props = self
            .selected
            .and_then(|id| self.doc.properties(id).ok())
            .unwrap_or_default();
        let header = match self.selected {
            Some(id) => {
                let name = self
                    .doc
                    .tree()
                    .get(id)
                    .map(|n| n.name().to_string())
                    .unwrap_or_default();
                format!("{name} ({})", self.doc.class_of(id).unwrap_or("?"))
            }
            None => "Nothing selected".into(),
        };
        set_label(tree, ui.header, header);
        let name = self
            .selected
            .and_then(|id| self.doc.tree().get(id))
            .map(|n| n.name().to_string())
            .unwrap_or_default();
        if let Some(field) = node_mut::<TextInput>(tree, ui.name) {
            field.set_text(name);
        }
        for (i, &(row, key_label, input)) in ui.rows.iter().enumerate() {
            let prop = props.get(i);
            self.row_keys[i] = prop.map(|(k, _)| *k);
            set_visible(tree, row, prop.is_some());
            if let Some((key, v)) = prop {
                set_label(tree, key_label, *key);
                if let Some(field) = node_mut::<TextInput>(tree, input) {
                    field.set_text(value::format(v));
                }
            }
        }
        let extra = props.len().saturating_sub(MAX_PROPERTIES);
        set_label(
            tree,
            ui.more,
            if extra > 0 {
                format!("+{extra} more")
            } else {
                String::new()
            },
        );
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
        ctx.connect::<ButtonClicked>(me, |ctx, ev| {
            let me = ctx.current_id();
            if let Some(app) = node_mut::<EditorApp>(ctx.tree(), me) {
                app.push(Intent::Click(ev.button));
            }
        });
        ctx.connect::<TextSubmitted>(me, |ctx, ev| {
            let me = ctx.current_id();
            if let Some(app) = node_mut::<EditorApp>(ctx.tree(), me) {
                app.push(Intent::Submit {
                    input: ev.input,
                    text: ev.text.clone(),
                });
            }
        });
        let mut servers = Servers {
            rendering: ctx.render(),
            audio: &mut self.preview_audio,
            physics: &mut self.preview_physics,
            input: &self.preview_input,
        };
        self.doc.ready(&mut servers);
        self.sync(ctx.tree());
    }

    fn process(&mut self, _dt: f32, ctx: &mut NodeContext) {
        let Some(ui) = self.ui.clone() else { return };
        let input = ctx.input();
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
        let typing = node_mut::<UiRoot>(tree, ui.ui_root)
            .and_then(|r| r.focused())
            .is_some_and(|f| tree.get(f).is_some_and(|n| n.as_any().is::<TextInput>()));

        if close {
            if !self.doc.is_dirty() || self.quit_armed {
                ctx.quit();
                return;
            }
            self.quit_armed = true;
            set_visible(tree, ui.dialog, true);
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
        for intent in std::mem::take(&mut self.pending) {
            self.quit_armed = false;
            if !self.apply(intent, tree) {
                ctx.quit();
                return;
            }
        }
        if self.outline_stale {
            self.outline_stale = false;
            self.rebuild_outline(ctx, &ui);
        }
        self.sync(ctx.tree());
    }

    /// The preview: the scene's own nodes, drawn inside a clipped
    /// viewport with the scene origin near its top-left corner.
    fn draw(&self, ctx: &mut DrawContext) {
        let area = preview_rect();
        let r = ctx.render();
        r.draw_rect(area, Color::new(0.06, 0.06, 0.08, 1.0));
        r.push_clip_rect(area);
        let camera = r.camera();
        r.set_camera(preview_camera(self.viewport));
        if let Some(size) = self.viewport {
            r.draw_rect(Rect::new(0.0, 0.0, size.x, size.y), Color::BLACK);
        }
        self.doc.tree().draw(ctx);
        if let Some(size) = self.viewport {
            ctx.render().draw_rect_outline(
                Rect::new(0.0, 0.0, size.x, size.y),
                1.0,
                Color::new(1.0, 1.0, 1.0, 0.25),
            );
        }
        if let Some(rect) = self.selected.and_then(|id| selection_rect(&self.doc, id)) {
            ctx.render()
                .draw_rect_outline(rect, 2.0, Color::new(1.0, 0.8, 0.2, 1.0));
        }
        let r = ctx.render();
        r.set_camera(camera);
        r.pop_clip_rect();
    }
}

/// The rect a selected node covers, for the preview highlight.
fn selection_rect(doc: &Document, id: NodeId) -> Option<Rect> {
    let props = doc.properties(id).ok()?;
    let get = |k: &str| {
        props
            .iter()
            .find(|(key, _)| *key == k)
            .and_then(|(_, v)| v.as_vec2())
    };
    let pos = get("position").unwrap_or(Vec2::ZERO);
    let size = get("size").unwrap_or(Vec2::new(12.0, 12.0));
    let pos = if get("size").is_some() {
        pos
    } else {
        pos - size * 0.5
    };
    Some(Rect::new(
        pos.x - 2.0,
        pos.y - 2.0,
        size.x + 4.0,
        size.y + 4.0,
    ))
}

// --- Tree helpers ------------------------------------------------------

fn node_mut<T: 'static>(tree: &mut NodeTree, id: NodeId) -> Option<&mut T> {
    tree.get_mut(id)?.as_any_mut().downcast_mut::<T>()
}

fn set_label(tree: &mut NodeTree, id: NodeId, text: impl Into<String>) {
    if let Some(label) = node_mut::<Label>(tree, id) {
        let text = text.into();
        if label.text != text {
            label.set_text(text);
        }
    }
}

fn set_visible(tree: &mut NodeTree, id: NodeId, visible: bool) {
    if let Some(row) = node_mut::<HBox>(tree, id) {
        row.base_mut().visible = visible;
    } else if let Some(panel) = node_mut::<Panel>(tree, id) {
        panel.base_mut().visible = visible;
    }
}

fn is_shown(tree: &NodeTree, id: NodeId) -> bool {
    tree.get(id).is_some_and(|n| n.visible())
}

fn add(tree: &mut NodeTree, parent: NodeId, node: impl Node) -> NodeId {
    let id = tree.insert(node);
    tree.attach(id, parent).expect("attach");
    id
}

/// The editor's node tree for `doc`.
pub fn build_tree(doc: Document) -> NodeTree {
    build_app_tree(EditorApp::new(doc))
}

/// Like [`build_tree`], for an `EditorApp` already configured
/// (for example [`EditorApp::with_viewport`]).
pub fn build_app_tree(app: EditorApp) -> NodeTree {
    let outline_rows = app.doc.outline();
    let classes = app.doc.addable_classes();
    let mut tree = NodeTree::with_root(app);
    let root = tree.root().expect("root");
    let ui = add(&mut tree, root, UiRoot::new(Theme::dark()));
    let theme = Theme::dark();

    // Toolbar
    let toolbar = add(
        &mut tree,
        ui,
        Panel::new()
            .with_padding(Margins::all(4.0))
            .with_anchors(Anchors::TOP_WIDE)
            .with_min_size(Vec2::new(0.0, TOOLBAR_H)),
    );
    let bar = add(
        &mut tree,
        toolbar,
        HBox::new()
            .with_spacing(12.0)
            .with_anchors(Anchors::FULL_RECT),
    );
    let save = add(&mut tree, bar, Button::new("Save"));
    let undo = add(&mut tree, bar, Button::new("Undo"));
    let redo = add(&mut tree, bar, Button::new("Redo"));
    let add_button = add(&mut tree, bar, Button::new("Add"));
    let delete = add(&mut tree, bar, Button::new("Delete"));
    let name = add(
        &mut tree,
        bar,
        TextInput::new()
            .with_placeholder("name")
            .with_min_size(Vec2::new(NAME_W, 0.0)),
    );
    let rename = add(&mut tree, bar, Button::new("Rename"));
    let status = add(&mut tree, bar, Label::new(""));

    // Outline
    let outline_panel = add(
        &mut tree,
        ui,
        Panel::new()
            .with_padding(Margins::all(6.0))
            .with_offset(Vec2::new(0.0, TOOLBAR_H))
            .with_min_size(Vec2::new(OUTLINE_W, WINDOW_H - TOOLBAR_H)),
    );
    let scroll = add(
        &mut tree,
        outline_panel,
        ScrollContainer::new().with_anchors(Anchors::FULL_RECT),
    );
    let outline_list = add(&mut tree, scroll, VBox::new().with_spacing(2.0));
    let list = outline_list;
    let outline = outline_rows
        .iter()
        .map(|row| {
            let b = add(
                &mut tree,
                list,
                Button::new(row.name.clone()).with_align(Alignment::Left),
            );
            (b, row.id)
        })
        .collect();

    // Inspector
    let inspector_panel = add(
        &mut tree,
        ui,
        Panel::new()
            .with_padding(Margins::all(10.0))
            .with_offset(Vec2::new(WINDOW_W - INSPECTOR_W, TOOLBAR_H))
            .with_min_size(Vec2::new(INSPECTOR_W, WINDOW_H - TOOLBAR_H)),
    );
    let column = add(
        &mut tree,
        inspector_panel,
        VBox::new()
            .with_spacing(6.0)
            .with_anchors(Anchors::FULL_RECT),
    );
    let header = add(
        &mut tree,
        column,
        Label::new("").with_size(theme.font_size_large),
    );
    add(
        &mut tree,
        column,
        Label::new("Enter applies a value").with_color(theme.text_dim),
    );
    let rows = (0..MAX_PROPERTIES)
        .map(|_| {
            let row = add(&mut tree, column, HBox::new().with_spacing(8.0).hidden());
            let key = add(
                &mut tree,
                row,
                Label::new("").with_min_size(Vec2::new(KEY_W, 0.0)),
            );
            let input = add(&mut tree, row, TextInput::new());
            (row, key, input)
        })
        .collect();
    let more = add(&mut tree, column, Label::new("").with_color(theme.text_dim));

    // Class picker (Add), over the outline.
    let classes_panel = add(
        &mut tree,
        ui,
        Panel::new()
            .with_padding(Margins::all(8.0))
            .with_offset(Vec2::new(OUTLINE_W, TOOLBAR_H))
            .with_min_size(Vec2::new(PICKER_W, 0.0))
            .hidden(),
    );
    let picker = add(&mut tree, classes_panel, VBox::new().with_spacing(2.0));
    add(
        &mut tree,
        picker,
        Label::new("Add under the selection").with_color(theme.text_dim),
    );
    let class_buttons = classes
        .iter()
        .map(|&c| {
            let b = add(
                &mut tree,
                picker,
                Button::new(c).with_align(Alignment::Left),
            );
            (b, c)
        })
        .collect();

    // Unsaved-changes dialog.
    let dialog = add(
        &mut tree,
        ui,
        Panel::new()
            .with_padding(Margins::all(16.0))
            .with_offset(Vec2::new(WINDOW_W / 2.0 - DIALOG_W / 2.0, WINDOW_H / 3.0))
            .with_min_size(Vec2::new(DIALOG_W, 0.0))
            .hidden(),
    );
    let dialog_column = add(&mut tree, dialog, VBox::new().with_spacing(10.0));
    add(
        &mut tree,
        dialog_column,
        Label::new("The scene has unsaved changes."),
    );
    let dialog_row = add(&mut tree, dialog_column, HBox::new().with_spacing(8.0));
    let save_quit = add(&mut tree, dialog_row, Button::new("Save and quit"));
    let discard_quit = add(&mut tree, dialog_row, Button::new("Quit without saving"));
    let cancel_quit = add(&mut tree, dialog_row, Button::new("Cancel"));

    let widgets = Widgets {
        ui_root: ui,
        save,
        undo,
        redo,
        add: add_button,
        delete,
        name,
        rename,
        status,
        classes: classes_panel,
        class_buttons,
        dialog,
        save_quit,
        discard_quit,
        cancel_quit,
        outline_list,
        outline,
        header,
        more,
        rows,
    };
    if let Some(app) = node_mut::<EditorApp>(&mut tree, root) {
        app.ui = Some(widgets);
    }
    tree
}
