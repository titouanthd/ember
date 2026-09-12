//! Rectangular container with a background and optional border.

use macroquad::prelude::{draw_rectangle, draw_rectangle_lines, Color};

#[derive(Debug, Clone)]
pub struct Panel {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub bg: Color,
    pub border: Option<Color>,
}

impl Panel {
    pub fn new(x: f32, y: f32, w: f32, h: f32, bg: Color) -> Self {
        Self { x, y, w, h, bg, border: None }
    }

    pub fn with_border(mut self, color: Color) -> Self {
        self.border = Some(color);
        self
    }

    pub fn draw(&self) {
        draw_rectangle(self.x, self.y, self.w, self.h, self.bg);
        if let Some(b) = self.border {
            draw_rectangle_lines(self.x, self.y, self.w, self.h, 1.0, b);
        }
    }
}