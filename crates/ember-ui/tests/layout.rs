//! Phase 5.1: anchors, containers' arrange hook, theme hand-down, the
//! control registry and `control_node!` drawing (ADR-034).

mod common;
use common::*;
use ember_core::Color;
use ember_core::math::Rect;
use ember_scene::{DrawContext, Node, NodeId, NodeTree};
use ember_servers::{MockRenderer, RecordedCall, RenderingServer};
use ember_ui::layout::layout_children;
use ember_ui::prelude::*;
use glam::Vec2;
use std::sync::Arc;

const SCREEN: Rect = Rect::new(0.0, 0.0, 800.0, 600.0);

// --- Helpers -------------------------------------------------------------

fn registry() -> ControlRegistry {
    let mut r = ControlRegistry::new();
    r.register::<Swatch>()
        .register::<Column>()
        .register::<Leaky>();
    r
}

/// A tree whose root is a plain node; returns (tree, root).
fn tree() -> (NodeTree, NodeId) {
    let mut t = NodeTree::new();
    let root = t.insert(Plain {
        name: "Root".into(),
    });
    t.set_root(root);
    (t, root)
}

fn add(tree: &mut NodeTree, parent: NodeId, node: impl Node) -> NodeId {
    let id = tree.insert(node);
    tree.attach(id, parent).unwrap();
    id
}

fn control<'a>(tree: &'a NodeTree, reg: &ControlRegistry, id: NodeId) -> &'a dyn Control {
    reg.cast(tree.get(id).unwrap()).unwrap()
}

fn run_layout(tree: &mut NodeTree, reg: &ControlRegistry, root: NodeId, theme: &Arc<Theme>) {
    let render = MockRenderer::new();
    layout_children(tree, reg, &render, root, SCREEN, theme);
}

// --- Layout --------------------------------------------------------------

#[test]
fn full_rect_child_fills_the_parent_and_gets_the_theme() {
    let (mut t, root) = tree();
    let reg = registry();
    let id = add(
        &mut t,
        root,
        Swatch::new(ControlBase::new().with_anchors(Anchors::FULL_RECT)),
    );
    let theme = Arc::new(Theme::light());
    run_layout(&mut t, &reg, root, &theme);

    let c = control(&t, &reg, id);
    assert_eq!(c.rect(), SCREEN);
    assert_eq!(*c.base().theme, Theme::light());
}

#[test]
fn top_left_child_uses_min_size_and_offset() {
    let (mut t, root) = tree();
    let reg = registry();
    let id = add(
        &mut t,
        root,
        Swatch::new(
            ControlBase::new()
                .with_min_size(Vec2::new(120.0, 40.0))
                .with_offset(Vec2::new(16.0, 8.0)),
        ),
    );
    run_layout(&mut t, &reg, root, &Arc::new(Theme::dark()));
    assert_eq!(
        control(&t, &reg, id).rect(),
        Rect::new(16.0, 8.0, 120.0, 40.0)
    );
}

#[test]
fn center_anchor_places_the_top_left_at_the_centre() {
    let (mut t, root) = tree();
    let reg = registry();
    let size = Vec2::new(200.0, 100.0);
    let id = add(
        &mut t,
        root,
        Swatch::new(
            ControlBase::new()
                .with_anchors(Anchors::CENTER)
                .with_min_size(size)
                .with_offset(Vec2::new(-100.0, -50.0)),
        ),
    );
    run_layout(&mut t, &reg, root, &Arc::new(Theme::dark()));
    assert_eq!(
        control(&t, &reg, id).rect(),
        Rect::new(300.0, 250.0, 200.0, 100.0)
    );
}

#[test]
fn nested_children_are_laid_out_inside_their_parent() {
    let (mut t, root) = tree();
    let reg = registry();
    let right_half = add(
        &mut t,
        root,
        Swatch::new(ControlBase::new().with_anchors(Anchors::new(0.5, 0.0, 1.0, 1.0))),
    );
    let bottom_of_it = add(
        &mut t,
        right_half,
        Swatch::new(ControlBase::new().with_anchors(Anchors::new(0.0, 0.5, 1.0, 1.0))),
    );
    run_layout(&mut t, &reg, root, &Arc::new(Theme::dark()));
    assert_eq!(
        control(&t, &reg, right_half).rect(),
        Rect::new(400.0, 0.0, 400.0, 600.0)
    );
    assert_eq!(
        control(&t, &reg, bottom_of_it).rect(),
        Rect::new(400.0, 300.0, 400.0, 300.0)
    );
}

