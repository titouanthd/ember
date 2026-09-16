// crates/ember-core/tests/collision.rs
//! Tests d'intégration pour les collisions AABB.
//!
//! ⚠️ Ces tests utilisaient l'ancienne fonction `aabb_collision` (8 floats).
//! Migrés vers `Aabb::overlaps` (struct propre).

use ember_core::math::Aabb;
use glam::Vec2;

#[test]
fn test_aabb_overlap_cases() {
    let a = Aabb::from_top_left_size(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));

    // Chevauchement partiel
    let b = Aabb::from_top_left_size(Vec2::new(5.0, 5.0), Vec2::new(10.0, 10.0));
    assert!(a.overlaps(&b));

    // Séparé horizontalement
    let c = Aabb::from_top_left_size(Vec2::new(15.0, 0.0), Vec2::new(10.0, 10.0));
    assert!(!a.overlaps(&c));

    // Séparé verticalement
    let d = Aabb::from_top_left_size(Vec2::new(0.0, 15.0), Vec2::new(10.0, 10.0));
    assert!(!a.overlaps(&d));

    // Bords qui se touchent (inclusif)
    let e = Aabb::from_top_left_size(Vec2::new(10.0, 0.0), Vec2::new(10.0, 10.0));
    assert!(a.overlaps(&e));
}
