//! ADR-041: a fixed canvas letterboxed into the window.

use ember_core::math::Rect;
use ember_core::{Color, MainLoop};
use ember_input::{Input, InputSink};
use ember_scene::prelude::*;
use ember_servers::{Camera2d, MockAudio, MockRenderer, RecordedCall};
use glam::Vec2;
use std::sync::{Arc, Mutex};

/// Draws the canvas outline and records the mouse and viewport it saw.
struct Probe {
    name: String,
    seen: Arc<Mutex<Vec<(Vec2, Rect)>>>,
}

impl Object for Probe {
    fn class_name() -> &'static str {
        "Probe"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl NodeName for Probe {
    fn name(&self) -> &str {
        &self.name
    }
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

impl Node for Probe {
    fn process(&mut self, _dt: f32, ctx: &mut NodeContext) {
        let mouse = ctx.input().mouse_pos;
        let viewport = ctx.render().viewport();
        self.seen.lock().unwrap().push((mouse, viewport));
    }
    fn draw(&self, ctx: &mut DrawContext) {
        ctx.render()
            .draw_rect(Rect::new(0.0, 0.0, 800.0, 600.0), Color::WHITE);
    }
}

fn game(seen: &Arc<Mutex<Vec<(Vec2, Rect)>>>) -> SceneLoop<MockRenderer, MockAudio> {
    let tree = NodeTree::with_root(Probe {
        name: "Probe".into(),
        seen: seen.clone(),
    });
    let mut l = SceneLoop::new(
        tree,
        MockRenderer::new(),
        MockAudio::new(),
        Rect::new(0.0, 0.0, 1000.0, 1000.0),
    )
    .with_virtual_resolution(800.0, 600.0);
    assert!(l.init());
    l.resize(1000.0, 1000.0);
    l
}

#[test]
fn the_canvas_is_letterboxed_with_bars_and_a_clip() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut l = game(&seen);
    l.draw();
    let letterbox = Camera2d::letterbox(Vec2::splat(1000.0), Vec2::new(800.0, 600.0));
    let calls = l.rendering().calls();
    assert_eq!(calls[0], RecordedCall::SetClearColor(Color::BLACK));
    assert_eq!(calls[1], RecordedCall::SetCamera(letterbox));
    assert!(matches!(
        calls[2],
        RecordedCall::PushClipRect { rect, .. } if rect == Rect::new(0.0, 125.0, 1000.0, 750.0)
    ));
    assert!(calls.iter().any(|c| matches!(
        c,
        RecordedCall::DrawRect { rect, .. } if *rect == Rect::new(0.0, 0.0, 800.0, 600.0)
    )));
    let n = calls.len();
    assert_eq!(calls[n - 2], RecordedCall::PopClipRect);
    assert_eq!(calls[n - 1], RecordedCall::SetCamera(Camera2d::IDENTITY));
    l.rendering().assert_frame_closed();
}

#[test]
fn nodes_see_the_canvas_and_canvas_mouse_positions() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut l = game(&seen);
    l.draw(); // the viewport is set by begin_frame
    l.set_input(Input {
        mouse_pos: Vec2::new(500.0, 500.0),
        ..Input::default()
    });
    l.process(1.0 / 60.0);
    let (mouse, viewport) = seen.lock().unwrap()[0];
    assert_eq!(mouse, Vec2::new(400.0, 300.0));
    assert_eq!(viewport, Rect::new(0.0, 0.0, 800.0, 600.0));
}

#[test]
fn a_resize_moves_the_letterbox() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut l = game(&seen);
    l.resize(1600.0, 600.0);
    assert_eq!(l.window_size(), Vec2::new(1600.0, 600.0));
    let c = l.screen_camera();
    assert_eq!(c.zoom, 1.0);
    assert_eq!(c.world_to_screen(Vec2::ZERO), Vec2::new(400.0, 0.0));
}

#[test]
fn without_a_virtual_resolution_nothing_changes() {
    let mut l = SceneLoop::new(
        NodeTree::new(),
        MockRenderer::new(),
        MockAudio::new(),
        Rect::new(0.0, 0.0, 800.0, 600.0),
    );
    l.resize(1600.0, 1200.0);
    assert_eq!(l.screen_camera(), Camera2d::IDENTITY);
    l.draw();
    assert!(l.rendering().calls().is_empty());
}