#[test]
fn a_container_arrange_hook_overrides_anchors() {
    let (mut t, root) = tree();
    let reg = registry();
    let col = add(
        &mut t,
        root,
        Column {
            name: "Column".into(),
            base: ControlBase::new().with_anchors(Anchors::FULL_RECT),
        },
    );
    let a = add(
        &mut t,
        col,
        Swatch::new(ControlBase::new().with_min_size(Vec2::new(0.0, 30.0))),
    );
    let b = add(
        &mut t,
        col,
        Swatch::new(
            ControlBase::new()
                .with_anchors(Anchors::FULL_RECT) // ignored inside a Column
                .with_min_size(Vec2::new(0.0, 50.0)),
        ),
    );
    run_layout(&mut t, &reg, root, &Arc::new(Theme::dark()));
    assert_eq!(
        control(&t, &reg, a).rect(),
        Rect::new(0.0, 0.0, 800.0, 30.0)
    );
    assert_eq!(
        control(&t, &reg, b).rect(),
        Rect::new(0.0, 30.0, 800.0, 50.0)
    );
}

#[test]
fn unregistered_controls_and_their_subtrees_are_skipped() {
    let (mut t, root) = tree();
    let mut reg = ControlRegistry::new();
    reg.register::<Swatch>();
    let col = add(
        &mut t,
        root,
        Column {
            name: "Column".into(),
            base: ControlBase::new().with_anchors(Anchors::FULL_RECT),
        },
    );
    let inside = add(
        &mut t,
        col,
        Swatch::new(ControlBase::new().with_anchors(Anchors::FULL_RECT)),
    );
    let plain = add(
        &mut t,
        root,
        Plain {
            name: "Plain".into(),
        },
    );
    let under_plain = add(
        &mut t,
        plain,
        Swatch::new(ControlBase::new().with_anchors(Anchors::FULL_RECT)),
    );
    run_layout(&mut t, &reg, root, &Arc::new(Theme::dark()));
    assert_eq!(control(&t, &reg, inside).rect(), Rect::default());
    assert_eq!(control(&t, &reg, under_plain).rect(), Rect::default());
}

// --- Registry ------------------------------------------------------------

#[test]
fn registry_casts_only_registered_types() {
    let mut reg = ControlRegistry::new();
    assert!(reg.is_empty());
    reg.register::<Swatch>();
    reg.register::<Swatch>(); // twice is harmless
    assert_eq!(reg.len(), 1);
    assert!(reg.is_registered::<Swatch>());
    assert!(!reg.is_registered::<Column>());

    let swatch = Swatch::new(ControlBase::new());
    let plain = Plain { name: "p".into() };
    assert!(reg.cast(&swatch).is_some());
    assert!(reg.cast(&plain).is_none());
}

// --- Drawing ---------------------------------------------------------------

#[test]
fn control_node_draws_on_the_ui_layer_with_the_theme() {
    let (mut t, root) = tree();
    let reg = registry();
    let id = add(
        &mut t,
        root,
        Swatch::new(ControlBase::new().with_anchors(Anchors::FULL_RECT)),
    );
    let theme = Arc::new(Theme::light());
    run_layout(&mut t, &reg, root, &theme);

    let node = t.get(id).unwrap();
    assert_eq!(node.z(), UI_Z);
    assert!(node.visible());

    let mut r = MockRenderer::new();
    r.begin_frame(SCREEN);
    node.draw(&mut DrawContext::new(&mut r));
    r.end_frame();
    assert_eq!(
        r.calls(),
        &[
            RecordedCall::DrawRect {
                rect: SCREEN,
                color: Color::RED,
                z: 0.0,
            },
            RecordedCall::DrawRectOutline {
                rect: SCREEN,
                thickness: 1.0,
                color: Theme::light().panel_border,
                z: 0.0,
            },
        ]
    );
}

#[test]
fn a_hidden_control_reports_invisible() {
    let mut base = ControlBase::new();
    base.visible = false;
    assert!(!Swatch::new(base).visible());
}

#[test]
fn leftover_clips_are_popped_when_drawing_ends() {
    let leaky = Leaky {
        name: "Leaky".into(),
        base: ControlBase::new(),
    };
    let mut r = MockRenderer::new();
    r.begin_frame(SCREEN);
    leaky.draw(&mut DrawContext::new(&mut r));
    assert_eq!(r.clip_rect(), None);
    r.end_frame();
    let pops = r
        .calls()
        .iter()
        .filter(|c| **c == RecordedCall::PopClipRect)
        .count();
    assert_eq!(pops, 2);
}
