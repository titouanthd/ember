//! Simple text label with alignment.

use macroquad::prelude::{draw_text, Color};
use macroquad::text::TextDimensions;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone)]
pub struct Label {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub size: u16,
    pub align: Align,
    pub color: Color,
}

impl Label {
    pub fn new(text: impl Into<String>, x: f32, y: f32, size: u16, color: Color) -> Self {
        Self {
            text: text.into(),
            x,
            y,
            size,
            align: Align::Left,
            color,
        }
    }

    pub fn centered(mut self) -> Self {
        self.align = Align::Center;
        self
    }

    pub fn right(mut self) -> Self {
        self.align = Align::Right;
        self
    }

    pub fn draw(&self) -> TextDimensions {
        match self.align {
            Align::Left => {
                draw_text(&self.text, self.x, self.y, self.size as f32, self.color)
            }
            Align::Center => {
                let dims = macroquad::text::measure_text(&self.text, None, self.size, 1.0);
                let tx = self.x - dims.width * 0.5;
                draw_text(&self.text, tx, self.y, self.size as f32, self.color)
            }
            Align::Right => {
                let dims = macroquad::text::measure_text(&self.text, None, self.size, 1.0);
                let tx = self.x - dims.width;
                draw_text(&self.text, tx, self.y, self.size as f32, self.color)
            }
        }
    }
}