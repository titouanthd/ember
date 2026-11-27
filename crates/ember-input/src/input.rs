//! Per-frame input snapshot.
//!
//! The backend produces one `Input` at the start of each frame (Phase 5).
//! Every node reads the same snapshot for the whole frame, so "was Space
//! pressed this frame?" has one answer regardless of when in the frame
//! a node asks.
//!
//! Games construct `Input` directly as a test fixture:
//! ```
//! # use ember_input::Input;
//! # use ember_input::Key;
//! # use glam::Vec2;
//! let input = Input {
//!     mouse_pos: Vec2::new(100.0, 50.0),
//!     mouse_left_pressed: true,
//!     ..Default::default()
//! };
//! ```
//! The struct is intentionally **not** `#[non_exhaustive]` — see
//! ADR-018. Games construct it as a fixture.

use crate::gamepad::GamepadButton;
use crate::key::Key;
use crate::mouse::MouseButton;
use crate::touch::TouchPoint;
use glam::Vec2;

/// A complete snapshot of input state for one frame.
///
/// Every field is a delta since the previous frame (`pressed`, `released`)
/// or the persistent state at the moment of capture (`down`). A key that
/// was pressed on frame N is in `keys_pressed` on frame N, in `keys_down`
/// on frames N, N+1, ..., and in `keys_released` on frame N+K.
#[derive(Debug, Clone, Default)]
pub struct Input {
    pub mouse_pos: Vec2,
    pub mouse_left_pressed: bool,
    pub mouse_left_down: bool,
    pub mouse_left_released: bool,
    pub mouse_right_pressed: bool,
    pub mouse_right_down: bool,
    pub mouse_right_released: bool,
    pub mouse_middle_pressed: bool,
    pub mouse_middle_down: bool,
    pub mouse_middle_released: bool,
    pub keys_pressed: Vec<Key>,
    pub keys_down: Vec<Key>,
    pub keys_released: Vec<Key>,
    pub gamepad_pressed: Vec<GamepadButton>,
    pub gamepad_down: Vec<GamepadButton>,
    pub gamepad_released: Vec<GamepadButton>,
    pub touch_points: Vec<TouchPoint>,
    /// Text typed this frame, in order, as produced by the OS keyboard
    /// layout (so `é`, `Q` on AZERTY, shifted symbols are already right).
    /// Control characters (Backspace, Enter, Tab, Escape…) are excluded:
    /// read those through `keys_pressed`. ADR-032.
    pub chars_typed: Vec<char>,
    /// Mouse-wheel movement this frame, in the backend's notches/lines.
    /// `y > 0` means the wheel moved up (away from the user); `x` is
    /// horizontal scrolling. `Vec2::ZERO` when the wheel did not move.
    pub mouse_wheel: Vec2,
    /// The window's close button was pressed this frame. Only reported
    /// when the window was opened with `WindowConfig::confirm_close`;
    /// otherwise the window just closes (ADR-045).
    pub close_requested: bool,
}

impl Input {
    // --- Key queries ---

    /// True if `key` was pressed **this frame**.
    pub fn is_key_pressed(&self, key: Key) -> bool {
        self.keys_pressed.contains(&key)
    }

    /// True if `key` is currently held down.
    pub fn is_key_down(&self, key: Key) -> bool {
        self.keys_down.contains(&key)
    }

    /// True if `key` was released **this frame**.
    pub fn is_key_released(&self, key: Key) -> bool {
        self.keys_released.contains(&key)
    }

    /// Convenience: pressed-or-held. Used for "sticky" keys (R for
    /// restart) where holding the key is as valid as pressing it.
    pub fn pressed_or_down(&self, key: Key) -> bool {
        self.is_key_pressed(key) || self.is_key_down(key)
    }

    // --- Mouse queries ---

    /// True if `button` was pressed this frame.
    pub fn is_mouse_pressed(&self, button: MouseButton) -> bool {
        match button {
            MouseButton::Left => self.mouse_left_pressed,
            MouseButton::Right => self.mouse_right_pressed,
            MouseButton::Middle => self.mouse_middle_pressed,
        }
    }

    pub fn is_mouse_down(&self, button: MouseButton) -> bool {
        match button {
            MouseButton::Left => self.mouse_left_down,
            MouseButton::Right => self.mouse_right_down,
            MouseButton::Middle => self.mouse_middle_down,
        }
    }

    pub fn is_mouse_released(&self, button: MouseButton) -> bool {
        match button {
            MouseButton::Left => self.mouse_left_released,
            MouseButton::Right => self.mouse_right_released,
            MouseButton::Middle => self.mouse_middle_released,
        }
    }

    // --- Gamepad queries ---

    pub fn is_gamepad_pressed(&self, button: GamepadButton) -> bool {
        self.gamepad_pressed.contains(&button)
    }

    pub fn is_gamepad_down(&self, button: GamepadButton) -> bool {
        self.gamepad_down.contains(&button)
    }

    pub fn is_gamepad_released(&self, button: GamepadButton) -> bool {
        self.gamepad_released.contains(&button)
    }

    // --- Text and wheel (ADR-032) ---

    /// This frame's typed text as a `String` (empty when nothing was typed).
    pub fn typed_text(&self) -> String {
        self.chars_typed.iter().collect()
    }

    /// True if the mouse wheel moved this frame, on either axis.
    pub fn wheel_moved(&self) -> bool {
        self.mouse_wheel != Vec2::ZERO
    }

    // --- Touch queries ---

    /// Iterate every touch point that started **this frame**.
    pub fn touches_started(&self) -> impl Iterator<Item = &TouchPoint> {
        self.touch_points
            .iter()
            .filter(|t| t.phase == crate::touch::TouchPhase::Started)
    }

    /// Iterate every active (non-ended, non-cancelled) touch point.
    pub fn touches_active(&self) -> impl Iterator<Item = &TouchPoint> {
        self.touch_points.iter().filter(|t| {
            matches!(
                t.phase,
                crate::touch::TouchPhase::Started
                    | crate::touch::TouchPhase::Moved
                    | crate::touch::TouchPhase::Stationary
            )
        })
    }

    // --- Convenience actions (ADR-010) ---

    /// True if the player pressed the game's primary "confirm" action.
    ///
    /// Any of:
    /// - Space / Enter (keyboard)
    /// - Left mouse button
    /// - Gamepad South (Xbox A, PS Cross, Switch B)
    /// - Any touch point that started this frame
    ///
    /// See `50-design-decisions.md` ADR-010.
    pub fn primary_action(&self) -> bool {
        self.is_key_pressed(Key::Space)
            || self.is_key_pressed(Key::Enter)
            || self.mouse_left_pressed
            || self.is_gamepad_pressed(GamepadButton::South)
            || self.touches_started().next().is_some()
    }

    /// True if the player pressed the game's secondary "cancel" action.
    ///
    /// Any of:
    /// - Escape / Backspace (keyboard)
    /// - Right mouse button
    /// - Gamepad East (Xbox B, PS Circle, Switch A)
    ///
    /// See `50-design-decisions.md` ADR-010.
    pub fn secondary_action(&self) -> bool {
        self.is_key_pressed(Key::Escape)
            || self.is_key_pressed(Key::Backspace)
            || self.mouse_right_pressed
            || self.is_gamepad_pressed(GamepadButton::East)
    }
}

/// Implemented by anything a runner feeds a fresh `Input` snapshot to
/// once per frame, before `process`.
pub trait InputSink {
    fn set_input(&mut self, input: Input);
}
