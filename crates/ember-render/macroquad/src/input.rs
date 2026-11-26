//! Per-frame `Input` snapshot built from macroquad's global state.
//! Needs a live macroquad context; not unit-testable.

use ember_input::{Input, Key, TouchPhase, TouchPoint};
use glam::Vec2;
use macroquad::input::{
    KeyCode, MouseButton, get_char_pressed, is_key_down, is_key_pressed, is_key_released,
    is_mouse_button_down, is_mouse_button_pressed, is_mouse_button_released, mouse_position,
    mouse_wheel, touches,
};

const KEYS: &[(KeyCode, Key)] = &[
    (KeyCode::Space, Key::Space),
    (KeyCode::Enter, Key::Enter),
    (KeyCode::Escape, Key::Escape),
    (KeyCode::Tab, Key::Tab),
    (KeyCode::Backspace, Key::Backspace),
    (KeyCode::Delete, Key::Delete),
    (KeyCode::Home, Key::Home),
    (KeyCode::End, Key::End),
    (KeyCode::PageUp, Key::PageUp),
    (KeyCode::PageDown, Key::PageDown),
    (KeyCode::Up, Key::Up),
    (KeyCode::Down, Key::Down),
    (KeyCode::Left, Key::Left),
    (KeyCode::Right, Key::Right),
    (KeyCode::A, Key::A),
    (KeyCode::B, Key::B),
    (KeyCode::C, Key::C),
    (KeyCode::D, Key::D),
    (KeyCode::E, Key::E),
    (KeyCode::F, Key::F),
    (KeyCode::G, Key::G),
    (KeyCode::H, Key::H),
    (KeyCode::I, Key::I),
    (KeyCode::J, Key::J),
    (KeyCode::K, Key::K),
    (KeyCode::L, Key::L),
    (KeyCode::M, Key::M),
    (KeyCode::N, Key::N),
    (KeyCode::O, Key::O),
    (KeyCode::P, Key::P),
    (KeyCode::Q, Key::Q),
    (KeyCode::R, Key::R),
    (KeyCode::S, Key::S),
    (KeyCode::T, Key::T),
    (KeyCode::U, Key::U),
    (KeyCode::V, Key::V),
    (KeyCode::W, Key::W),
    (KeyCode::X, Key::X),
    (KeyCode::Y, Key::Y),
    (KeyCode::Z, Key::Z),
    (KeyCode::Key0, Key::Num0),
    (KeyCode::Key1, Key::Num1),
    (KeyCode::Key2, Key::Num2),
    (KeyCode::Key3, Key::Num3),
    (KeyCode::Key4, Key::Num4),
    (KeyCode::Key5, Key::Num5),
    (KeyCode::Key6, Key::Num6),
    (KeyCode::Key7, Key::Num7),
    (KeyCode::Key8, Key::Num8),
    (KeyCode::Key9, Key::Num9),
    (KeyCode::F1, Key::F1),
    (KeyCode::F2, Key::F2),
    (KeyCode::F3, Key::F3),
    (KeyCode::F4, Key::F4),
    (KeyCode::F5, Key::F5),
    (KeyCode::F6, Key::F6),
    (KeyCode::F7, Key::F7),
    (KeyCode::F8, Key::F8),
    (KeyCode::F9, Key::F9),
    (KeyCode::F10, Key::F10),
    (KeyCode::F11, Key::F11),
    (KeyCode::F12, Key::F12),
    (KeyCode::Semicolon, Key::Semicolon),
    (KeyCode::LeftShift, Key::LeftShift),
    (KeyCode::RightShift, Key::RightShift),
    (KeyCode::LeftControl, Key::LeftControl),
    (KeyCode::RightControl, Key::RightControl),
];

/// Build this frame's `Input`. macroquad has no gamepad support, so the
/// gamepad lists stay empty.
pub fn poll_input() -> Input {
    let (mx, my) = mouse_position();
    let (wx, wy) = mouse_wheel();
    let mut input = Input {
        mouse_pos: Vec2::new(mx, my),
        mouse_wheel: Vec2::new(wx, wy),
        mouse_left_pressed: is_mouse_button_pressed(MouseButton::Left),
        mouse_left_down: is_mouse_button_down(MouseButton::Left),
        mouse_left_released: is_mouse_button_released(MouseButton::Left),
        mouse_right_pressed: is_mouse_button_pressed(MouseButton::Right),
        mouse_right_down: is_mouse_button_down(MouseButton::Right),
        mouse_right_released: is_mouse_button_released(MouseButton::Right),
        mouse_middle_pressed: is_mouse_button_pressed(MouseButton::Middle),
        mouse_middle_down: is_mouse_button_down(MouseButton::Middle),
        mouse_middle_released: is_mouse_button_released(MouseButton::Middle),
        ..Input::default()
    };
    for &(code, key) in KEYS {
        if is_key_pressed(code) {
            input.keys_pressed.push(key);
        }
        if is_key_down(code) {
            input.keys_down.push(key);
        }
        if is_key_released(code) {
            input.keys_released.push(key);
        }
    }
    // Drain macroquad's char queue: one call per typed char this frame.
    while let Some(c) = get_char_pressed() {
        if !c.is_control() {
            input.chars_typed.push(c);
        }
    }
    input.touch_points = touches()
        .into_iter()
        .map(|t| TouchPoint::new(t.id, t.position, phase(t.phase)))
        .collect();
    input
}

fn phase(p: macroquad::input::TouchPhase) -> TouchPhase {
    use macroquad::input::TouchPhase as Mq;
    match p {
        Mq::Started => TouchPhase::Started,
        Mq::Moved => TouchPhase::Moved,
        Mq::Stationary => TouchPhase::Stationary,
        Mq::Ended => TouchPhase::Ended,
        Mq::Cancelled => TouchPhase::Cancelled,
    }
}
