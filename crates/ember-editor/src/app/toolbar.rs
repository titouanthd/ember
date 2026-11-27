//! The toolbar (Save, Undo, Redo, Add, Delete, name + Rename, status)
//! and the two popups it opens: the class picker and the quit dialog.

use super::{
    CommandRequested, EditorCommand, OUTLINE_W, TOOLBAR_H, WINDOW_H, WINDOW_W, add, is_shown,
    node_mut, set_button, set_field, set_label, set_visible,
};
use crate::Document;
use ember_macros::Node;
use ember_scene::{NodeContext, NodeId, NodeTree};
use ember_ui::prelude::*;
use glam::Vec2;

const NAME_W: f32 = 140.0;
const PICKER_W: f32 = 180.0;
const DIALOG_W: f32 = 420.0;

/// The toolbar panel node: its widget ids, and the signals it sends.
#[derive(Node, Clone, Debug)]
#[class_name = "EditorToolbar"]
pub struct Toolbar {
    pub name: String,
    pub save: NodeId,
    pub undo: NodeId,
    pub redo: NodeId,
    pub add: NodeId,
    pub delete: NodeId,
    /// The selected node's name; Enter (or `rename`) renames it.
    pub name_input: NodeId,
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
}

impl Toolbar {
    /// The toolbar strip under `ui_root`. The popups come later
    /// ([`build_popups`](Self::build_popups)) so they draw on top.
    pub fn build(tree: &mut NodeTree, ui_root: NodeId) -> Self {
        let panel = add(
            tree,
            ui_root,
            Panel::new()
                .with_padding(Margins::all(4.0))
                .with_anchors(Anchors::TOP_WIDE)
                .with_min_size(Vec2::new(0.0, TOOLBAR_H)),
        );
        let bar = add(
            tree,
            panel,
            HBox::new()
                .with_spacing(12.0)
                .with_anchors(Anchors::FULL_RECT),
        );
        let save = add(tree, bar, Button::new("Save"));
        let undo = add(tree, bar, Button::new("Undo"));
        let redo = add(tree, bar, Button::new("Redo"));
        let add_button = add(tree, bar, Button::new("Add"));
        let delete = add(tree, bar, Button::new("Delete"));
        let name_input = add(
            tree,
            bar,
            TextInput::new()
                .with_placeholder("name")
                .with_min_size(Vec2::new(NAME_W, 0.0)),
        );
        let rename = add(tree, bar, Button::new("Rename"));
        let status = add(tree, bar, Label::new(""));
        Self {
            name: "Toolbar".into(),
            save,
            undo,
            redo,
            add: add_button,
            delete,
            name_input,
            rename,
            status,
            classes: panel,
            class_buttons: Vec::new(),
            dialog: panel,
            save_quit: panel,
            discard_quit: panel,
            cancel_quit: panel,
        }
    }

