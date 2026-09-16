//! Minimal immediate-mode UI.
//!
//! Design: no retained state. Each frame, the game creates widgets with
//! current positions, passes the input snapshot, and gets back events.
//! The game decides what to do with the events.
//!
//! IMPORTANT: a widget must be `update()`d and `draw()`n on the SAME
//! instance. Creating a fresh instance in the render path loses the
//! hover/press state computed by `update()`. Store widgets in the game's
//! state struct.

pub mod button;
pub mod circle_button;
pub mod label;
pub mod panel;
pub mod timer;

pub use button::{Button, ButtonEvent};
pub use circle_button::{CircleButton, CircleButtonEvent};
pub use label::Label;
pub use panel::Panel;
pub use timer::TimerDisplay;
