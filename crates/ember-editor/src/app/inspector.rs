//! The inspector: a header and one typed editor row per exported
//! property, in a `ScrollContainer` (no row cap). Rows are rebuilt when
//! the selection or its property list changes, and updated in place
//! after an edit (so a focused field keeps its focus).
//!
//! | value | editor |
//! |---|---|
//! | `bool` | a toggle `Button` |
//! | `#[export(enum)]` (ADR-049) | a `Button` cycling the names |
//! | `Int`, `Float`, text | one `TextInput` |
//! | `Vec2` | two `TextInput`s (x, y) |
//! | `Color` | a swatch and four `TextInput`s (r, g, b, a) |

use super::{
    Edit, INSPECTOR_W, PropertyEdited, TOOLBAR_H, WINDOW_H, WINDOW_W, add, node_mut, set_button,
    set_field, set_label,
};
use crate::{Document, value};
use ember_core::{Color, Variant};
use ember_macros::Node;
use ember_scene::{NodeContext, NodeId, NodeTree, PropertyHint};
use ember_ui::prelude::*;
use glam::Vec2;

const KEY_W: f32 = 84.0;
const PAD: f32 = 10.0;
const HEADER_H: f32 = 52.0;
const SWATCH: f32 = 18.0;

/// One property row.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub key: &'static str,
    /// The `HBox` holding the row.
    pub row: NodeId,
    pub label: NodeId,
    pub editor: RowEditor,
    /// The value shown, as last read from the document.
    pub value: Variant,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RowEditor {
    /// `bool`: a click flips it.
    Toggle(NodeId),
    /// An enum: a click picks the next name.
    Cycle(NodeId, &'static [&'static str]),
    /// One field per part (see `value::components`), plus a swatch for
    /// colours. Enter in field `i` sets part `i`.
    Fields {
        fields: Vec<NodeId>,
        swatch: Option<NodeId>,
    },
}

/// The editor kind a value gets; rows are reused only when it matches.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    Toggle,
    Cycle,
    Fields(usize),
}

fn kind_of(v: &Variant, hint: PropertyHint) -> Kind {
    match (v, hint) {
        (Variant::String(_), PropertyHint::Enum(_)) => Kind::Cycle,
        (Variant::Bool(_), _) => Kind::Toggle,
        (v, _) => Kind::Fields(value::components(v).len()),
    }
}

fn kind(editor: &RowEditor) -> Kind {
    match editor {
        RowEditor::Toggle(_) => Kind::Toggle,
        RowEditor::Cycle(..) => Kind::Cycle,
        RowEditor::Fields { fields, .. } => Kind::Fields(fields.len()),
    }
}

fn toggle_text(v: &Variant) -> &'static str {
    if v.as_bool() == Some(true) {
        "[x] true"
    } else {
        "[ ] false"
    }
}

fn cycle_text(v: &Variant) -> String {
    format!("{} >", v.as_str().unwrap_or(""))
}

#[derive(Node, Clone, Debug)]
#[class_name = "EditorInspector"]
pub struct Inspector {
    pub name: String,
    pub header: NodeId,
    /// The `VBox` inside the `ScrollContainer` that holds the rows.
    pub list: NodeId,
    pub scroll: NodeId,
    /// The node the rows show.
    pub node: Option<NodeId>,
    pub rows: Vec<Row>,
}

impl Inspector {
    pub fn build(tree: &mut NodeTree, ui_root: NodeId, theme: &Theme) -> Self {
        let panel = add(
            tree,
            ui_root,
            Panel::new()
                .with_padding(Margins::all(PAD))
                .with_offset(Vec2::new(WINDOW_W - INSPECTOR_W, TOOLBAR_H))
                .with_min_size(Vec2::new(INSPECTOR_W, WINDOW_H - TOOLBAR_H)),
        );
        let top = add(tree, panel, VBox::new().with_spacing(6.0));
        let header = add(tree, top, Label::new("").with_size(theme.font_size_large));
        add(
            tree,
            top,
            Label::new("Enter applies a value").with_color(theme.text_dim),
        );
        let scroll = add(
            tree,
            panel,
            ScrollContainer::new()
                .with_offset(Vec2::new(0.0, HEADER_H))
                .with_min_size(Vec2::new(
                    INSPECTOR_W - 2.0 * PAD,
                    WINDOW_H - TOOLBAR_H - HEADER_H - 2.0 * PAD,
                )),
        );
        let list = add(tree, scroll, VBox::new().with_spacing(6.0));
        Self {
            name: "Inspector".into(),
            header,
            list,
            scroll,
            node: None,
            rows: Vec::new(),
        }
    }