    /// The class picker (over the outline's right edge) and the quit
    /// dialog, both hidden.
    pub fn build_popups(
        &mut self,
        tree: &mut NodeTree,
        ui_root: NodeId,
        theme: &Theme,
        classes: &[&'static str],
    ) {
        self.classes = add(
            tree,
            ui_root,
            Panel::new()
                .with_padding(Margins::all(8.0))
                .with_offset(Vec2::new(OUTLINE_W, TOOLBAR_H))
                .with_min_size(Vec2::new(PICKER_W, 0.0))
                .hidden(),
        );
        let picker = add(tree, self.classes, VBox::new().with_spacing(2.0));
        add(
            tree,
            picker,
            Label::new("Add under the selection").with_color(theme.text_dim),
        );
        self.class_buttons = classes
            .iter()
            .map(|&c| {
                (
                    add(tree, picker, Button::new(c).with_align(Alignment::Left)),
                    c,
                )
            })
            .collect();

        self.dialog = add(
            tree,
            ui_root,
            Panel::new()
                .with_padding(Margins::all(16.0))
                .with_offset(Vec2::new(WINDOW_W / 2.0 - DIALOG_W / 2.0, WINDOW_H / 3.0))
                .with_min_size(Vec2::new(DIALOG_W, 0.0))
                .hidden(),
        );
        let column = add(tree, self.dialog, VBox::new().with_spacing(10.0));
        add(tree, column, Label::new("The scene has unsaved changes."));
        let row = add(tree, column, HBox::new().with_spacing(8.0));
        self.save_quit = add(tree, row, Button::new("Save and quit"));
        self.discard_quit = add(tree, row, Button::new("Quit without saving"));
        self.cancel_quit = add(tree, row, Button::new("Cancel"));
    }

    /// The command a click on `button` asks for, if it is one of ours.
    pub fn on_click(&self, tree: &mut NodeTree, button: NodeId) -> Option<EditorCommand> {
        use EditorCommand as C;
        let fixed = [
            (self.save, C::Save),
            (self.undo, C::Undo),
            (self.redo, C::Redo),
            (self.delete, C::Delete),
            (self.add, C::ToggleClasses),
            (self.save_quit, C::SaveAndQuit),
            (self.discard_quit, C::QuitWithoutSaving),
            (self.cancel_quit, C::CancelQuit),
        ];
        if let Some((_, c)) = fixed.into_iter().find(|(b, _)| *b == button) {
            return Some(c);
        }
        if button == self.rename {
            let text = node_mut::<TextInput>(tree, self.name_input)
                .map(|t| t.text().to_string())
                .unwrap_or_default();
            return Some(C::Rename(text));
        }
        let &(_, class) = self.class_buttons.iter().find(|&&(b, _)| b == button)?;
        Some(C::Add(class))
    }

    pub fn picker_open(&self, tree: &NodeTree) -> bool {
        is_shown(tree, self.classes)
    }

    /// The picker or the dialog is open (they cover the preview).
    pub fn popup_open(&self, tree: &NodeTree) -> bool {
        is_shown(tree, self.classes) || is_shown(tree, self.dialog)
    }

    pub fn show_picker(&self, tree: &mut NodeTree, open: bool) {
        set_visible(tree, self.classes, open);
    }

    pub fn show_dialog(&self, tree: &mut NodeTree, open: bool) {
        set_visible(tree, self.dialog, open);
    }

    pub fn set_name_text(&self, tree: &mut NodeTree, name: String) {
        set_field(tree, self.name_input, name);
    }

    /// Status line, "Save *" while dirty, Undo / Redo greyed when empty.
    pub fn sync(&self, tree: &mut NodeTree, doc: &Document, status: &str) {
        set_label(tree, self.status, status);
        set_button(
            tree,
            self.save,
            if doc.is_dirty() { "Save *" } else { "Save" },
        );
        for (id, on) in [(self.undo, doc.can_undo()), (self.redo, doc.can_redo())] {
            if let Some(b) = node_mut::<Button>(tree, id)
                && b.is_disabled() == on
            {
                b.set_disabled(!on);
            }
        }
    }
}

impl ember_scene::Node for Toolbar {
    fn ready(&mut self, ctx: &mut NodeContext) {
        let me = ctx.current_id();
        ctx.connect::<ButtonClicked>(me, |ctx, ev| {
            let me = ctx.current_id();
            let Some(bar) = node_mut::<Toolbar>(ctx.tree(), me).cloned() else {
                return;
            };
            if let Some(command) = bar.on_click(ctx.tree(), ev.button) {
                ctx.emit(CommandRequested {
                    sender: me,
                    command,
                });
            }
        });
        ctx.connect::<TextSubmitted>(me, |ctx, ev| {
            let me = ctx.current_id();
            let ours =
                node_mut::<Toolbar>(ctx.tree(), me).is_some_and(|t| t.name_input == ev.input);
            if ours {
                ctx.emit(CommandRequested {
                    sender: me,
                    command: EditorCommand::Rename(ev.text.clone()),
                });
            }
        });
    }
}
