//! Phase 5.2: VBox, HBox, Stretch and Panel layout, bottom-up measuring.

mod common;
use common::*;
use ember_core::math::Rect;
use ember_scene::{Node, NodeId, NodeTree};
use ember_servers::MockRenderer;
use ember_ui::layout::{layout_children, measure_node};
use ember_ui::prelude::*;
use glam::Vec2;
use std::sync::Arc;

const SCREEN: Rect = Rect::new(0.0, 0.0, 800.0, 600.0);

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

fn registry() -> ControlRegistry {
    let mut r = ControlRegistry::with_builtins();
    r.register::<Swatch>();
    r
}

fn layout(tree: &mut NodeTree, reg: &ControlRegistry, root: NodeId) {
    let render = MockRenderer::new();
    layout_children(tree, reg, &render, root, SCREEN, &Arc::new(Theme::dark()));
}

fn rect(tree: &NodeTree, reg: &ControlRegistry, id: NodeId) -> Rect {
    reg.cast(tree.get(id).unwrap()).unwrap().rect()
}

fn swatch(w: f32, h: f32) -> Swatch {
    Swatch::new(ControlBase::new().with_min_size(Vec2::new(w, h)))
}

#[test]
fn vbox_stacks_children_with_spacing_and_padding() {
    let (mut t, root) = tree();
    let reg = registry();
    let vbox = add(
        &mut t,
        root,
        VBox::new()
            .with_spacing(10.0)
            .with_padding(Margins::all(5.0))
            .with_anchors(Anchors::FULL_RECT),
    );
    let a = add(&mut t, vbox, swatch(50.0, 30.0));
    let b = add(&mut t, vbox, swatch(80.0, 40.0));
    layout(&mut t, &reg, root);

    assert_eq!(rect(&t, &reg, vbox), SCREEN);
    assert_eq!(rect(&t, &reg, a), Rect::new(5.0, 5.0, 790.0, 30.0));
    assert_eq!(rect(&t, &reg, b), Rect::new(5.0, 45.0, 790.0, 40.0));
}

#[test]
fn hbox_mirrors_vbox_on_the_x_axis() {
    let (mut t, root) = tree();
    let reg = registry();
    let hbox = add(
        &mut t,
        root,
        HBox::new()
            .with_spacing(4.0)
            .with_anchors(Anchors::TOP_WIDE)
            .with_min_size(Vec2::new(0.0, 50.0)),
    );
    let a = add(&mut t, hbox, swatch(100.0, 10.0));
    let b = add(&mut t, hbox, swatch(60.0, 10.0));
    layout(&mut t, &reg, root);

    assert_eq!(rect(&t, &reg, hbox), Rect::new(0.0, 0.0, 800.0, 50.0));
    assert_eq!(rect(&t, &reg, a), Rect::new(0.0, 0.0, 100.0, 50.0));
    assert_eq!(rect(&t, &reg, b), Rect::new(104.0, 0.0, 60.0, 50.0));
}

#[test]
fn a_box_is_measured_from_its_children() {
    let (mut t, root) = tree();
    let reg = registry();
    // TOP_LEFT gives zero size, so the box's size must come from its children.
    let vbox = add(
        &mut t,
        root,
        VBox::new()
            .with_spacing(10.0)
            .with_padding(Margins::symmetric(4.0, 2.0)),
    );
    add(&mut t, vbox, swatch(50.0, 30.0));
    add(&mut t, vbox, swatch(80.0, 40.0));

    let render = MockRenderer::new();
    let theme = Theme::dark();
    let m = Measure::new(&render, &theme);
    let size = measure_node(&t, &reg, &m, vbox, SCREEN.size()).unwrap();
    // width: widest child + padding; height: sum + one gap + padding
    assert_eq!(size, Vec2::new(88.0, 84.0));

    layout(&mut t, &reg, root);
    assert_eq!(rect(&t, &reg, vbox), Rect::new(0.0, 0.0, 88.0, 84.0));
}

#[test]
fn nested_boxes_measure_bottom_up() {
    let (mut t, root) = tree();
    let reg = registry();
    let outer = add(&mut t, root, VBox::new());
    let row = add(&mut t, outer, HBox::new().with_spacing(2.0));
    add(&mut t, row, swatch(10.0, 20.0));
    add(&mut t, row, swatch(10.0, 25.0));
    add(&mut t, outer, swatch(40.0, 5.0));
    layout(&mut t, &reg, root);

    // row: 10 + 2 + 10 wide, 25 high; outer: max(22, 40) wide, 25 + 5 high
    assert_eq!(rect(&t, &reg, outer), Rect::new(0.0, 0.0, 40.0, 30.0));
    assert_eq!(rect(&t, &reg, row), Rect::new(0.0, 0.0, 40.0, 25.0));
}

#[test]
fn stretch_fill_gives_children_the_content_rect() {
    let (mut t, root) = tree();
    let reg = registry();
    let s = add(
        &mut t,
        root,
        Stretch::new(StretchMode::Fill)
            .with_padding(Margins::all(20.0))
            .with_anchors(Anchors::FULL_RECT),
    );
    let child = add(&mut t, s, swatch(10.0, 10.0));
    layout(&mut t, &reg, root);
    assert_eq!(rect(&t, &reg, child), Rect::new(20.0, 20.0, 760.0, 560.0));
}

#[test]
fn stretch_fit_centres_children_at_their_size() {
    let (mut t, root) = tree();
    let reg = registry();
    let s = add(
        &mut t,
        root,
        Stretch::new(StretchMode::Fit).with_anchors(Anchors::FULL_RECT),
    );
    let child = add(&mut t, s, swatch(200.0, 100.0));
    layout(&mut t, &reg, root);
    assert_eq!(rect(&t, &reg, child), Rect::new(300.0, 250.0, 200.0, 100.0));
}

#[test]
fn panel_padding_shrinks_the_area_its_children_see() {
    let (mut t, root) = tree();
    let reg = registry();
    let p = add(
        &mut t,
        root,
        Panel::new()
            .with_padding(Margins::new(10.0, 20.0, 30.0, 40.0))
            .with_anchors(Anchors::FULL_RECT),
    );
    let child = add(
        &mut t,
        p,
        Swatch::new(ControlBase::new().with_anchors(Anchors::FULL_RECT)),
    );
    layout(&mut t, &reg, root);
    assert_eq!(rect(&t, &reg, child), Rect::new(10.0, 20.0, 760.0, 540.0));
}
