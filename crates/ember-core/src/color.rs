//! Backend-agnostic RGBA color.
//!
//! `#[repr(C)]` so backends can cast to their own color type without
//! a per-call conversion. Every engine crate talks in `ember_core::Color`;
//! `ember_render_macroquad` is the only crate that converts to
//! `macroquad::Color`.

/// RGBA color, each channel in `[0.0, 1.0]`.
///
/// Construction: [`Color::new`], [`Color::rgb`], [`Color::rgba`], or the
/// `WHITE`/`BLACK`/... constants.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[repr(C)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const WHITE: Self = Self::rgb(1.0, 1.0, 1.0);
    pub const BLACK: Self = Self::rgb(0.0, 0.0, 0.0);
    pub const RED: Self = Self::rgb(1.0, 0.0, 0.0);
    pub const GREEN: Self = Self::rgb(0.0, 1.0, 0.0);
    pub const BLUE: Self = Self::rgb(0.0, 0.0, 1.0);
    pub const YELLOW: Self = Self::rgb(1.0, 1.0, 0.0);
    pub const TRANSPARENT: Self = Self::new(0.0, 0.0, 0.0, 0.0);

    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::new(r, g, b, 1.0)
    }

    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self::new(r, g, b, a)
    }

    pub fn with_alpha(self, a: f32) -> Self {
        Self::new(self.r, self.g, self.b, a)
    }

    pub fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    pub fn from_array([r, g, b, a]: [f32; 4]) -> Self {
        Self::new(r, g, b, a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_sets_alpha_to_one() {
        let c = Color::rgb(0.1, 0.2, 0.3);
        assert_eq!(c.a, 1.0);
    }

    #[test]
    fn with_alpha_replaces_alpha_only() {
        let c = Color::rgb(0.4, 0.5, 0.6).with_alpha(0.25);
        assert_eq!(c.r, 0.4);
        assert_eq!(c.a, 0.25);
    }

    #[test]
    fn array_roundtrip() {
        let c = Color::new(0.1, 0.2, 0.3, 0.4);
        assert_eq!(Color::from_array(c.to_array()), c);
    }

    #[test]
    fn constants_are_well_formed() {
        assert_eq!(Color::WHITE, Color::rgb(1.0, 1.0, 1.0));
        assert_eq!(Color::TRANSPARENT.a, 0.0);
    }
}
