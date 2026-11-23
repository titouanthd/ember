//! Phase 5 deliverable: every `ember_ui` widget in one window.
//!
//! - Tab / Down and Up move focus; Enter or Space presses the focused
//!   button; gamepad D-pad and South do the same.
//! - Type a name, then Greet (or press Enter in the field).
//! - The wheel scrolls the list; rows scrolled out of view are clipped.
//! - Escape quits.
//!
//! Run with `make ui-demo`.

use ember::prelude::*;

#[derive(Node)]
#[class_name = "Demo"]
struct Demo {
    name: String,
}

impl Node for Demo {
    fn process(&mut self, _dt: f32, ctx: &mut NodeContext) {
        if ctx.input().is_key_pressed(Key::Escape) {
            ctx.quit();
        }
    }
}

/// `tree.get_mut(id)` as a concrete control.
fn control<T: 'static>(tree: &mut NodeTree, id: NodeId) -> Option<&mut T> {
    tree.get_mut(id)?.as_any_mut().downcast_mut::<T>()
}

fn add(tree: &mut NodeTree, parent: NodeId, node: impl Node) -> NodeId {
    let id = tree.insert(node);
    tree.attach(id, parent).expect("attach");
    id
}

fn greet(tree: &mut NodeTree, field: NodeId, status: NodeId) {
    let name = control::<TextInput>(tree, field)
        .map(|t| t.text().trim().to_string())
        .unwrap_or_default();
    let message = if name.is_empty() {
        "Type a name first".to_string()
    } else {
        format!("Hello, {name}!")
    };
    if let Some(label) = control::<Label>(tree, status) {
        label.set_text(message);
    }
}

fn main() {
    const W: f32 = 800.0;
    const H: f32 = 600.0;
    const PANEL: Vec2 = Vec2::new(380.0, 480.0);

    let mut tree = NodeTree::with_root(Demo {
        name: "Demo".into(),
    });
    let root = tree.root().expect("root");
    let ui = add(&mut tree, root, UiRoot::new(Theme::dark()));

    // A centred panel: CENTER anchors give the centre point, the offset
    // moves the top-left corner back by half the size.
    let panel = add(
        &mut tree,
        ui,
        Panel::new()
            .with_padding(Margins::all(16.0))
            .with_anchors(Anchors::CENTER)
            .with_offset(-PANEL * 0.5)
            .with_min_size(PANEL),
    );
    let column = add(&mut tree, panel, VBox::new().with_spacing(10.0));

    add(
        &mut tree,
        column,
        Label::new("ember_ui demo").with_size(Theme::dark().font_size_large),
    );
    let field = add(
        &mut tree,
        column,
        TextInput::new()
            .with_placeholder("Your name")
            .with_max_len(24),
    );
    let buttons = add(&mut tree, column, HBox::new().with_spacing(8.0));
    let greet_button = add(&mut tree, buttons, Button::new("Greet"));
    let clear_button = add(&mut tree, buttons, Button::new("Clear"));
    add(&mut tree, buttons, Button::new("Disabled").disabled());
    let quit_button = add(&mut tree, buttons, Button::new("Quit"));

    let scroll = add(
        &mut tree,
        column,
        ScrollContainer::new().with_min_size(Vec2::new(0.0, 220.0)),
    );
    let list = add(&mut tree, scroll, VBox::new().with_spacing(4.0));
    for i in 1..=30 {
        add(&mut tree, list, Label::new(format!("Row {i:02}")));
    }
    let status = add(&mut tree, column, Label::new("Tab to start"));

    // Signals are broadcast by type: handlers check which control sent
    // them (ADR-036).
    tree.connect::<ButtonClicked>(root, move |ctx, ev| {
        if ev.button == greet_button {
            greet(ctx.tree(), field, status);
        } else if ev.button == clear_button {
            if let Some(t) = control::<TextInput>(ctx.tree(), field) {
                t.set_text("");
            }
            if let Some(l) = control::<Label>(ctx.tree(), status) {
                l.set_text("Cleared");
            }
        } else if ev.button == quit_button {
            ctx.quit();
        }
    });
    tree.connect::<TextSubmitted>(root, move |ctx, ev| {
        if ev.input == field {
            greet(ctx.tree(), field, status);
        }
    });

    let scene = SceneLoop::new(
        tree,
        MacroquadRenderer::new(),
        MacroquadAudio::new(),
        Rect::new(0.0, 0.0, W, H),
    );
    run_window("ui_demo", W as i32, H as i32, scene);
}
