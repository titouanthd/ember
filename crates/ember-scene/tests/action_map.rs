//! `SceneLoop::with_action_map` (ADR-052): nodes read named actions
//! from `ctx.input()`, swipes measured in canvas units.

use ember_core::MainLoop;
use ember_core::math::Rect;
use ember_input::{ActionMap, Input, InputSink, Key, TouchPhase, TouchPoint};
use ember_scene::prelude::*;
use ember_servers::{MockAudio, MockRenderer};
use glam::Vec2;
use std::sync::{Arc, Mutex};

type Seen = Arc<Mutex<Vec<String>>>;

struct Mover {
    seen: Seen,
}

impl Object for Mover {
    fn class_name() -> &'static str {
        "Mover"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl NodeName for Mover {
    fn name(&self) -> &str {
        "Mover"
    }
    fn set_name(&mut self, _: String) {}
}

impl Node for Mover {
    fn process(&mut self, _dt: f32, ctx: &mut NodeContext) {
        let i = ctx.input();
        let mut line = Vec::new();
        for a in ["left", "right"] {
            if i.action_pressed(a) {
                line.push(a);
            }
        }
        self.seen.lock().unwrap().push(line.join(","));
    }
}

fn map() -> ActionMap {
    ActionMap::from_ron(
        r#"(actions: [
            (name: "left", keys: [Left], swipe: [Left]),
            (name: "right", keys: [Right], swipe: [Right]),
        ])"#,
    )
    .unwrap()
}

fn game(seen: &Seen) -> SceneLoop<MockRenderer, MockAudio> {
    let tree = NodeTree::with_root(Mover { seen: seen.clone() });
    let mut l = SceneLoop::new(
        tree,
        MockRenderer::new(),
        MockAudio::new(),
        Rect::new(0.0, 0.0, 1000.0, 1000.0),
    )
    .with_action_map(map());
    assert!(l.init());
    l
}

fn frame(l: &mut SceneLoop<MockRenderer, MockAudio>, input: Input) {
    l.set_input(input);
    l.process(1.0 / 60.0);
}

fn touch(x: f32, phase: TouchPhase) -> Input {
    Input {
        touch_points: vec![TouchPoint::new(1, Vec2::new(x, 500.0), phase)],
        ..Default::default()
    }
}

#[test]
fn nodes_read_actions_from_keys_and_swipes() {
    let seen = Seen::default();
    let mut l = game(&seen);
    assert_eq!(l.action_map().unwrap().actions.len(), 2);
    frame(
        &mut l,
        Input {
            keys_pressed: vec![Key::Left],
            keys_down: vec![Key::Left],
            ..Default::default()
        },
    );
    frame(&mut l, Input::default());
    frame(&mut l, touch(500.0, TouchPhase::Started));
    frame(&mut l, touch(560.0, TouchPhase::Moved));
    assert_eq!(seen.lock().unwrap().as_slice(), ["left", "", "", "right"]);
}

#[test]
fn swipe_distance_is_in_canvas_units_under_a_virtual_resolution() {
    let seen = Seen::default();
    // 1000x1000 window, 800x600 canvas: 1.25 window pixels per unit.
    let mut l = game(&seen).with_virtual_resolution(800.0, 600.0);
    l.resize(1000.0, 1000.0);
    frame(&mut l, touch(500.0, TouchPhase::Started));
    // 35 window px = 28 canvas units < 32: no swipe.
    frame(&mut l, touch(465.0, TouchPhase::Moved));
    // 45 window px = 36 canvas units: swipe.
    frame(&mut l, touch(455.0, TouchPhase::Moved));
    assert_eq!(seen.lock().unwrap().as_slice(), ["", "", "left"]);
}

#[test]
fn swapping_or_dropping_the_map() {
    let seen = Seen::default();
    let mut l = game(&seen);
    let held = || Input {
        keys_down: vec![Key::Right],
        ..Default::default()
    };
    frame(&mut l, held());
    frame(&mut l, held());
    l.set_action_map(Some(map()));
    frame(&mut l, held()); // state starts over: pressed again
    l.set_action_map(None);
    assert!(l.action_map().is_none());
    frame(&mut l, held());
    assert_eq!(seen.lock().unwrap().as_slice(), ["right", "", "right", ""]);
}
