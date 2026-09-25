//! Hit testing sur rectangles.
//!
//! Trois formes d'appel pour le même test, selon ce que le call site a
//! sous la main :
//!
//! - `contains(rect, p)` — quand on a un [`macroquad::prelude::Rect`]
//! - `contains_xywh(x, y, w, h, p)` — quand on a les 4 floats séparés
//! - `contains_tuple((x, y, w, h), p)` — quand on a un tuple `(f32, f32, f32, f32)`
//!
//! **Convention** : les bords sont **inclusifs** (`>=` / `<=`). C'est le
//! comportement historique de tous les jeux. Une variante stricte
//! (`contains_strict`) existe pour les cas où on veut exclure les bords.
//!
//! Les rectangles à largeur/hauteur négatives ne sont pas supportés.

use glam::Vec2;
use macroquad::prelude::Rect;

/// `true` si `p` est à l'intérieur de `rect` (bords inclusifs).
pub fn contains(rect: Rect, p: Vec2) -> bool {
    p.x >= rect.x
        && p.x <= rect.x + rect.w
        && p.y >= rect.y
        && p.y <= rect.y + rect.h
}

/// Idem, avec `(x, y, w, h)` séparés.
pub fn contains_xywh(x: f32, y: f32, w: f32, h: f32, p: Vec2) -> bool {
    p.x >= x && p.x <= x + w && p.y >= y && p.y <= y + h
}

/// Idem, avec un tuple `(x, y, w, h)`.
pub fn contains_tuple(r: (f32, f32, f32, f32), p: Vec2) -> bool {
    contains_xywh(r.0, r.1, r.2, r.3, p)
}

/// Variante stricte : les bords sont **exclus**.
pub fn contains_strict(rect: Rect, p: Vec2) -> bool {
    p.x > rect.x
        && p.x < rect.x + rect.w
        && p.y > rect.y
        && p.y < rect.y + rect.h
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn r() -> Rect {
        Rect::new(10.0, 20.0, 100.0, 50.0)
    }

    #[test]
    fn test_contains_inside() {
        assert!(contains(r(), Vec2::new(60.0, 45.0)));
    }

    #[test]
    fn test_contains_on_left_edge() {
        assert!(contains(r(), Vec2::new(10.0, 45.0)));
    }

    #[test]
    fn test_contains_on_right_edge() {
        assert!(contains(r(), Vec2::new(110.0, 45.0)));
    }

    #[test]
    fn test_contains_on_top_edge() {
        assert!(contains(r(), Vec2::new(60.0, 20.0)));
    }

    #[test]
    fn test_contains_on_bottom_edge() {
        assert!(contains(r(), Vec2::new(60.0, 70.0)));
    }

    #[test]
    fn test_contains_on_corner() {
        assert!(contains(r(), Vec2::new(10.0, 20.0)));
        assert!(contains(r(), Vec2::new(110.0, 20.0)));
        assert!(contains(r(), Vec2::new(10.0, 70.0)));
        assert!(contains(r(), Vec2::new(110.0, 70.0)));
    }

    #[test]
    fn test_contains_outside() {
        assert!(!contains(r(), Vec2::new(9.9, 45.0)));
        assert!(!contains(r(), Vec2::new(110.1, 45.0)));
        assert!(!contains(r(), Vec2::new(60.0, 19.9)));
        assert!(!contains(r(), Vec2::new(60.0, 70.1)));
        assert!(!contains(r(), Vec2::new(0.0, 0.0)));
    }

    #[test]
    fn test_contains_strict_inside() {
        assert!(contains_strict(r(), Vec2::new(60.0, 45.0)));
    }

    #[test]
    fn test_contains_strict_excludes_edges() {
        assert!(!contains_strict(r(), Vec2::new(10.0, 45.0)));
        assert!(!contains_strict(r(), Vec2::new(110.0, 45.0)));
        assert!(!contains_strict(r(), Vec2::new(60.0, 20.0)));
        assert!(!contains_strict(r(), Vec2::new(60.0, 70.0)));
        assert!(!contains_strict(r(), Vec2::new(10.0, 20.0)));
    }

    #[test]
    fn test_contains_xywh_matches_rect() {
        let rect = r();
        let p_inside = Vec2::new(60.0, 45.0);
        let p_outside = Vec2::new(0.0, 0.0);
        assert_eq!(
            contains(rect, p_inside),
            contains_xywh(rect.x, rect.y, rect.w, rect.h, p_inside),
        );
        assert_eq!(
            contains(rect, p_outside),
            contains_xywh(rect.x, rect.y, rect.w, rect.h, p_outside),
        );
    }

    #[test]
    fn test_contains_tuple_matches_rect() {
        let rect = r();
        let tuple = (rect.x, rect.y, rect.w, rect.h);
        let p = Vec2::new(60.0, 45.0);
        assert_eq!(contains(rect, p), contains_tuple(tuple, p));
    }
}