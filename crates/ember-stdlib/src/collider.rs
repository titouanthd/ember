// crates/ember-stdlib/src/collider.rs
//! Composants et fonctions de collision.
//!
//! Ce module s'appuie sur `ember_core::math::{Aabb, Circle}` pour les
//! primitives géométriques, et ajoute :
//! - `Collider` : composant de collision (forme + état actif).
//! - `Contact` : information de contact (normale + pénétration).
//! - `collides` : test booléen (compatibilité).
//! - `collide_contact` : test détaillé (normale + profondeur).
//! - `swept_circle_vs_aabb` / `swept_circle_vs_obb` : physique continue.

use ember_core::math::{Aabb, Circle};
use glam::Vec2;

// ============================================================================
// Composants
// ============================================================================

/// Forme géométrique pour la détection de collision.
///
/// Toutes les formes sont centrées sur la position passée à `collides` /
/// `collide_contact`. **Il n'y a plus d'offset** : le call site passe le
/// centre du collider (typiquement `entity.position + entity.scale / 2`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shape {
    /// Boîte alignée sur les axes, définie par sa demi-taille.
    Aabb { half_size: Vec2 },
    /// Cercle défini par son rayon.
    Circle { radius: f32 },
}

impl Shape {
    /// Convertit la forme en `Aabb` centrée à l'origine.
    pub fn as_aabb(&self) -> Aabb {
        match self {
            Shape::Aabb { half_size } => Aabb::from_center_half(Vec2::ZERO, *half_size),
            Shape::Circle { radius } => {
                Aabb::from_center_half(Vec2::ZERO, Vec2::splat(*radius))
            }
        }
    }

    /// Convertit la forme en `Circle` (approximation pour AABB).
    pub fn as_circle(&self) -> Circle {
        match self {
            Shape::Aabb { half_size } => {
                Circle::new(Vec2::ZERO, half_size.min_element())
            }
            Shape::Circle { radius } => Circle::new(Vec2::ZERO, *radius),
        }
    }
}

/// Composant de collision.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Collider {
    pub shape: Shape,
    pub active: bool,
}

impl Collider {
    /// Crée un nouveau collider actif.
    pub fn new(shape: Shape) -> Self {
        Self { shape, active: true }
    }

    /// Active ou désactive le collider.
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }
}

/// Information de contact entre deux formes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Contact {
    /// Normale du contact, pointant de la forme B vers la forme A.
    pub normal: Vec2,
    /// Profondeur de pénétration sur l'axe de la normale.
    pub penetration: f32,
}

// ============================================================================
// Détection de collision
// ============================================================================

/// Test booléen de collision entre deux colliders.
///
/// `pos_a` et `pos_b` sont les **centres** des colliders.
pub fn collides(
    pos_a: Vec2,
    collider_a: &Collider,
    pos_b: Vec2,
    collider_b: &Collider,
) -> bool {
    collide_contact(pos_a, collider_a, pos_b, collider_b).is_some()
}

/// Test détaillé : retourne `Some(Contact)` si collision, `None` sinon.
///
/// `pos_a` et `pos_b` sont les **centres** des colliders.
pub fn collide_contact(
    pos_a: Vec2,
    collider_a: &Collider,
    pos_b: Vec2,
    collider_b: &Collider,
) -> Option<Contact> {
    if !collider_a.active || !collider_b.active {
        return None;
    }

    match (collider_a.shape, collider_b.shape) {
        (Shape::Aabb { half_size: hs_a }, Shape::Aabb { half_size: hs_b }) => {
            aabb_vs_aabb_contact(pos_a, hs_a, pos_b, hs_b)
        }
        (Shape::Aabb { half_size: hs_a }, Shape::Circle { radius: r_b }) => {
            aabb_vs_circle_contact(pos_a, hs_a, pos_b, r_b)
        }
        (Shape::Circle { radius: r_a }, Shape::Aabb { half_size: hs_b }) => {
            // On inverse : AABB vs Circle, puis on inverse la normale.
            aabb_vs_circle_contact(pos_b, hs_b, pos_a, r_a)
                .map(|c| Contact { normal: -c.normal, penetration: c.penetration })
        }
        (Shape::Circle { radius: r_a }, Shape::Circle { radius: r_b }) => {
            circle_vs_circle_contact(pos_a, r_a, pos_b, r_b)
        }
    }
}

