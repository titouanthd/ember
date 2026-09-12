// crates/ember-core/src/math/circle.rs
//! Cercle 2D.

use crate::math::Aabb;
use glam::Vec2;

/// Un cercle défini par son centre et son rayon.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Circle {
    pub center: Vec2,
    pub radius: f32,
}

impl Circle {
    /// Crée un cercle.
    pub fn new(center: Vec2, radius: f32) -> Self {
        Self { center, radius }
    }

    /// Diamètre du cercle.
    pub fn diameter(&self) -> f32 {
        self.radius * 2.0
    }

    /// Aire du cercle.
    pub fn area(&self) -> f32 {
        std::f32::consts::PI * self.radius * self.radius
    }

    /// Circonférence.
    pub fn circumference(&self) -> f32 {
        std::f32::consts::TAU * self.radius
    }

    /// Vrai si le point est strictement à l'intérieur (bord exclu).
    pub fn contains_strict(&self, point: Vec2) -> bool {
        (point - self.center).length_squared() < self.radius * self.radius
    }

    /// Vrai si le point est à l'intérieur ou sur le bord.
    pub fn contains(&self, point: Vec2) -> bool {
        (point - self.center).length_squared() <= self.radius * self.radius
    }

    /// Vrai si deux cercles se chevauchent (bords inclusifs).
    pub fn overlaps(&self, other: &Circle) -> bool {
        let dist_sq = (self.center - other.center).length_squared();
        let radius_sum = self.radius + other.radius;
        dist_sq <= radius_sum * radius_sum
    }

    /// Vrai si le cercle chevauche une AABB (bords inclusifs).
    pub fn overlaps_aabb(&self, aabb: &Aabb) -> bool {
        let closest = aabb.closest_point(self.center);
        (self.center - closest).length_squared() <= self.radius * self.radius
    }

    /// Point le plus proche du cercle sur son bord, depuis un point externe.
    /// Si le point est au centre, retourne un point arbitraire sur le bord (vers +X).
    pub fn closest_point_on_edge(&self, point: Vec2) -> Vec2 {
        let dir = point - self.center;
        if dir.length_squared() < 1e-10 {
            self.center + Vec2::new(self.radius, 0.0)
        } else {
            self.center + dir.normalize() * self.radius
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diameter_area_circumference() {
        let c = Circle::new(Vec2::ZERO, 5.0);
        assert_eq!(c.diameter(), 10.0);
        assert!((c.area() - std::f32::consts::PI * 25.0).abs() < 1e-4);
        assert!((c.circumference() - std::f32::consts::TAU * 5.0).abs() < 1e-4);
    }

    #[test]
    fn test_contains_strict() {
        let c = Circle::new(Vec2::ZERO, 5.0);
        assert!(c.contains_strict(Vec2::new(3.0, 0.0)));
        assert!(!c.contains_strict(Vec2::new(5.0, 0.0)));   // sur le bord
        assert!(!c.contains_strict(Vec2::new(6.0, 0.0)));   // dehors
    }

    #[test]
    fn test_contains_includes_border() {
        let c = Circle::new(Vec2::ZERO, 5.0);
        assert!(c.contains(Vec2::new(3.0, 0.0)));
        assert!(c.contains(Vec2::new(5.0, 0.0)));
        assert!(!c.contains(Vec2::new(6.0, 0.0)));
    }

    #[test]
    fn test_circle_overlaps_circle() {
        let a = Circle::new(Vec2::ZERO, 5.0);
        let b = Circle::new(Vec2::new(8.0, 0.0), 5.0);   // dist 8, somme 10
        let c = Circle::new(Vec2::new(15.0, 0.0), 5.0);  // dist 15, somme 10
        assert!(a.overlaps(&b));
        assert!(!a.overlaps(&c));
    }

    #[test]
    fn test_circle_overlaps_aabb_face() {
        let c = Circle::new(Vec2::new(15.0, 5.0), 6.0); // centre à droite, rayon 6
        let a = Aabb::from_top_left_size(Vec2::ZERO, Vec2::new(10.0, 10.0));
        // closest = (10, 5), dist = 5, rayon = 6 → overlap
        assert!(c.overlaps_aabb(&a));
    }

    #[test]
    fn test_circle_overlaps_aabb_corner() {
        let c = Circle::new(Vec2::new(12.0, 12.0), 4.0); // près du coin (10, 10)
        let a = Aabb::from_top_left_size(Vec2::ZERO, Vec2::new(10.0, 10.0));
        // closest = (10, 10), dist = sqrt(4+4) ≈ 2.83, rayon = 4 → overlap
        assert!(c.overlaps_aabb(&a));
    }

    #[test]
    fn test_circle_does_not_overlap_aabb() {
        let c = Circle::new(Vec2::new(20.0, 5.0), 4.0);
        let a = Aabb::from_top_left_size(Vec2::ZERO, Vec2::new(10.0, 10.0));
        // closest = (10, 5), dist = 10, rayon = 4 → pas d'overlap
        assert!(!c.overlaps_aabb(&a));
    }

    #[test]
    fn test_closest_point_on_edge() {
        let c = Circle::new(Vec2::ZERO, 5.0);
        let p = c.closest_point_on_edge(Vec2::new(10.0, 0.0));
        assert!((p - Vec2::new(5.0, 0.0)).length() < 1e-5);
    }

    #[test]
    fn test_closest_point_on_edge_from_center() {
        let c = Circle::new(Vec2::ZERO, 5.0);
        let p = c.closest_point_on_edge(Vec2::ZERO);
        // Fallback : (5, 0)
        assert!((p - Vec2::new(5.0, 0.0)).length() < 1e-5);
    }
}