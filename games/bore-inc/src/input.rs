//! Per-frame input snapshot for the game.
//!
//! `Game::tick` takes one of these so the game logic is testable
//! headless — construct a `GameInput::default()` for a no-op frame,
//! or set fields directly.
//!
//! This is separate from the menu/title input helpers in `systems.rs`
//! (which need a macroquad context and are used for screen
//! transitions, not simulation).

use macroquad::prelude::*;

/// Direction Archie is facing — and therefore mining toward.
///
/// Arrow keys set this while **held**. Diagonal variants need both
/// arrows held (e.g. `↑` + `→` = `UpRight`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Facing {
    #[default]
    Right,
    Left,
    Up,
    Down,
    UpLeft,
    UpRight,
    DownLeft,
    DownRight,
}

impl Facing {
    /// True for any facing with a leftward component. Used by the
    /// animation selector to pick the L or R sprite.
    pub fn is_left(self) -> bool {
        matches!(self, Facing::Left | Facing::UpLeft | Facing::DownLeft)
    }
}

/// Per-frame input. All fields are 0 / false / None for a "no input"
/// frame.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct GameInput {
    /// Horizontal axis: -1 (left), 0 (nothing), +1 (right). Comes
    /// from the left/right arrow keys.
    pub horizontal: f32,

    /// Direction indicated by the currently-held arrows. `None` when
    /// no arrow is held. Diagonal when two perpendicular arrows are
    /// held at once.
    pub facing_held: Option<Facing>,

    /// Mine key held. Mines in `facing`.
    pub mine_held: bool,

    /// Space, edge-triggered.
    pub jump_pressed: bool,
}

impl GameInput {
    /// Snapshot the current keyboard state. Call once per frame.
    ///
    /// **Not unit-testable** — macroquad's input functions require an
    /// active context. Tests construct `GameInput::default()` or fill
    /// fields directly.
    pub fn from_keyboard() -> Self {
        let up = is_key_down(KeyCode::Up);
        let down = is_key_down(KeyCode::Down);
        let left = is_key_down(KeyCode::Left);
        let right = is_key_down(KeyCode::Right);

        let horizontal = match (left, right) {
            (true, false) => -1.0,
            (false, true) => 1.0,
            _ => 0.0,
        };

        let facing_held = match (up, down, left, right) {
            (true, false, true, false) => Some(Facing::UpLeft),
            (true, false, false, true) => Some(Facing::UpRight),
            (false, true, true, false) => Some(Facing::DownLeft),
            (false, true, false, true) => Some(Facing::DownRight),
            (true, false, _, _) => Some(Facing::Up),
            (false, true, _, _) => Some(Facing::Down),
            (_, _, true, false) => Some(Facing::Left),
            (_, _, false, true) => Some(Facing::Right),
            _ => None,
        };

        Self {
            horizontal,
            facing_held,
            // Accept both physical positions so the same key works on
            // QWERTY and AZERTY:
            //  - QWERTY: KeyCode::A = labeled `A`, KeyCode::Q = labeled `Q`
            //  - AZERTY: KeyCode::A = labeled `Q`, KeyCode::Q = labeled `A`
            mine_held: is_key_down(KeyCode::A) || is_key_down(KeyCode::Q),
            jump_pressed: is_key_pressed(KeyCode::Space),
        }
    }
}