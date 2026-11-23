//! Signals emitted by built-in controls (through `UiRoot`, ADR-036).
//!
//! Signals are broadcast: every handler of the type runs, so handlers
//! check which control sent it.
//!
//! ```ignore
//! let play = ui.add(Button::new("Play"));           // NodeId
//! ctx.connect::<ButtonClicked>(menu_id, move |ctx, ev| {
//!     if ev.button == play {
//!         ctx.emit(StartGame);
//!     }
//! });
//! ```

use ember_scene::NodeId;

/// A `Button` was clicked or activated from the keyboard / gamepad.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ButtonClicked {
    pub button: NodeId,
}

/// Keyboard focus moved. `None` means nothing is focused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FocusChanged {
    pub from: Option<NodeId>,
    pub to: Option<NodeId>,
}

/// A `TextInput`'s text changed (typing, Backspace, Delete).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextChanged {
    pub input: NodeId,
    pub text: String,
}

/// Enter was pressed in a focused `TextInput`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextSubmitted {
    pub input: NodeId,
    pub text: String,
}
