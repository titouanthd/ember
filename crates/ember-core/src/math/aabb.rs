// crates/ember-core/src/math/aabb.rs
//! Axis-Aligned Bounding Box (boîte alignée sur les axes).

use glam::Vec2;

/// Une boîte alignée sur les axes, définie par ses coins `min` et `max`.
///
/// Convention :
/// - `min.x <= max.x`
/// - `min.y <= max.y`
/// - En écran (Y vers le bas), `min.y` est le **haut** et `max.y` le **bas**.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: Vec2,
    pub max: Vec2,
}

impl Aabb {
    /// Crée une AABB à partir de deux coins. Les coins sont normalisés
    /// (min/max sont réordonnés si nécessaire).
    pub fn from_corners(a: Vec2, b: Vec2) -> Self {
        Self {
            min: Vec2::new(a.x.min(b.x), a.y.min(b.y)),
            max: Vec2::new(a.x.max(b.x), a.y.max(b.y)),
        }
    }

    /// Crée une AABB à partir du centre et de la demi-taille.
    pub fn from_center_half(center: Vec2, half: Vec2) -> Self {
        Self {
            min: center - half,
            max: center + half,
        }
    }

    /// Crée une AABB à partir du coin top-left et de la taille.
    pub fn from_top_left_size(top_left: Vec2, size: Vec2) -> Self {
        Self {
            min: top_left,
            max: top_left + size,
        }
    }

    /// Centre de l'AABB.
    pub fn center(&self) -> Vec2 {
        (self.min + self.max) / 2.0
    }

    /// Taille (largeur, hauteur).
    pub fn size(&self) -> Vec2 {
        self.max - self.min
    }

    /// Demi-taille.
    pub fn half_size(&self) -> Vec2 {
        self.size() / 2.0
    }

    /// Largeur.
    pub fn width(&self) -> f32 {
        self.max.x - self.min.x
    }

    /// Hauteur.
    pub fn height(&self) -> f32 {
        self.max.y - self.min.y
    }

    /// Vrai si le point est strictement à l'intérieur (bords exclus).
    pub fn contains_strict(&self, point: Vec2) -> bool {
        point.x > self.min.x
            && point.x < self.max.x
            && point.y > self.min.y
            && point.y < self.max.y
    }

