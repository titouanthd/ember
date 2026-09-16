//! Clickable button widget.

use glam::Vec2;
use macroquad::prelude::{Color, draw_rectangle, draw_rectangle_lines, draw_text};

use crate::input::Input;

/// What happened to the button this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonEvent {
    /// Nothing.
    None,
    /// Mouse entered the button this frame.
    Hovered,
    /// Mouse is over the button and left button is held down.
    Held,
    /// Mouse released over the button this frame.
    Clicked,
}

/// A simple rectangular button. Fields are public so the game can style it.
#[derive(Debug, Clone)]
pub struct Button {
    pub rect: (f32, f32, f32, f32), // x, y, w, h
    pub label: String,
    pub enabled: bool,
}

impl Button {
    pub fn new(x: f32, y: f32, w: f32, h: f32, label: impl Into<String>) -> Self {
        Self {
            rect: (x, y, w, h),
            label: label.into(),
            enabled: true,
        }
    }

    pub fn contains(&self, p: Vec2) -> bool {
        let (x, y, w, h) = self.rect;
        p.x >= x && p.x <= x + w && p.y >= y && p.y <= y + h
    }

    /// Evaluate the button against this frame's input.
    pub fn update(&self, input: &Input) -> ButtonEvent {
        if !self.enabled {
            return ButtonEvent::None;
        }
        let over = self.contains(input.mouse_pos);
        if !over {
            return ButtonEvent::None;
        }
        if input.mouse_left_pressed {
            return ButtonEvent::Hovered; // pressed-but-not-yet-released
        }
        if input.mouse_left_down {
            return ButtonEvent::Held;
        }
        if input.mouse_left_released {
            return ButtonEvent::Clicked;
        }
        ButtonEvent::Hovered
    }

    /// Render the button with three colors: idle, hover, pressed.
    pub fn draw(&self, input: &Input, idle: Color, hover: Color, pressed: Color, text: Color) {
        let (x, y, w, h) = self.rect;
        let over = self.enabled && self.contains(input.mouse_pos);
        let down = over && input.mouse_left_down;
        let bg = if !self.enabled {
            Color::new(idle.r * 0.5, idle.g * 0.5, idle.b * 0.5, idle.a)
        } else if down {
            pressed
        } else if over {
            hover
        } else {
            idle
        };
        draw_rectangle(x, y, w, h, bg);
        draw_rectangle_lines(x, y, w, h, 1.0, Color::new(1.0, 1.0, 1.0, 0.3));

        // Center the label. macroquad's draw_text measures width.
        let font_size = 20.0;
        let text_w = macroquad::text::measure_text(&self.label, None, font_size as u16, 1.0).width;
        let tx = x + (w - text_w) * 0.5;
        let ty = y + (h + font_size) * 0.5 - 2.0;
        draw_text(&self.label, tx, ty, font_size, text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn btn() -> Button {
        Button::new(10.0, 10.0, 100.0, 40.0, "OK")
    }

    #[test]
    fn test_contains_inside() {
        let b = btn();
        assert!(b.contains(Vec2::new(50.0, 30.0)));
    }

    #[test]
    fn test_contains_edges() {
        let b = btn();
        assert!(b.contains(Vec2::new(10.0, 10.0)));
        assert!(b.contains(Vec2::new(110.0, 50.0)));
    }

    #[test]
    fn test_contains_outside() {
        let b = btn();
        assert!(!b.contains(Vec2::new(0.0, 0.0)));
        assert!(!b.contains(Vec2::new(200.0, 30.0)));
    }

    #[test]
    fn test_no_hover_outside() {
        let b = btn();
        let input = Input {
            mouse_pos: Vec2::new(500.0, 500.0),
            mouse_left_pressed: true,
            ..Default::default()
        };
        assert_eq!(b.update(&input), ButtonEvent::None);
    }

    #[test]
    fn test_hover_only() {
        let b = btn();
        let input = Input {
            mouse_pos: Vec2::new(50.0, 30.0),
            ..Default::default()
        };
        assert_eq!(b.update(&input), ButtonEvent::Hovered);
    }

    #[test]
    fn test_held() {
        let b = btn();
        let input = Input {
            mouse_pos: Vec2::new(50.0, 30.0),
            mouse_left_down: true,
            ..Default::default()
        };
        assert_eq!(b.update(&input), ButtonEvent::Held);
    }

    #[test]
    fn test_clicked_on_release() {
        let b = btn();
        let input = Input {
            mouse_pos: Vec2::new(50.0, 30.0),
            mouse_left_released: true,
            ..Default::default()
        };
        assert_eq!(b.update(&input), ButtonEvent::Clicked);
    }

    #[test]
    fn test_disabled_returns_none() {
        let mut b = btn();
        b.enabled = false;
        let input = Input {
            mouse_pos: Vec2::new(50.0, 30.0),
            mouse_left_released: true,
            ..Default::default()
        };
        assert_eq!(b.update(&input), ButtonEvent::None);
    }
}