/// AABB vs AABB : retourne la normale et la profondeur minimale.
fn aabb_vs_aabb_contact(
    center_a: Vec2,
    half_a: Vec2,
    center_b: Vec2,
    half_b: Vec2,
) -> Option<Contact> {
    let a = Aabb::from_center_half(center_a, half_a);
    let b = Aabb::from_center_half(center_b, half_b);

    if !a.overlaps_strict(&b) {
        return None;
    }

    let (axis, depth) = a.penetration(&b)?;
    // La normale doit pointer de B vers A.
    let dir = if axis.x != 0.0 {
        (center_a.x - center_b.x).signum()
    } else {
        (center_a.y - center_b.y).signum()
    };
    let normal = axis * dir;
    Some(Contact { normal, penetration: depth })
}

/// AABB vs Cercle : normale pointe du cercle vers l'AABB.
fn aabb_vs_circle_contact(
    aabb_center: Vec2,
    aabb_half: Vec2,
    circle_center: Vec2,
    circle_radius: f32,
) -> Option<Contact> {
    let aabb = Aabb::from_center_half(aabb_center, aabb_half);
    let circle = Circle::new(circle_center, circle_radius);

    if !circle.overlaps_aabb(&aabb) {
        return None;
    }

    let closest = aabb.closest_point(circle_center);
    let diff = closest - circle_center;
    let dist_sq = diff.length_squared();

    if dist_sq < 1e-10 {
        // Le centre du cercle est dans l'AABB : normale par défaut.
        // On prend la face la plus proche.
        let to_left = (circle_center.x - aabb.min.x).abs();
        let to_right = (circle_center.x - aabb.max.x).abs();
        let to_top = (circle_center.y - aabb.min.y).abs();
        let to_bottom = (circle_center.y - aabb.max.y).abs();
        let min = to_left.min(to_right).min(to_top).min(to_bottom);
        let normal = if (min - to_left).abs() < 1e-5 {
            Vec2::new(-1.0, 0.0)
        } else if (min - to_right).abs() < 1e-5 {
            Vec2::new(1.0, 0.0)
        } else if (min - to_top).abs() < 1e-5 {
            Vec2::new(0.0, -1.0)
        } else {
            Vec2::new(0.0, 1.0)
        };
        return Some(Contact { normal, penetration: circle_radius + min });
    }

    let dist = dist_sq.sqrt();
    let normal = diff / dist;
    let penetration = circle_radius - dist;
    Some(Contact { normal, penetration })
}

/// Cercle vs Cercle.
fn circle_vs_circle_contact(
    center_a: Vec2,
    radius_a: f32,
    center_b: Vec2,
    radius_b: f32,
) -> Option<Contact> {
    let diff = center_b - center_a;
    let dist_sq = diff.length_squared();
    let radius_sum = radius_a + radius_b;

    if dist_sq > radius_sum * radius_sum {
        return None;
    }

    let dist = dist_sq.sqrt();
    let normal = if dist < 1e-10 {
        Vec2::new(1.0, 0.0)
    } else {
        diff / dist
    };
    let penetration = radius_sum - dist;
    Some(Contact { normal, penetration })
}

// ============================================================================
// Swept collisions (physique continue)
// ============================================================================