    /// Vrai si le point est à l'intérieur ou sur les bords.
    pub fn contains(&self, point: Vec2) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }

    /// Vrai si deux AABB se chevauchent (bords inclusifs).
    pub fn overlaps(&self, other: &Aabb) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
    }

    /// Vrai si deux AABB se chevauchent strictement (bords exclusifs).
    pub fn overlaps_strict(&self, other: &Aabb) -> bool {
        self.min.x < other.max.x
            && self.max.x > other.min.x
            && self.min.y < other.max.y
            && self.max.y > other.min.y
    }

    /// Profondeur de pénétration si les deux AABB se chevauchent.
    /// Retourne `None` s'il n'y a pas de chevauchement.
    ///
    /// La profondeur est retournée sous forme `(Vec2, f32)` où `Vec2` est
    /// l'axe de séparation minimale et `f32` la profondeur sur cet axe.
    pub fn penetration(&self, other: &Aabb) -> Option<(Vec2, f32)> {
        let dx = (self.center().x - other.center().x).abs();
        let dy = (self.center().y - other.center().y).abs();
        let half_sum = self.half_size() + other.half_size();

        let overlap_x = half_sum.x - dx;
        let overlap_y = half_sum.y - dy;

        if overlap_x <= 0.0 || overlap_y <= 0.0 {
            return None;
        }

        if overlap_x < overlap_y {
            Some((Vec2::new(1.0, 0.0), overlap_x))
        } else {
            Some((Vec2::new(0.0, 1.0), overlap_y))
        }
    }

    /// Distance minimale d'un point à l'AABB (0 si le point est dedans).
    pub fn distance_to_point(&self, point: Vec2) -> f32 {
        let closest = self.closest_point(point);
        (point - closest).length()
    }

    /// Point le plus proche de l'AABB (sur les bords ou dedans).
    pub fn closest_point(&self, point: Vec2) -> Vec2 {
        Vec2::new(
            point.x.clamp(self.min.x, self.max.x),
            point.y.clamp(self.min.y, self.max.y),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_corners_normalizes() {
        let a = Aabb::from_corners(Vec2::new(10.0, 10.0), Vec2::new(0.0, 0.0));
        assert_eq!(a.min, Vec2::ZERO);
        assert_eq!(a.max, Vec2::new(10.0, 10.0));
    }

    #[test]
    fn test_from_center_half() {
        let a = Aabb::from_center_half(Vec2::new(5.0, 5.0), Vec2::new(5.0, 5.0));
        assert_eq!(a.min, Vec2::ZERO);
        assert_eq!(a.max, Vec2::new(10.0, 10.0));
    }

    #[test]
    fn test_from_top_left_size() {
        let a = Aabb::from_top_left_size(Vec2::new(2.0, 3.0), Vec2::new(10.0, 20.0));
        assert_eq!(a.min, Vec2::new(2.0, 3.0));
        assert_eq!(a.max, Vec2::new(12.0, 23.0));
    }

    #[test]
    fn test_center_size_half() {
        let a = Aabb::from_top_left_size(Vec2::new(0.0, 0.0), Vec2::new(10.0, 20.0));
        assert_eq!(a.center(), Vec2::new(5.0, 10.0));
        assert_eq!(a.size(), Vec2::new(10.0, 20.0));
        assert_eq!(a.half_size(), Vec2::new(5.0, 10.0));
    }

    #[test]
    fn test_contains_strict_excludes_borders() {
        let a = Aabb::from_top_left_size(Vec2::ZERO, Vec2::new(10.0, 10.0));
        assert!(a.contains_strict(Vec2::new(5.0, 5.0)));
        assert!(!a.contains_strict(Vec2::new(0.0, 5.0)));  // bord gauche
        assert!(!a.contains_strict(Vec2::new(10.0, 5.0))); // bord droit
    }

    #[test]
    fn test_contains_includes_borders() {
        let a = Aabb::from_top_left_size(Vec2::ZERO, Vec2::new(10.0, 10.0));
        assert!(a.contains(Vec2::new(5.0, 5.0)));
        assert!(a.contains(Vec2::new(0.0, 5.0)));
        assert!(a.contains(Vec2::new(10.0, 10.0)));
        assert!(!a.contains(Vec2::new(11.0, 5.0)));
    }

    #[test]
    fn test_overlaps() {
        let a = Aabb::from_top_left_size(Vec2::ZERO, Vec2::new(10.0, 10.0));
        let b = Aabb::from_top_left_size(Vec2::new(5.0, 5.0), Vec2::new(10.0, 10.0));
        let c = Aabb::from_top_left_size(Vec2::new(20.0, 0.0), Vec2::new(10.0, 10.0));
        assert!(a.overlaps(&b));
        assert!(!a.overlaps(&c));
    }

    #[test]
    fn test_overlaps_touching_borders_is_true() {
        let a = Aabb::from_top_left_size(Vec2::ZERO, Vec2::new(10.0, 10.0));
        let b = Aabb::from_top_left_size(Vec2::new(10.0, 0.0), Vec2::new(10.0, 10.0));
        assert!(a.overlaps(&b), "les bords qui se touchent comptent comme overlap");
    }

    #[test]
    fn test_overlaps_strict_touching_borders_is_false() {
        let a = Aabb::from_top_left_size(Vec2::ZERO, Vec2::new(10.0, 10.0));
        let b = Aabb::from_top_left_size(Vec2::new(10.0, 0.0), Vec2::new(10.0, 10.0));
        assert!(!a.overlaps_strict(&b));
    }

    #[test]
    fn test_penetration_horizontal() {
        let a = Aabb::from_top_left_size(Vec2::ZERO, Vec2::new(10.0, 10.0));
        let b = Aabb::from_top_left_size(Vec2::new(8.0, 0.0), Vec2::new(10.0, 10.0));
        let (axis, depth) = a.penetration(&b).expect("should overlap");
        assert_eq!(axis, Vec2::new(1.0, 0.0));
        assert!((depth - 2.0).abs() < 1e-5);
    }

    #[test]
    fn test_penetration_none_when_separate() {
        let a = Aabb::from_top_left_size(Vec2::ZERO, Vec2::new(10.0, 10.0));
        let b = Aabb::from_top_left_size(Vec2::new(20.0, 0.0), Vec2::new(10.0, 10.0));
        assert!(a.penetration(&b).is_none());
    }

    #[test]
    fn test_closest_point_inside() {
        let a = Aabb::from_top_left_size(Vec2::ZERO, Vec2::new(10.0, 10.0));
        assert_eq!(a.closest_point(Vec2::new(5.0, 5.0)), Vec2::new(5.0, 5.0));
    }

    #[test]
    fn test_closest_point_outside() {
        let a = Aabb::from_top_left_size(Vec2::ZERO, Vec2::new(10.0, 10.0));
        // À droite
        assert_eq!(a.closest_point(Vec2::new(20.0, 5.0)), Vec2::new(10.0, 5.0));
        // En haut à gauche
        assert_eq!(a.closest_point(Vec2::new(-5.0, -5.0)), Vec2::ZERO);
    }

    #[test]
    fn test_distance_to_point() {
        let a = Aabb::from_top_left_size(Vec2::ZERO, Vec2::new(10.0, 10.0));
        assert_eq!(a.distance_to_point(Vec2::new(5.0, 5.0)), 0.0);
        assert_eq!(a.distance_to_point(Vec2::new(15.0, 5.0)), 5.0);
    }
}