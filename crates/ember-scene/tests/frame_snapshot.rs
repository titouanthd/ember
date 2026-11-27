//! ADR-050: the frame snapshot on `NodeContext` / `DrawContext` —
//! elapsed time, frame count, viewport and the pointer in world space.

use ember_core::MainLoop;
use ember_core::math::Rect;
use ember_input::{Input, InputSink, TouchPhase, TouchPoint};
use ember_scene::prelude::*;
use ember_servers::{Camera2d, MockAudio, MockRenderer};
use glam::Vec2;
use std::sync::{Arc, Mutex};

#[derive(Debug, Default, Clone, PartialEq)]
struct Seen {
    /// `(time, frame)` in each `process`.
    process: Vec<(f64, u64)>,
    physics: Vec<f64>,
    draw: Vec<f64>,
    viewport: Vec<Rect>,
    /// `(pointer, pointer_world)` in each `process`.
    pointer: Vec<(Vec2, Vec2)>,
}

/// Records the snapshot it sees; sets `camera` in `process` first, as a
/// game with a world camera does.
struct Probe {
    name: String,
    camera: Option<Camera2d>,
    seen: Arc<Mutex<Seen>>,
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
        if let Some(camera) = self.camera {
            ctx.render().set_camera(camera);
        }
        let mut seen = self.seen.lock().unwrap();
        seen.process.push((ctx.time(), ctx.frame()));
        seen.viewport.push(ctx.viewport());
        seen.pointer.push((ctx.pointer(), ctx.pointer_world()));
    }
    fn physics_process(&mut self, _dt: f32, ctx: &mut NodeContext) {
        self.seen.lock().unwrap().physics.push(ctx.time());
    }
    fn draw(&self, ctx: &mut DrawContext) {
        self.seen.lock().unwrap().draw.push(ctx.time());
    }
}

fn game(camera: Option<Camera2d>) -> (SceneLoop<MockRenderer, MockAudio>, Arc<Mutex<Seen>>) {
    let seen = Arc::new(Mutex::new(Seen::default()));
    let tree = NodeTree::with_root(Probe {
        name: "Probe".into(),
        camera,
        seen: seen.clone(),
    });
    let mut l = SceneLoop::new(
        tree,
        MockRenderer::new(),
        MockAudio::new(),
        Rect::new(0.0, 0.0, 640.0, 480.0),
    );
    assert!(l.init());
    (l, seen)
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn time_is_zero_before_the_first_process() {
    let (l, _) = game(None);
    assert_eq!(l.tree.time(), 0.0);
    assert_eq!(l.tree.frame(), 0);
}

#[test]
fn time_sums_process_deltas_and_counts_frames() {
    let (mut l, seen) = game(None);
    for dt in [0.5, 0.25, 0.125] {
        l.process(dt);
    }
    let seen = seen.lock().unwrap();
    let process = &seen.process;
    assert_eq!(process.len(), 3);
    // This frame's dt is already counted while `process` runs.
    assert!(close(process[0].0, 0.5));
    assert!(close(process[1].0, 0.75));
    assert!(close(process[2].0, 0.875));
    assert_eq!(
        process.iter().map(|p| p.1).collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    assert!(close(l.tree.time(), 0.875));
    assert_eq!(l.tree.frame(), 3);
}

#[test]
fn physics_and_draw_read_the_frame_time() {
    let (mut l, seen) = game(None);
    l.process(0.5);
    l.physics_process(1.0 / 60.0);
    l.physics_process(1.0 / 60.0);
    l.draw();
    let seen = seen.lock().unwrap();
    // The fixed step does not advance the clock: one frame, one time.
    assert_eq!(seen.physics, vec![0.5, 0.5]);
    assert_eq!(seen.draw, vec![0.5]);
}

#[test]
fn a_negative_or_nan_dt_adds_nothing() {
    let (mut l, _) = game(None);
    l.process(1.0);
    l.process(-3.0);
    l.process(f32::NAN);
    assert!(close(l.tree.time(), 1.0));
    assert_eq!(l.tree.frame(), 3);
}

#[test]
fn a_draw_context_outside_a_tree_reads_zero() {
    let mut r = MockRenderer::new();
    assert_eq!(DrawContext::new(&mut r).time(), 0.0);
}

#[test]
fn viewport_is_the_last_drawn_frame() {
    let (mut l, seen) = game(None);
    l.draw();
    l.process(0.1);
    assert_eq!(
        seen.lock().unwrap().viewport,
        vec![Rect::new(0.0, 0.0, 640.0, 480.0)]
    );
}

#[test]
fn viewport_is_the_canvas_with_a_virtual_resolution() {
    let (l, seen) = game(None);
    let mut l = l.with_virtual_resolution(320.0, 240.0);
    l.resize(1280.0, 960.0);
    l.draw();
    l.set_input(Input {
        mouse_pos: Vec2::new(640.0, 480.0),
        ..Input::default()
    });
    l.process(0.1);
    let seen = seen.lock().unwrap();
    assert_eq!(seen.viewport, vec![Rect::new(0.0, 0.0, 320.0, 240.0)]);
    // Window centre → canvas centre; no world camera → same point.
    assert_eq!(
        seen.pointer,
        vec![(Vec2::new(160.0, 120.0), Vec2::new(160.0, 120.0))]
    );
}

#[test]
fn pointer_world_maps_through_the_camera() {
    let camera = Camera2d::at(Vec2::new(100.0, 50.0)).with_zoom(2.0);
    let (mut l, seen) = game(Some(camera));
    l.set_input(Input {
        mouse_pos: Vec2::new(20.0, 40.0),
        ..Input::default()
    });
    l.process(0.1);
    let (screen, world) = seen.lock().unwrap().pointer[0];
    assert_eq!(screen, Vec2::new(20.0, 40.0));
    assert_eq!(world, camera.screen_to_world(screen));
    assert_eq!(world, Vec2::new(110.0, 70.0));
}

#[test]
fn a_touch_wins_over_the_mouse() {
    let (mut l, seen) = game(None);
    l.set_input(Input {
        mouse_pos: Vec2::new(1.0, 1.0),
        touch_points: vec![
            TouchPoint::new(7, Vec2::new(30.0, 60.0), TouchPhase::Started),
            TouchPoint::new(8, Vec2::new(90.0, 90.0), TouchPhase::Started),
        ],
        ..Input::default()
    });
    l.process(0.1);
    assert_eq!(seen.lock().unwrap().pointer[0].0, Vec2::new(30.0, 60.0));
}
