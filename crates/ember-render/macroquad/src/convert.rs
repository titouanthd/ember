//! Conversions between ember-core types and macroquad types.
//!
//! Both sides are `#[repr(C)] { f32, f32, ... }` for `Color` and
//! `Rect`, but we construct field-by-field rather than transmuting.
//! The cost is two loads and two stores — the compiler inlines them
//! into nothing. Correctness over cleverness.
//!
//! `Vec2` is `glam::Vec2` on both sides; macroquad re-exports it, so
//! no conversion is needed.

use ember_core::Color as EmberColor;
use ember_core::math::Rect as EmberRect;
use macroquad::prelude::{Color as MqColor, Rect as MqRect};

#[inline]
pub fn color_to_mq(c: EmberColor) -> MqColor {
    MqColor::new(c.r, c.g, c.b, c.a)
}

#[inline]
#[allow(dead_code)]
pub fn color_from_mq(c: MqColor) -> EmberColor {
    EmberColor::new(c.r, c.g, c.b, c.a)
}

#[inline]
pub fn rect_to_mq(r: EmberRect) -> MqRect {
    MqRect::new(r.x, r.y, r.w, r.h)
}

#[inline]
#[allow(dead_code)]
pub fn rect_from_mq(r: MqRect) -> EmberRect {
    EmberRect::new(r.x, r.y, r.w, r.h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_roundtrip() {
        let ember = EmberColor::new(0.1, 0.2, 0.3, 0.4);
        let mq = color_to_mq(ember);
        assert_eq!(mq.r, 0.1);
        assert_eq!(mq.g, 0.2);
        assert_eq!(mq.b, 0.3);
        assert_eq!(mq.a, 0.4);
        assert_eq!(color_from_mq(mq), ember);
    }

    #[test]
    fn rect_roundtrip() {
        let ember = EmberRect::new(10.0, 20.0, 30.0, 40.0);
        let mq = rect_to_mq(ember);
        assert_eq!(mq.x, 10.0);
        assert_eq!(mq.y, 20.0);
        assert_eq!(mq.w, 30.0);
        assert_eq!(mq.h, 40.0);
        assert_eq!(rect_from_mq(mq), ember);
    }

    #[test]
    fn constants_survive_roundtrip() {
        assert_eq!(
            color_from_mq(color_to_mq(EmberColor::WHITE)),
            EmberColor::WHITE
        );
        assert_eq!(
            color_from_mq(color_to_mq(EmberColor::BLACK)),
            EmberColor::BLACK
        );
    }
}
