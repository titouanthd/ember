//! Backend-agnostic input snapshot.
//!
//! The `Input` struct is produced once per frame by the backend (Phase 5)
//! and consumed everywhere (`NodeContext`, game systems, `ember::run`).
//!
//! Two APIs cover the common cases:
//!
//! - [`Input::primary_action`] — "did the player confirm?" (Space / Enter /
//!   Left click / Gamepad South / touch). One line replaces three.
//! - [`Input::secondary_action`] — "did the player cancel?" (Escape /
//!   Backspace / Right click / Gamepad East).
//!
//! Everything else is a direct query: [`Input::is_key_pressed`],
//! [`Input::is_mouse_pressed`], [`Input::is_gamepad_pressed`], plus
//! iteration over [`TouchPoint`]s.

pub mod gamepad;
pub mod input;
pub mod key;
pub mod mouse;
pub mod prelude;
pub mod touch;

pub use gamepad::GamepadButton;
pub use input::{Input, InputSink};
pub use key::Key;
pub use mouse::MouseButton;
pub use touch::{TouchPhase, TouchPoint};
