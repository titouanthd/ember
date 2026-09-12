//! Minimal immediate-mode UI.
//!
//! Design: no retained state. Each frame, the game creates widgets with
//! current positions, passes the input snapshot, and gets back events.
//! The game decides what to do with the events. Widgets are drawn by the
//! game (or by a render helper) — the UI module only handles hit-testing
//! and state transitions.

pub mod button;
pub mod label;
pub mod panel;
pub mod timer;

pub use button::{Button, ButtonEvent};
pub use label::Label;
pub use panel::Panel;
pub use timer::TimerDisplay;