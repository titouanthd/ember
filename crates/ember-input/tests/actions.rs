//! Input action map (ADR-052): RON format, edges, axes, swipes.

use ember_input::{
    ActionMap, ActionMapError, ActionResolver, GamepadButton, Input, Key, MouseButton, Swipe,
    TouchPhase, TouchPoint,
};
use glam::Vec2;

const MAP: &str = r#"(
    actions: [
        (name: "move_left",  keys: [A, Left],  gamepad: [DPadLeft],  swipe: [Left]),
        (name: "move_right", keys: [D, Right], gamepad: [DPadRight], swipe: [Right]),
        (name: "move_up",    keys: [W, Up],    swipe: [Up]),
        (name: "move_down",  keys: [S, Down],  swipe: [Down]),
        (name: "undo",       keys: [Z, Backspace], gamepad: [West]),
        (name: "confirm",    keys: [Space], mouse: [Left]),
        (name: "jump",       keys: [Space]),
    ],
)"#;

fn resolver() -> ActionResolver {
    ActionResolver::new(ActionMap::from_ron(MAP).unwrap())
}

fn keys(down: &[Key]) -> Input {
    Input {
        keys_down: down.to_vec(),
        ..Default::default()
    }
}

fn step(r: &mut ActionResolver, mut input: Input) -> Input {
    r.resolve(&mut input);
    input
}

fn touch(id: u64, x: f32, y: f32, phase: TouchPhase) -> Input {
    Input {
        touch_points: vec![TouchPoint::new(id, Vec2::new(x, y), phase)],
        ..Default::default()
    }
}

#[test]
fn ron_round_trips_and_defaults_apply() {
    let map = ActionMap::from_ron(MAP).unwrap();
    assert_eq!(map.swipe_distance, ActionMap::DEFAULT_SWIPE_DISTANCE);
    assert!(!map.mouse_swipe);
    assert_eq!(map.actions.len(), 7);
    assert_eq!(map.get("undo").unwrap().gamepad, [GamepadButton::West]);
    let again = ActionMap::from_ron(&map.to_ron()).unwrap();
    assert_eq!(again, map);
    assert!(
        !map.to_ron().contains("swipe_distance"),
        "defaults left out"
    );

    let tuned = ActionMap::from_ron(
        r#"(swipe_distance: 12.0, mouse_swipe: true, actions: [(name: "a", keys: [Unknown(7)])])"#,
    )
    .unwrap();
    assert_eq!(tuned.swipe_distance, 12.0);
    assert!(tuned.mouse_swipe);
    assert_eq!(tuned.get("a").unwrap().keys, [Key::Unknown(7)]);
}

#[test]
fn bad_maps_are_rejected() {
    assert!(matches!(
        ActionMap::from_ron("(actions: [(name: \"a\", keys: [NotAKey])])"),
        Err(ActionMapError::Parse(_))
    ));
    assert_eq!(
        ActionMap::from_ron(r#"(actions: [(name: "")])"#),
        Err(ActionMapError::EmptyName)
    );
    assert_eq!(
        ActionMap::from_ron(r#"(actions: [(name: "a"), (name: "a")])"#),
        Err(ActionMapError::DuplicateName("a".into()))
    );
}

#[test]
fn builder_matches_ron() {
    let built = ActionMap::new()
        .with_key("move_left", Key::A)
        .with_key("move_left", Key::Left)
        .with_gamepad("move_left", GamepadButton::DPadLeft)
        .with_swipe("move_left", Swipe::Left)
        .with_mouse("confirm", MouseButton::Left);
    let parsed = ActionMap::from_ron(
        r#"(actions: [
            (name: "move_left", keys: [A, Left], gamepad: [DPadLeft], swipe: [Left]),
            (name: "confirm", mouse: [Left]),
        ])"#,
    )
    .unwrap();
    assert_eq!(built, parsed);
    assert!(built.validate().is_ok());
}

#[test]
fn without_a_resolver_every_action_is_idle() {
    let i = keys(&[Key::A]);
    assert!(!i.action("move_left"));
    assert!(!i.action_pressed("move_left"));
    assert_eq!(i.action_axis("move_left", "move_right"), 0.0);
    assert_eq!(i.swipe(), None);
}

#[test]
fn pressed_down_released_are_edges_across_frames() {
    let mut r = resolver();
    let f1 = step(&mut r, keys(&[Key::A]));
    assert!(f1.action_pressed("move_left") && f1.action("move_left"));
    let f2 = step(&mut r, keys(&[Key::A]));
    assert!(!f2.action_pressed("move_left") && f2.action("move_left"));
    // A second binding going down while the first is held: no new press.
    let f3 = step(&mut r, keys(&[Key::A, Key::Left]));
    assert!(!f3.action_pressed("move_left"));
    let f4 = step(&mut r, keys(&[Key::Left]));
    assert!(f4.action("move_left") && !f4.action_released("move_left"));
    let f5 = step(&mut r, keys(&[]));
    assert!(f5.action_released("move_left") && !f5.action("move_left"));
    let f6 = step(&mut r, keys(&[]));
    assert!(!f6.action_released("move_left"));
    assert!(
        !f6.action("nope") && !f6.action_pressed("nope"),
        "unknown name"
    );
}

#[test]
fn a_key_pressed_only_counts_and_one_key_serves_two_actions() {
    let mut r = resolver();
    let i = step(
        &mut r,
        Input {
            keys_pressed: vec![Key::Space],
            ..Default::default()
        },
    );
    assert!(i.action_pressed("confirm") && i.action_pressed("jump"));
    assert_eq!(i.actions.pressed, ["confirm", "jump"], "map order");
}

