//! Contract tests. Every behavior a game or engine crate relies on.

use ember_input::{GamepadButton, Input, Key, MouseButton, TouchPhase, TouchPoint};
use glam::Vec2;

fn key_pressed(k: Key) -> Input {
    Input {
        keys_pressed: vec![k],
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// Default state
// ---------------------------------------------------------------------------

#[test]
fn default_input_has_no_activity() {
    let i = Input::default();
    assert_eq!(i.mouse_pos, Vec2::ZERO);
    assert!(!i.mouse_left_pressed);
    assert!(!i.mouse_right_down);
    assert!(!i.mouse_middle_released);
    assert!(i.keys_pressed.is_empty());
    assert!(i.gamepad_pressed.is_empty());
    assert!(i.touch_points.is_empty());
    assert!(!i.primary_action());
    assert!(!i.secondary_action());
}

// ---------------------------------------------------------------------------
// Key queries
// ---------------------------------------------------------------------------

#[test]
fn key_pressed_query() {
    let i = key_pressed(Key::Space);
    assert!(i.is_key_pressed(Key::Space));
    assert!(!i.is_key_pressed(Key::Enter));
    assert!(!i.is_key_down(Key::Space)); // different vector
}

#[test]
fn key_down_query_is_independent_of_pressed() {
    let i = Input {
        keys_down: vec![Key::A],
        ..Default::default()
    };
    assert!(i.is_key_down(Key::A));
    assert!(!i.is_key_pressed(Key::A));
}

#[test]
fn key_released_query() {
    let i = Input {
        keys_released: vec![Key::Escape],
        ..Default::default()
    };
    assert!(i.is_key_released(Key::Escape));
}

#[test]
fn pressed_or_down_covers_both() {
    let i = key_pressed(Key::R);
    assert!(i.pressed_or_down(Key::R));

    let j = Input {
        keys_down: vec![Key::R],
        ..Default::default()
    };
    assert!(j.pressed_or_down(Key::R));

    assert!(!Input::default().pressed_or_down(Key::R));
}

#[test]
fn unknown_key_with_code_is_queryable() {
    let i = key_pressed(Key::Unknown(42));
    assert!(i.is_key_pressed(Key::Unknown(42)));
    assert!(!i.is_key_pressed(Key::Unknown(43)));
}

// ---------------------------------------------------------------------------
// Mouse queries
// ---------------------------------------------------------------------------

#[test]
fn mouse_button_queries() {
    let i = Input {
        mouse_left_pressed: true,
        mouse_right_down: true,
        mouse_middle_released: true,
        ..Default::default()
    };
    assert!(i.is_mouse_pressed(MouseButton::Left));
    assert!(!i.is_mouse_pressed(MouseButton::Right));
    assert!(i.is_mouse_down(MouseButton::Right));
    assert!(i.is_mouse_released(MouseButton::Middle));
}

// ---------------------------------------------------------------------------
// Gamepad queries
// ---------------------------------------------------------------------------

#[test]
fn gamepad_queries() {
    let i = Input {
        gamepad_pressed: vec![GamepadButton::South],
        gamepad_down: vec![GamepadButton::South, GamepadButton::East],
        gamepad_released: vec![GamepadButton::East],
        ..Default::default()
    };
    assert!(i.is_gamepad_pressed(GamepadButton::South));
    assert!(!i.is_gamepad_pressed(GamepadButton::East));
    assert!(i.is_gamepad_down(GamepadButton::South));
    assert!(i.is_gamepad_down(GamepadButton::East));
    assert!(i.is_gamepad_released(GamepadButton::East));
}

// ---------------------------------------------------------------------------
// Touch queries
// ---------------------------------------------------------------------------

#[test]
fn touch_started_iterator() {
    let i = Input {
        touch_points: vec![
            TouchPoint::new(1, Vec2::new(10.0, 10.0), TouchPhase::Started),
            TouchPoint::new(2, Vec2::new(20.0, 20.0), TouchPhase::Moved),
        ],
        ..Default::default()
    };
    assert_eq!(i.touches_started().count(), 1);
    assert_eq!(i.touches_active().count(), 2);
}

#[test]
fn touch_active_excludes_ended_and_cancelled() {
    let i = Input {
        touch_points: vec![
            TouchPoint::new(1, Vec2::ZERO, TouchPhase::Ended),
            TouchPoint::new(2, Vec2::ZERO, TouchPhase::Cancelled),
            TouchPoint::new(3, Vec2::ZERO, TouchPhase::Stationary),
        ],
        ..Default::default()
    };
    assert_eq!(i.touches_started().count(), 0);
    assert_eq!(i.touches_active().count(), 1);
}

// ---------------------------------------------------------------------------
// primary_action (ADR-010)
// ---------------------------------------------------------------------------

#[test]
fn primary_action_from_space() {
    assert!(key_pressed(Key::Space).primary_action());
}

#[test]
fn primary_action_from_enter() {
    assert!(key_pressed(Key::Enter).primary_action());
}

#[test]
fn primary_action_from_left_click() {
    let i = Input {
        mouse_left_pressed: true,
        ..Default::default()
    };
    assert!(i.primary_action());
}

#[test]
fn primary_action_from_gamepad_south() {
    let i = Input {
        gamepad_pressed: vec![GamepadButton::South],
        ..Default::default()
    };
    assert!(i.primary_action());
}

#[test]
fn primary_action_from_touch_started() {
    let i = Input {
        touch_points: vec![TouchPoint::new(1, Vec2::ZERO, TouchPhase::Started)],
        ..Default::default()
    };
    assert!(i.primary_action());
}

#[test]
fn primary_action_ignores_irrelevant_input() {
    // A key that's not in the primary set.
    assert!(!key_pressed(Key::A).primary_action());
    // Right click is secondary, not primary.
    let right = Input {
        mouse_right_pressed: true,
        ..Default::default()
    };
    assert!(!right.primary_action());
    // Gamepad East is secondary.
    let east = Input {
        gamepad_pressed: vec![GamepadButton::East],
        ..Default::default()
    };
    assert!(!east.primary_action());
}

// ---------------------------------------------------------------------------
// secondary_action (ADR-010)
// ---------------------------------------------------------------------------

#[test]
fn secondary_action_from_escape() {
    assert!(key_pressed(Key::Escape).secondary_action());
}

#[test]
fn secondary_action_from_backspace() {
    assert!(key_pressed(Key::Backspace).secondary_action());
}

#[test]
fn secondary_action_from_right_click() {
    let i = Input {
        mouse_right_pressed: true,
        ..Default::default()
    };
    assert!(i.secondary_action());
}

#[test]
fn secondary_action_from_gamepad_east() {
    let i = Input {
        gamepad_pressed: vec![GamepadButton::East],
        ..Default::default()
    };
    assert!(i.secondary_action());
}

#[test]
fn secondary_action_ignores_irrelevant_input() {
    assert!(!key_pressed(Key::A).secondary_action());
    let left = Input {
        mouse_left_pressed: true,
        ..Default::default()
    };
    assert!(!left.secondary_action());
}

#[test]
fn primary_and_secondary_are_mutually_exclusive_for_most_inputs() {
    // Space is primary only.
    let space = key_pressed(Key::Space);
    assert!(space.primary_action());
    assert!(!space.secondary_action());

    // Escape is secondary only.
    let esc = key_pressed(Key::Escape);
    assert!(!esc.primary_action());
    assert!(esc.secondary_action());
}

// ---------------------------------------------------------------------------
// Snapshot semantics: construct as fixture, use as read-only
// ---------------------------------------------------------------------------

#[test]
fn input_can_be_constructed_as_a_test_fixture() {
    // This test proves ADR-018: `Input` is not `#[non_exhaustive]`,
    // so games can construct it with struct-update syntax.
    let input = Input {
        mouse_pos: Vec2::new(100.0, 50.0),
        mouse_left_pressed: true,
        keys_pressed: vec![Key::Space],
        ..Default::default()
    };
    assert_eq!(input.mouse_pos, Vec2::new(100.0, 50.0));
    assert!(input.primary_action());
}

#[test]
fn input_is_cloneable() {
    let i = Input {
        mouse_pos: Vec2::new(1.0, 2.0),
        ..Default::default()
    };
    let j = i.clone();
    assert_eq!(j.mouse_pos, Vec2::new(1.0, 2.0));
}

// ---------------------------------------------------------------------------
// Text and wheel (ADR-032)
// ---------------------------------------------------------------------------

#[test]
fn text_and_wheel_default_to_nothing() {
    let i = Input::default();
    assert!(i.chars_typed.is_empty());
    assert_eq!(i.typed_text(), "");
    assert_eq!(i.mouse_wheel, Vec2::ZERO);
    assert!(!i.wheel_moved());
}

#[test]
fn typed_text_keeps_order_and_unicode() {
    let i = Input {
        chars_typed: vec!['é', 't', 'é', ' ', '春'],
        ..Default::default()
    };
    assert_eq!(i.typed_text(), "été 春");
}

#[test]
fn wheel_moved_on_either_axis() {
    let up = Input {
        mouse_wheel: Vec2::new(0.0, 1.0),
        ..Default::default()
    };
    let side = Input {
        mouse_wheel: Vec2::new(-2.0, 0.0),
        ..Default::default()
    };
    assert!(up.wheel_moved());
    assert!(side.wheel_moved());
}

#[test]
fn editing_keys_exist_and_differ() {
    let keys = [
        Key::Delete,
        Key::Home,
        Key::End,
        Key::PageUp,
        Key::PageDown,
        Key::Backspace,
    ];
    for (n, a) in keys.iter().enumerate() {
        for b in &keys[n + 1..] {
            assert_ne!(a, b);
        }
    }
    let i = Input {
        keys_pressed: vec![Key::Home],
        ..Default::default()
    };
    assert!(i.is_key_pressed(Key::Home));
    assert!(!i.is_key_pressed(Key::End));
}
