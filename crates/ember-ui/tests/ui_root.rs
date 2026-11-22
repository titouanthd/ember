//! `UiRoot` inside a real `NodeTree`: one `process` lays the UI out in
//! the viewport and hands every control the theme (Phase 5.1).

mod common;
use common::*;
use ember_core::math::Rect;
use ember_input::Input;
use ember_scene::{NodeTree, Servers};
use ember_servers::{LinearPhysics, MockAudio, MockRenderer, RenderingServer};
use ember_ui::prelude::*;

#[test]
fn ui_root_lays_out_its_subtree_each_process() {
    let mut tree = NodeTree::with_root(UiRoot::new(Theme::light()).register::<Swatch>());
    let root = tree.root().unwrap();
    let bottom_half = tree.insert(Swatch::new(
        ControlBase::new().with_anchors(Anchors::new(0.0, 0.5, 1.0, 1.0)),
    ));
    tree.attach(bottom_half, root).unwrap();

    let mut render = MockRenderer::new();
    render.begin_frame(Rect::new(0.0, 0.0, 640.0, 480.0));
    let mut audio = MockAudio::new();
    let mut physics = LinearPhysics::new();
    let input = Input::default();
    let mut servers = Servers {
        rendering: &mut render,
        audio: &mut audio,
        physics: &mut physics,
        input: &input,
    };
    tree.process(&mut servers, 0.016);

    let swatch = tree
        .get(bottom_half)
        .unwrap()
        .as_any()
        .downcast_ref::<Swatch>()
        .unwrap();
    assert_eq!(swatch.rect(), Rect::new(0.0, 240.0, 640.0, 240.0));
    assert_eq!(*swatch.base().theme, Theme::light());
    assert_eq!(tree.get(root).unwrap().z(), UI_Z);
}