#[test]
fn gamepad_and_mouse_bindings() {
    let mut r = resolver();
    let i = step(
        &mut r,
        Input {
            gamepad_down: vec![GamepadButton::West],
            mouse_left_pressed: true,
            ..Default::default()
        },
    );
    assert!(i.action_pressed("undo") && i.action_pressed("confirm"));
}

#[test]
fn axis_and_vector() {
    let mut r = resolver();
    let i = step(&mut r, keys(&[Key::D]));
    assert_eq!(i.action_axis("move_left", "move_right"), 1.0);
    let i = step(&mut r, keys(&[Key::A, Key::D]));
    assert_eq!(i.action_axis("move_left", "move_right"), 0.0, "both cancel");
    let i = step(&mut r, keys(&[Key::A, Key::W]));
    let v = i.action_vector("move_left", "move_right", "move_up", "move_down");
    assert!((v - Vec2::new(-1.0, -1.0).normalize()).length() < 1e-6);
    let i = step(&mut r, keys(&[Key::S]));
    assert_eq!(
        i.action_vector("move_left", "move_right", "move_up", "move_down"),
        Vec2::new(0.0, 1.0)
    );
}

#[test]
fn a_swipe_fires_once_when_it_crosses_the_distance() {
    let mut r = resolver();
    let f1 = step(&mut r, touch(1, 100.0, 100.0, TouchPhase::Started));
    assert_eq!(f1.swipe(), None);
    let f2 = step(&mut r, touch(1, 90.0, 105.0, TouchPhase::Moved));
    assert_eq!(f2.swipe(), None, "10 units: not yet");
    let f3 = step(&mut r, touch(1, 60.0, 110.0, TouchPhase::Moved));
    assert_eq!(f3.swipe(), Some(Swipe::Left));
    assert!(f3.action_pressed("move_left") && f3.action("move_left"));
    // One swipe per stroke, and the action lasts one frame.
    let f4 = step(&mut r, touch(1, 0.0, 110.0, TouchPhase::Moved));
    assert_eq!(f4.swipe(), None);
    assert!(f4.action_released("move_left"));
    let f5 = step(&mut r, touch(1, 0.0, 300.0, TouchPhase::Ended));
    assert_eq!(f5.swipe(), None);
}

#[test]
fn a_quick_flick_fires_on_the_frame_the_finger_lifts() {
    let mut r = resolver();
    step(&mut r, touch(3, 50.0, 200.0, TouchPhase::Started));
    let up = step(&mut r, touch(3, 52.0, 120.0, TouchPhase::Ended));
    assert_eq!(up.swipe(), Some(Swipe::Up));
    assert!(up.action_pressed("move_up"));
}

#[test]
fn taps_cancels_and_vanished_fingers_do_not_swipe() {
    let mut r = resolver();
    step(&mut r, touch(1, 0.0, 0.0, TouchPhase::Started));
    assert_eq!(
        step(&mut r, touch(1, 5.0, 0.0, TouchPhase::Ended)).swipe(),
        None
    );
    step(&mut r, touch(2, 0.0, 0.0, TouchPhase::Started));
    let c = step(&mut r, touch(2, 100.0, 0.0, TouchPhase::Cancelled));
    assert_eq!(c.swipe(), None);
    // Finger 4 starts, then the backend stops reporting it.
    step(&mut r, touch(4, 0.0, 0.0, TouchPhase::Started));
    step(&mut r, Input::default());
    assert_eq!(
        step(&mut r, touch(4, 0.0, 100.0, TouchPhase::Moved)).swipe(),
        None
    );
}

#[test]
fn mouse_drags_swipe_only_when_enabled() {
    let drag = |r: &mut ActionResolver| {
        step(
            r,
            Input {
                mouse_pos: Vec2::new(0.0, 0.0),
                mouse_left_pressed: true,
                mouse_left_down: true,
                ..Default::default()
            },
        );
        step(
            r,
            Input {
                mouse_pos: Vec2::new(0.0, 50.0),
                mouse_left_down: true,
                ..Default::default()
            },
        )
        .swipe()
    };
    assert_eq!(drag(&mut resolver()), None);
    let mut map = ActionMap::from_ron(MAP).unwrap();
    map.mouse_swipe = true;
    let mut r = ActionResolver::new(map);
    assert_eq!(drag(&mut r), Some(Swipe::Down));
    // Released and moved again without a press: nothing.
    step(
        &mut r,
        Input {
            mouse_left_released: true,
            ..Default::default()
        },
    );
    let after = step(
        &mut r,
        Input {
            mouse_pos: Vec2::new(200.0, 50.0),
            ..Default::default()
        },
    );
    assert_eq!(after.swipe(), None);
}

#[test]
fn swipe_direction_is_the_dominant_axis() {
    assert_eq!(Swipe::of(Vec2::new(-3.0, 2.0)), Swipe::Left);
    assert_eq!(Swipe::of(Vec2::new(3.0, -2.0)), Swipe::Right);
    assert_eq!(Swipe::of(Vec2::new(1.0, -2.0)), Swipe::Up);
    assert_eq!(Swipe::of(Vec2::new(1.0, 2.0)), Swipe::Down);
    assert_eq!(
        Swipe::of(Vec2::new(2.0, 2.0)),
        Swipe::Right,
        "tie: horizontal"
    );
}
