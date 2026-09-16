// games/asteroids/src/components.rs
//! Composants de jeu : vaisseau, balle, astéroïde.

use ember_stdlib::{Collider, Shape, Sprite, Transform};
use glam::Vec2;
use macroquad::prelude::Color;

// ============================================================================
// Ship (vaisseau)
// ============================================================================

/// Le vaisseau du joueur.
///
/// `transform.rotation` = direction (0 = pointe vers +X, convention écran).
/// La vélocité est stockée à côté pour l'inertie.
#[derive(Debug, Clone)]
pub struct Ship {
    pub transform: Transform,
    pub sprite: Sprite,
    pub collider: Collider,
    pub velocity: Vec2,
}

impl Ship {
    pub fn new(position: Vec2, radius: f32, color: Color) -> Self {
        let transform = Transform::new(position, 0.0, Vec2::splat(radius));
        let sprite = Sprite::new(color);
        let collider = Collider::new(Shape::Circle { radius });
        Self {
            transform,
            sprite,
            collider,
            velocity: Vec2::ZERO,
        }
    }

    pub fn radius(&self) -> f32 {
        self.transform.scale.x
    }

    /// Direction "avant" du vaisseau (vecteur unitaire).
    pub fn forward(&self) -> Vec2 {
        Vec2::new(self.transform.rotation.cos(), self.transform.rotation.sin())
    }

    /// Pointe du vaisseau (là où spawn les balles).
    pub fn nose(&self) -> Vec2 {
        self.transform.position + self.forward() * self.radius()
    }
}

// ============================================================================
// Bullet
// ============================================================================

/// Une balle tirée par le vaisseau.
#[derive(Debug, Clone)]
pub struct Bullet {
    pub transform: Transform,
    pub sprite: Sprite,
    pub collider: Collider,
    pub velocity: Vec2,
    /// Temps de vie restant (en secondes). 0 → la balle meurt.
    pub lifetime: f32,
}

impl Bullet {
    pub fn new(position: Vec2, velocity: Vec2, radius: f32, lifetime: f32, color: Color) -> Self {
        let angle = velocity.y.atan2(velocity.x);
        let transform = Transform::new(position, angle, Vec2::splat(radius));
        let sprite = Sprite::new(color);
        let collider = Collider::new(Shape::Circle { radius });
        Self {
            transform,
            sprite,
            collider,
            velocity,
            lifetime,
        }
    }

    pub fn radius(&self) -> f32 {
        self.transform.scale.x
    }

    pub fn is_alive(&self) -> bool {
        self.lifetime > 0.0
    }
}

// ============================================================================
// Asteroid
// ============================================================================

/// Taille d'un astéroïde (3 = grand, 2 = moyen, 1 = petit).
pub type AsteroidSize = u8;

/// Un astéroïde qui dérive dans l'espace.
#[derive(Debug, Clone)]
pub struct Asteroid {
    pub transform: Transform,
    pub sprite: Sprite,
    pub collider: Collider,
    pub velocity: Vec2,
    /// Vitesse angulaire (rad/s) pour un effet visuel.
    pub spin: f32,
    /// Taille : 3, 2 ou 1.
    pub size: AsteroidSize,
}

impl Asteroid {
    pub fn new(
        position: Vec2,
        velocity: Vec2,
        spin: f32,
        size: AsteroidSize,
        radius: f32,
        color: Color,
    ) -> Self {
        let transform = Transform::new(position, 0.0, Vec2::splat(radius));
        let sprite = Sprite::new(color);
        let collider = Collider::new(Shape::Circle { radius });
        Self {
            transform,
            sprite,
            collider,
            velocity,
            spin,
            size,
        }
    }

    pub fn radius(&self) -> f32 {
        self.transform.scale.x
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ship_forward_at_zero_rotation() {
        let ship = Ship::new(Vec2::ZERO, 10.0, Color::new(1.0, 1.0, 1.0, 1.0));
        let f = ship.forward();
        assert!((f.x - 1.0).abs() < 1e-5);
        assert!(f.y.abs() < 1e-5);
    }

    #[test]
    fn test_ship_forward_at_90_degrees() {
        let mut ship = Ship::new(Vec2::ZERO, 10.0, Color::new(1.0, 1.0, 1.0, 1.0));
        ship.transform.rotation = std::f32::consts::FRAC_PI_2;
        let f = ship.forward();
        assert!(f.x.abs() < 1e-5);
        assert!((f.y - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_ship_nose_is_ahead() {
        let mut ship = Ship::new(
            Vec2::new(100.0, 100.0),
            10.0,
            Color::new(1.0, 1.0, 1.0, 1.0),
        );
        ship.transform.rotation = 0.0;
        let nose = ship.nose();
        assert!((nose.x - 110.0).abs() < 1e-5);
        assert!((nose.y - 100.0).abs() < 1e-5);
    }

    #[test]
    fn test_bullet_is_alive() {
        let b = Bullet::new(
            Vec2::ZERO,
            Vec2::new(100.0, 0.0),
            3.0,
            1.0,
            Color::new(1.0, 1.0, 1.0, 1.0),
        );
        assert!(b.is_alive());
    }

    #[test]
    fn test_bullet_dies_when_lifetime_zero() {
        let mut b = Bullet::new(
            Vec2::ZERO,
            Vec2::new(100.0, 0.0),
            3.0,
            1.0,
            Color::new(1.0, 1.0, 1.0, 1.0),
        );
        b.lifetime = 0.0;
        assert!(!b.is_alive());
    }

    #[test]
    fn test_asteroid_radius() {
        let a = Asteroid::new(
            Vec2::ZERO,
            Vec2::ZERO,
            0.0,
            3,
            40.0,
            Color::new(1.0, 1.0, 1.0, 1.0),
        );
        assert_eq!(a.radius(), 40.0);
    }
}
