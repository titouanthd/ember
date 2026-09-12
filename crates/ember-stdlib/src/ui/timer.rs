//! Simple MM:SS timer display widget.

use macroquad::prelude::Color;

use super::label::Label;

#[derive(Debug, Clone)]
pub struct TimerDisplay {
    pub x: f32,
    pub y: f32,
    pub size: u16,
    pub color: Color,
}

impl TimerDisplay {
    pub fn new(x: f32, y: f32, size: u16, color: Color) -> Self {
        Self { x, y, size, color }
    }

    /// Format `seconds` as MM:SS (or HH:MM:SS if >= 1 hour).
    pub fn format(seconds: f32) -> String {
        let total = seconds.max(0.0) as u64;
        let h = total / 3600;
        let m = (total % 3600) / 60;
        let s = total % 60;
        if h > 0 {
            format!("{:02}:{:02}:{:02}", h, m, s)
        } else {
            format!("{:02}:{:02}", m, s)
        }
    }

    pub fn draw(&self, seconds: f32) {
        let text = Self::format(seconds);
        Label::new(text, self.x, self.y, self.size, self.color)
            .right()
            .draw();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_under_a_minute() {
        assert_eq!(TimerDisplay::format(5.0), "00:05");
        assert_eq!(TimerDisplay::format(59.9), "00:59");
    }

    #[test]
    fn test_format_minutes() {
        assert_eq!(TimerDisplay::format(65.0), "01:05");
        assert_eq!(TimerDisplay::format(3599.0), "59:59");
    }

    #[test]
    fn test_format_hours() {
        assert_eq!(TimerDisplay::format(3600.0), "01:00:00");
        assert_eq!(TimerDisplay::format(3661.0), "01:01:01");
    }

    #[test]
    fn test_format_negative_clamps_to_zero() {
        assert_eq!(TimerDisplay::format(-10.0), "00:00");
    }
}