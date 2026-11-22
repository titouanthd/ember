//! `Button`: a clickable text label (spec §5.2). Emits
//! [`ButtonClicked`](crate::ButtonClicked) through `UiRoot` (ADR-036).

use crate::{ButtonClicked, Control, ControlBase, Measure, UiDrawContext};
use ember_input::MouseButton;
use glam::Vec2;

/// Horizontal padding around the text, in pixels.
pub const BUTTON_PADDING_H: f32 = 16.0;
/// Vertical padding around the text, in pixels.
pub const BUTTON_PADDING_V: f32 = 8.0;

/// What the button looks like this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonState {
    #[default]
    Normal,
    Hover,
    Pressed,
    Disabled,
}

/// Click = left press and release both on the button. Enter / Space /
/// gamepad South click it while it has focus.
pub struct Button {
    name: String,
    base: ControlBase,
    pub text: String,
    /// `None` = `Theme::font_size`.
    pub size: Option<f32>,
    disabled: bool,
    state: ButtonState,
}

impl Button {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            name: "Button".into(),
            base: ControlBase::new(),
            text: text.into(),
            size: None,
            disabled: false,
            state: ButtonState::Normal,
        }
    }

    pub fn with_size(mut self, size: f32) -> Self {
        self.size = Some(size);
        self
    }

    pub fn disabled(mut self) -> Self {
        self.set_disabled(true);
        self
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// A disabled button ignores input and cannot take focus.
    pub fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
        self.state = if disabled {
            ButtonState::Disabled
        } else if self.base.hovered {
            ButtonState::Hover
        } else {
            ButtonState::Normal
        };
    }

    pub fn state(&self) -> ButtonState {
        self.state
    }

    fn click(&mut self) {
        self.base.emit(|id| ButtonClicked { button: id });
    }
}

impl Default for Button {
    fn default() -> Self {
        Self::new("")
    }
}

crate::widgets::ui_object!(Button, "Button");

impl Control for Button {
    fn base(&self) -> &ControlBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ControlBase {
        &mut self.base
    }

    fn measure(&self, _available: Vec2, m: &Measure) -> Vec2 {
        let size = self.size.unwrap_or(m.theme().font_size);
        let text = m.text(&self.text, size);
        let padded = Vec2::new(
            text.x + BUTTON_PADDING_H * 2.0,
            text.y + BUTTON_PADDING_V * 2.0,
        );
        padded.max(self.min_size())
    }

    fn focusable(&self) -> bool {
        !self.disabled
    }

    fn on_mouse_enter(&mut self) {
        if !self.disabled {
            self.state = if self.base.pressed {
                ButtonState::Pressed
            } else {
                ButtonState::Hover
            };
        }
    }

    fn on_mouse_leave(&mut self) {
        if !self.disabled {
            self.state = ButtonState::Normal;
        }
    }

    fn on_mouse_down(&mut self, button: MouseButton, _pos: Vec2) {
        if !self.disabled && button == MouseButton::Left {
            self.state = ButtonState::Pressed;
        }
    }

    fn on_mouse_up(&mut self, button: MouseButton, pos: Vec2) {
        if self.disabled || button != MouseButton::Left {
            return;
        }
        let inside = self.rect().contains(pos);
        // Only a press that started here counts (`pressed` is cleared by
        // the root right after this call).
        if inside && self.base.pressed {
            self.click();
        }
        self.state = if inside {
            ButtonState::Hover
        } else {
            ButtonState::Normal
        };
    }

    fn on_activate(&mut self) {
        if !self.disabled {
            self.click();
        }
    }

    fn draw_control(&self, ui: &mut UiDrawContext) {
        let rect = self.rect();
        let theme = ui.theme();
        let bg = theme.button_bg(self.state);
        let text_color = theme.button_text(self.state);
        let (border_width, border) = if ui.is_focused() {
            (2.0, theme.focus_border)
        } else {
            (1.0, theme.panel_border)
        };
        let size = self.size.unwrap_or(theme.font_size);
        ui.draw_background(rect, bg);
        ui.draw_border(rect, border_width, border);
        if self.text.is_empty() {
            return;
        }
        let metrics = ui.render().measure_text(&self.text, size);
        let top_left = Vec2::new(
            rect.x + (rect.w - metrics.width) * 0.5,
            rect.y + (rect.h - metrics.height) * 0.5,
        );
        // `draw_text` takes the baseline.
        let baseline = Vec2::new(top_left.x, top_left.y + metrics.offset_y);
        ui.draw_text(&self.text, baseline, size, text_color);
    }
}

crate::control_node!(Button);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabling_sets_the_state_and_blocks_focus() {
        let mut b = Button::new("Quit");
        assert!(b.focusable());
        b.set_disabled(true);
        assert_eq!(b.state(), ButtonState::Disabled);
        assert!(!b.focusable());
        b.on_mouse_enter();
        assert_eq!(b.state(), ButtonState::Disabled);
        b.set_disabled(false);
        assert_eq!(b.state(), ButtonState::Normal);
    }

    #[test]
    fn activation_queues_one_click() {
        let mut b = Button::new("Play");
        b.on_activate();
        assert_eq!(b.base().queued_signals(), 1);
        b.disabled = true;
        b.on_activate();
        assert_eq!(b.base().queued_signals(), 1);
    }

    #[test]
    fn release_without_press_does_not_click() {
        let mut b = Button::new("Play");
        b.base_mut().rect = ember_core::math::Rect::new(0.0, 0.0, 100.0, 40.0);
        b.on_mouse_up(MouseButton::Left, Vec2::new(10.0, 10.0));
        assert_eq!(b.base().queued_signals(), 0);
        b.base_mut().pressed = true;
        b.on_mouse_up(MouseButton::Left, Vec2::new(10.0, 10.0));
        assert_eq!(b.base().queued_signals(), 1);
        assert_eq!(b.state(), ButtonState::Hover);
    }
}
