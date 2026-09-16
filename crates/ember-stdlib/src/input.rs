//! Frame input snapshot, decoupled from macroquad.
//!
//! `main.rs` builds one per frame from macroquad calls, then passes it to
//! game systems. Tests construct one directly.

use glam::Vec2;
use macroquad::prelude::{
    KeyCode, MouseButton, is_key_pressed, is_mouse_button_down, is_mouse_button_pressed,
    is_mouse_button_released, mouse_position,
};

/// Snapshot of all relevant input for one frame.
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
    pub keys_pressed: Vec<KeyCode>,
    pub keys_down: Vec<KeyCode>,
    pub keys_released: Vec<KeyCode>,
}

impl Input {
    /// Build from macroquad's global state. Call once per frame.
    ///
    /// **Does not populate `keys_pressed`, `keys_down`, or `keys_released`.**
    /// The caller must push the keys it cares about explicitly, or use
    /// [`Self::from_macroquad_with_keys`] instead.
    pub fn from_macroquad() -> Self {
        let (mx, my) = mouse_position();
        Self {
            mouse_pos: Vec2::new(mx, my),
            mouse_left_pressed: is_mouse_button_pressed(MouseButton::Left),
            mouse_left_down: is_mouse_button_down(MouseButton::Left),
            mouse_left_released: is_mouse_button_released(MouseButton::Left),
            mouse_right_pressed: is_mouse_button_pressed(MouseButton::Right),
            mouse_right_down: is_mouse_button_down(MouseButton::Right),
            mouse_right_released: is_mouse_button_released(MouseButton::Right),
            mouse_middle_pressed: is_mouse_button_pressed(MouseButton::Middle),
            mouse_middle_down: is_mouse_button_down(MouseButton::Middle),
            mouse_middle_released: is_mouse_button_released(MouseButton::Middle),
            keys_pressed: Vec::new(), // populated by caller if needed
            keys_down: Vec::new(),
            keys_released: Vec::new(),
        }
    }

    /// Like [`Self::from_macroquad`], but also populates `keys_pressed`
    /// with the keys from `keys` that were pressed this frame.
    ///
    /// `keys_down` and `keys_released` are **not** populated (no game needs
    /// them via `Input` right now). Games that need `is_key_down` can call
    /// `macroquad::prelude::is_key_down` directly.
    ///
    /// **Not unit-testable**: macroquad's input functions require an active
    /// macroquad context (a `#[macroquad::main]` or equivalent). This is the
    /// same constraint as [`AudioClip::load`](crate::audio::AudioClip::load).
    ///
    /// # Example
    /// ```ignore
    /// let input = Input::from_macroquad_with_keys(&[
    ///     KeyCode::R, KeyCode::Escape, KeyCode::Enter,
    /// ]);
    /// if input.is_key_pressed(KeyCode::R) { /* ... */ }
    /// ```
    pub fn from_macroquad_with_keys(keys: &[KeyCode]) -> Self {
        let mut input = Self::from_macroquad();
        for &k in keys {
            if is_key_pressed(k) {
                input.keys_pressed.push(k);
            }
        }
        input
    }

    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.keys_pressed.contains(&key)
    }

    pub fn is_key_down(&self, key: KeyCode) -> bool {
        self.keys_down.contains(&key)
    }

    pub fn is_key_released(&self, key: KeyCode) -> bool {
        self.keys_released.contains(&key)
    }

    /// Helper: check if a key is either pressed or held. Used by the game
    /// loop for keys that should be "sticky" (like R for restart).
    pub fn pressed_or_down(&self, key: KeyCode) -> bool {
        self.is_key_pressed(key) || self.is_key_down(key)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_all_false() {
        let i = Input::default();
        assert!(!i.mouse_left_pressed);
        assert!(!i.mouse_left_down);
        assert!(!i.mouse_right_pressed);
        assert_eq!(i.mouse_pos, Vec2::ZERO);
    }

    #[test]
    fn test_key_pressed_query() {
        let i = Input {
            keys_pressed: vec![KeyCode::Space],
            ..Default::default()
        };
        assert!(i.is_key_pressed(KeyCode::Space));
        assert!(!i.is_key_pressed(KeyCode::Enter));
    }

    #[test]
    fn test_key_down_query() {
        let i = Input {
            keys_down: vec![KeyCode::A],
            ..Default::default()
        };
        assert!(i.is_key_down(KeyCode::A));
        assert!(!i.is_key_down(KeyCode::B));
    }

    #[test]
    fn test_pressed_or_down() {
        let i = Input {
            keys_pressed: vec![KeyCode::R],
            ..Default::default()
        };
        assert!(i.pressed_or_down(KeyCode::R));
        assert!(!i.pressed_or_down(KeyCode::Q));

        let j = Input {
            keys_down: vec![KeyCode::R],
            ..Default::default()
        };
        assert!(j.pressed_or_down(KeyCode::R));
    }
}
