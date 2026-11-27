//! The outline: a button per scene node (rebuilt after structural
//! edits) and Up / Down / In / Out buttons that move the selection
//! (`Document::shift`, one undoable `Reparent` each).

use super::{
    CommandRequested, EditorCommand, NodeSelected, OUTLINE_W, TOOLBAR_H, WINDOW_H, add, node_mut,
    set_button,
};
use crate::{Document, Shift};
use ember_macros::Node;
use ember_scene::{NodeContext, NodeId, NodeTree};
use ember_ui::prelude::*;
use glam::Vec2;

const MOVES_H: f32 = 30.0;

#[derive(Node, Clone, Debug)]
#[class_name = "EditorOutline"]
pub struct Outline {
    pub name: String,
    /// The button list, rebuilt after structural edits.
    pub list: NodeId,
    /// `(button, scene node)` per row, in outline order.
    pub rows: Vec<(NodeId, NodeId)>,
    /// `(button, move)`: Up, Down, In, Out.
    pub moves: Vec<(NodeId, Shift)>,
}

impl Outline {
    pub fn build(tree: &mut NodeTree, ui_root: NodeId) -> Self {
        let panel = add(
            tree,
            ui_root,
            Panel::new()
                .with_padding(Margins::all(6.0))
                .with_offset(Vec2::new(0.0, TOOLBAR_H))
                .with_min_size(Vec2::new(OUTLINE_W, WINDOW_H - TOOLBAR_H)),
        );
        let bar = add(tree, panel, HBox::new().with_spacing(4.0));
        let moves = [
            ("Up", Shift::Up),
            ("Down", Shift::Down),
            ("In", Shift::In),
            ("Out", Shift::Out),
        ]
        .into_iter()
        .map(|(text, shift)| (add(tree, bar, Button::new(text)), shift))
        .collect();
        let scroll = add(
            tree,
            panel,
            ScrollContainer::new()
                .with_offset(Vec2::new(0.0, MOVES_H))
                .with_min_size(Vec2::new(
                    OUTLINE_W - 12.0,
                    WINDOW_H - TOOLBAR_H - MOVES_H - 12.0,
                )),
        );
        let list = add(tree, scroll, VBox::new().with_spacing(2.0));
        Self {
            name: "Outline".into(),
            list,
            rows: Vec::new(),
            moves,
        }
    }

    fn text(doc: &Document, selected: Option<NodeId>, node: NodeId, depth: usize) -> String {
        let mark = if selected == Some(node) { "> " } else { "  " };
        let name = doc.tree().get(node).map_or("?", |n| n.name());
        format!("{mark}{}{name}", "    ".repeat(depth))
    }

    /// Replace the buttons with one per live node.
    pub fn rebuild(&mut self, ctx: &mut NodeContext, doc: &Document, selected: Option<NodeId>) {
        for &(button, _) in &self.rows {
            ctx.despawn(button);
        }
        self.rows = doc
            .outline()
            .iter()
            .map(|row| {
                let text = Self::text(doc, selected, row.id, row.depth);
                let b = ctx.spawn_child(self.list, Button::new(text).with_align(Alignment::Left));
                (b, row.id)
            })
            .collect();
    }

    /// Update the selection marker (and names) in place.
    pub fn refresh(&self, tree: &mut NodeTree, doc: &Document, selected: Option<NodeId>) {
        for (&(button, node), row) in self.rows.iter().zip(doc.outline()) {
            set_button(tree, button, Self::text(doc, selected, node, row.depth));
        }
    }
}

impl ember_scene::Node for Outline {
    fn ready(&mut self, ctx: &mut NodeContext) {
        let me = ctx.current_id();
        ctx.connect::<ButtonClicked>(me, |ctx, ev| {
            let me = ctx.current_id();
            let Some(outline) = node_mut::<Outline>(ctx.tree(), me) else {
                return;
            };
            if let Some(&(_, node)) = outline.rows.iter().find(|(b, _)| *b == ev.button) {
                ctx.emit(NodeSelected { sender: me, node });
            } else if let Some(&(_, shift)) = outline.moves.iter().find(|(b, _)| *b == ev.button) {
                ctx.emit(CommandRequested {
                    sender: me,
                    command: EditorCommand::Shift(shift),
                });
            }
        });
    }
}