    /// The rows' text fields of `key` (one for a number, two for a
    /// `Vec2`, four for a `Color`).
    pub fn fields(&self, key: &str) -> Vec<NodeId> {
        match self.rows.iter().find(|r| r.key == key).map(|r| &r.editor) {
            Some(RowEditor::Fields { fields, .. }) => fields.clone(),
            _ => Vec::new(),
        }
    }

    /// The toggle or cycle button of `key`.
    pub fn button(&self, key: &str) -> Option<NodeId> {
        match self.rows.iter().find(|r| r.key == key)?.editor {
            RowEditor::Toggle(b) | RowEditor::Cycle(b, _) => Some(b),
            RowEditor::Fields { .. } => None,
        }
    }

    /// The colour swatch of `key`.
    pub fn swatch(&self, key: &str) -> Option<NodeId> {
        match &self.rows.iter().find(|r| r.key == key)?.editor {
            RowEditor::Fields { swatch, .. } => *swatch,
            _ => None,
        }
    }

    /// Show `selected`: rebuild the rows if the node or its property
    /// shape changed, else update them in place.
    pub fn show(&mut self, ctx: &mut NodeContext, doc: &Document, selected: Option<NodeId>) {
        let props: Vec<(&'static str, Variant, PropertyHint)> = selected
            .and_then(|id| doc.properties(id).ok())
            .unwrap_or_default()
            .into_iter()
            .map(|(k, v)| {
                let hint = selected.map_or(PropertyHint::None, |id| doc.property_hint(id, k));
                (k, v, hint)
            })
            .collect();
        let header = match selected {
            Some(id) => {
                let name = doc.tree().get(id).map_or("", |n| n.name());
                format!("{name} ({})", doc.class_of(id).unwrap_or("?"))
            }
            None => "Nothing selected".into(),
        };
        set_label(ctx.tree(), self.header, header);
        let same = self.node == selected
            && self.rows.len() == props.len()
            && self
                .rows
                .iter()
                .zip(&props)
                .all(|(r, (k, v, h))| r.key == *k && kind(&r.editor) == kind_of(v, *h));
        if same {
            let tree = ctx.tree();
            for (row, (_, v, _)) in self.rows.iter_mut().zip(props) {
                row.value = v;
                refresh(tree, row);
            }
            return;
        }
        for row in &self.rows {
            ctx.despawn(row.row);
        }
        self.node = selected;
        self.rows = props
            .into_iter()
            .map(|(key, v, hint)| spawn_row(ctx, self.list, key, v, hint))
            .collect();
        if let Some(scroll) = node_mut::<ScrollContainer>(ctx.tree(), self.scroll) {
            scroll.set_scroll(0.0);
        }
    }

    /// The edit a click on `button` asks for, if it is one of ours.
    pub fn on_click(&self, button: NodeId) -> Option<(NodeId, &'static str, Edit)> {
        let node = self.node?;
        self.rows.iter().find_map(|r| match r.editor {
            RowEditor::Toggle(b) if b == button => {
                let on = r.value.as_bool().unwrap_or(false);
                Some((node, r.key, Edit::Set(Variant::Bool(!on))))
            }
            RowEditor::Cycle(b, names) if b == button && !names.is_empty() => {
                let at = names.iter().position(|n| Some(*n) == r.value.as_str());
                let next = names[at.map_or(0, |i| (i + 1) % names.len())];
                Some((node, r.key, Edit::Set(Variant::String(next.into()))))
            }
            _ => None,
        })
    }

    /// The edit Enter in `input` asks for, if it is one of ours.
    pub fn on_submit(&self, input: NodeId, text: &str) -> Option<(NodeId, &'static str, Edit)> {
        let node = self.node?;
        self.rows.iter().find_map(|r| match &r.editor {
            RowEditor::Fields { fields, .. } => {
                let index = fields.iter().position(|&f| f == input)?;
                let text = text.to_string();
                Some((node, r.key, Edit::Part { index, text }))
            }
            _ => None,
        })
    }
}

/// Field widths per part count, so a row fits the panel.
fn field_width(parts: usize) -> f32 {
    match parts {
        1 => 180.0,
        2 => 86.0,
        _ => 40.0,
    }
}

fn spawn_row(
    ctx: &mut NodeContext,
    list: NodeId,
    key: &'static str,
    value: Variant,
    hint: PropertyHint,
) -> Row {
    let row = ctx.spawn_child(list, HBox::new().with_spacing(6.0));
    let label = ctx.spawn_child(row, Label::new(key).with_min_size(Vec2::new(KEY_W, 0.0)));
    let editor = match kind_of(&value, hint) {
        Kind::Toggle => RowEditor::Toggle(ctx.spawn_child(row, Button::new(toggle_text(&value)))),
        Kind::Cycle => {
            let names = match hint {
                PropertyHint::Enum(names) => names,
                PropertyHint::None => &[],
            };
            RowEditor::Cycle(ctx.spawn_child(row, Button::new(cycle_text(&value))), names)
        }
        Kind::Fields(n) => {
            let swatch = value.as_color().map(|c| {
                ctx.spawn_child(
                    row,
                    Panel::new()
                        .with_background(c)
                        .with_border(1.0, Some(Color::new(1.0, 1.0, 1.0, 0.5)))
                        .with_min_size(Vec2::splat(SWATCH)),
                )
            });
            let fields = value::components(&value)
                .into_iter()
                .map(|text| {
                    ctx.spawn_child(
                        row,
                        TextInput::new()
                            .with_text(text)
                            .with_min_size(Vec2::new(field_width(n), 0.0)),
                    )
                })
                .collect();
            RowEditor::Fields { fields, swatch }
        }
    };
    Row {
        key,
        row,
        label,
        editor,
        value,
    }
}

/// Show `row.value` in its existing widgets.
fn refresh(tree: &mut NodeTree, row: &Row) {
    match &row.editor {
        RowEditor::Toggle(b) => set_button(tree, *b, toggle_text(&row.value)),
        RowEditor::Cycle(b, _) => set_button(tree, *b, cycle_text(&row.value)),
        RowEditor::Fields { fields, swatch } => {
            for (&f, text) in fields.iter().zip(value::components(&row.value)) {
                set_field(tree, f, text);
            }
            if let (Some(s), Some(c)) = (swatch, row.value.as_color())
                && let Some(panel) = node_mut::<Panel>(tree, *s)
            {
                panel.background = Some(c);
            }
        }
    }
}

impl ember_scene::Node for Inspector {
    fn ready(&mut self, ctx: &mut NodeContext) {
        let me = ctx.current_id();
        ctx.connect::<ButtonClicked>(me, |ctx, ev| {
            let me = ctx.current_id();
            let edit = node_mut::<Inspector>(ctx.tree(), me).and_then(|i| i.on_click(ev.button));
            if let Some((node, key, edit)) = edit {
                ctx.emit(PropertyEdited {
                    sender: me,
                    node,
                    key,
                    edit,
                });
            }
        });
        ctx.connect::<TextSubmitted>(me, |ctx, ev| {
            let me = ctx.current_id();
            let edit =
                node_mut::<Inspector>(ctx.tree(), me).and_then(|i| i.on_submit(ev.input, &ev.text));
            if let Some((node, key, edit)) = edit {
                ctx.emit(PropertyEdited {
                    sender: me,
                    node,
                    key,
                    edit,
                });
            }
        });
    }
}
