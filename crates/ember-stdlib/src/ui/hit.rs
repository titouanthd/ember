use glam::Vec2;
use macroquad::prelude::Rect;

/// True if `p` is inside `rect` (borders inclusive).
pub fn contains(rect: Rect, p: Vec2) -> bool {
    p.x >= rect.x && p.x <= rect.x + rect.w
        && p.y >= rect.y && p.y <= rect.y + rect.h
}

/// Same, with a (x, y, w, h) tuple.
pub fn contains_xywh(x: f32, y: f32, w: f32, h: f32, p: Vec2) -> bool {
    p.x >= x && p.x <= x + w && p.y >= y && p.y <= y + h
}

/// Strict version — borders excluded.
pub fn contains_strict(rect: Rect, p: Vec2) -> bool {
    p.x > rect.x && p.x < rect.x + rect.w
        && p.y > rect.y && p.y < rect.y + rect.h
}