/// Swept collision : cercle en mouvement vs AABB statique.
///
/// Trouve le **premier** contact d'un cercle (pos, vel, radius) avec une
/// AABB centrée à l'origine. Retourne `None` si pas de contact sur `[0, 1]`.
///
/// `max_t` : fraction du déplacement à considérer (1.0 = tout le pas).
///
/// # Méthode
/// Slab method : on gonfle l'AABB par le rayon, puis on cherche le
/// premier intervalle où le centre du cercle est dedans.
pub fn swept_circle_vs_aabb(
    pos: Vec2,
    vel: Vec2,
    radius: f32,
    half: Vec2,
    max_t: f32,
) -> Option<Contact> {
    let expanded = half + Vec2::splat(radius);

    // Déjà en contact ?
    if pos.x.abs() < expanded.x && pos.y.abs() < expanded.y {
        let dx = expanded.x - pos.x.abs();
        let dy = expanded.y - pos.y.abs();
        let (normal, penetration) = if dx < dy {
            let sign = if pos.x < 0.0 { -1.0 } else { 1.0 };
            (Vec2::new(sign, 0.0), dx)
        } else {
            let sign = if pos.y < 0.0 { -1.0 } else { 1.0 };
            (Vec2::new(0.0, sign), dy)
        };
        return Some(Contact { normal, penetration });
    }

    let inv_vx = if vel.x.abs() > 1e-8 { 1.0 / vel.x } else { f32::INFINITY };
    let inv_vy = if vel.y.abs() > 1e-8 { 1.0 / vel.y } else { f32::INFINITY };

    let tx1 = (-expanded.x - pos.x) * inv_vx;
    let tx2 = ( expanded.x - pos.x) * inv_vx;
    let (tx_near, tx_far) = if tx1 < tx2 { (tx1, tx2) } else { (tx2, tx1) };

    let ty1 = (-expanded.y - pos.y) * inv_vy;
    let ty2 = ( expanded.y - pos.y) * inv_vy;
    let (ty_near, ty_far) = if ty1 < ty2 { (ty1, ty2) } else { (ty2, ty1) };

    let t_near = tx_near.max(ty_near);
    let t_far  = tx_far.min(ty_far);

    if t_near > t_far || t_far < 0.0 || t_near > max_t {
        return None;
    }

    let t = t_near.max(0.0);
    let normal = if tx_near > ty_near {
        Vec2::new(if vel.x > 0.0 { -1.0 } else { 1.0 }, 0.0)
    } else {
        Vec2::new(0.0, if vel.y > 0.0 { -1.0 } else { 1.0 })
    };

    Some(Contact { normal, penetration: t })
}

/// Swept collision : cercle en mouvement vs OBB (rectangle tourné).
///
/// On transforme dans le repère local de l'OBB, on fait la collision AABB,
/// puis on remet la normale dans le repère monde.
pub fn swept_circle_vs_obb(
    circle_pos: Vec2,
    circle_vel: Vec2,
    circle_radius: f32,
    obb_center: Vec2,
    obb_half: Vec2,
    obb_angle: f32,
    max_t: f32,
) -> Option<Contact> {
    let (s, c) = obb_angle.sin_cos();

    let rel = circle_pos - obb_center;
    let local_pos = Vec2::new(
         rel.x * c + rel.y * s,
        -rel.x * s + rel.y * c,
    );
    let local_vel = Vec2::new(
         circle_vel.x * c + circle_vel.y * s,
        -circle_vel.x * s + circle_vel.y * c,
    );

    let contact_local = swept_circle_vs_aabb(local_pos, local_vel, circle_radius, obb_half, max_t)?;

    let world_normal = Vec2::new(
        contact_local.normal.x * c - contact_local.normal.y * s,
        contact_local.normal.x * s + contact_local.normal.y * c,
    );

    Some(Contact {
        normal: world_normal,
        penetration: contact_local.penetration,
    })
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- Helpers ---
    fn aabb_collider(w: f32, h: f32) -> Collider {
        Collider::new(Shape::Aabb { half_size: Vec2::new(w / 2.0, h / 2.0) })
    }
    fn circle_collider(r: f32) -> Collider {
        Collider::new(Shape::Circle { radius: r })
    }

    // --- AABB vs AABB ---
    #[test]
    fn test_aabb_vs_aabb_overlap() {
        let a = aabb_collider(10.0, 10.0);
        let b = aabb_collider(10.0, 10.0);
        assert!(collides(Vec2::ZERO, &a, Vec2::new(3.0, 3.0), &b));
    }

    #[test]
    fn test_aabb_vs_aabb_no_overlap() {
        let a = aabb_collider(10.0, 10.0);
        let b = aabb_collider(10.0, 10.0);
        assert!(!collides(Vec2::ZERO, &a, Vec2::new(15.0, 0.0), &b));
    }

    // --- AABB vs Cercle ---
    #[test]
    fn test_aabb_vs_circle_overlap() {
        let a = aabb_collider(10.0, 10.0);
        let b = circle_collider(2.0);
        assert!(collides(Vec2::ZERO, &a, Vec2::new(6.0, 0.0), &b));
    }

    #[test]
    fn test_aabb_vs_circle_no_overlap() {
        let a = aabb_collider(10.0, 10.0);
        let b = circle_collider(2.0);
        assert!(!collides(Vec2::ZERO, &a, Vec2::new(12.0, 0.0), &b));
    }

    // --- Cercle vs Cercle ---
    #[test]
    fn test_circle_vs_circle_overlap() {
        let a = circle_collider(5.0);
        let b = circle_collider(5.0);
        assert!(collides(Vec2::ZERO, &a, Vec2::new(4.0, 0.0), &b));
    }

    #[test]
    fn test_circle_vs_circle_no_overlap() {
        let a = circle_collider(5.0);
        let b = circle_collider(5.0);
        assert!(!collides(Vec2::ZERO, &a, Vec2::new(12.0, 0.0), &b));
    }

    // --- Inactif ---
    #[test]
    fn test_collider_inactive() {
        let mut a = aabb_collider(10.0, 10.0);
        a.set_active(false);
        let b = circle_collider(1.0);
        assert!(!collides(Vec2::ZERO, &a, Vec2::ZERO, &b));
    }

    // --- Contact : normale ---
    #[test]
    fn test_contact_normal_points_from_b_to_a() {
        // A à gauche, B à droite → normale doit pointer vers la gauche (-X)
        // puisque la normale va de B vers A.
        let a = aabb_collider(10.0, 10.0);
        let b = aabb_collider(10.0, 10.0);
        let c = collide_contact(Vec2::ZERO, &a, Vec2::new(3.0, 0.0), &b).unwrap();
        assert!(c.normal.x < 0.0, "normale doit pointer vers A (gauche)");
    }

    // --- Swept circle vs AABB ---
    #[test]
    fn test_swept_no_collision() {
        // Balle qui part vers le haut, AABB en bas.
        let c = swept_circle_vs_aabb(
            Vec2::new(0.0, -100.0),
            Vec2::new(0.0, -100.0),
            5.0,
            Vec2::new(10.0, 10.0),
            1.0,
        );
        assert!(c.is_none());
    }

    #[test]
    fn test_swept_head_on_from_above() {
        // Balle au-dessus, vélocité vers le bas.
        let c = swept_circle_vs_aabb(
            Vec2::new(0.0, -30.0),
            Vec2::new(0.0, 100.0),
            5.0,
            Vec2::new(10.0, 10.0),
            1.0,
        )
        .expect("should collide");
        assert!((c.normal.y + 1.0).abs() < 1e-3, "normale doit pointer vers le haut");
    }

    #[test]
    fn test_swept_high_speed_no_tunnel() {
        // Balle très rapide qui traverserait une AABB mince en physique discrète.
        let c = swept_circle_vs_aabb(
            Vec2::new(0.0, -500.0),
            Vec2::new(0.0, 2000.0),
            5.0,
            Vec2::new(50.0, 5.0),
            1.0,
        )
        .expect("should not tunnel");
        assert!(c.penetration >= 0.0);
    }

    // --- Swept circle vs OBB ---
    #[test]
    fn test_swept_obb_rotated_45() {
        let c = swept_circle_vs_obb(
            Vec2::new(0.0, -100.0),
            Vec2::new(0.0, 200.0),
            5.0,
            Vec2::ZERO,
            Vec2::new(50.0, 10.0),
            std::f32::consts::FRAC_PI_4,
            1.0,
        )
        .expect("should collide");
        // Normale doit pointer globalement vers le haut.
        assert!(c.normal.y < 0.0);
    }
}