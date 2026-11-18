//! `LinearPhysics`: O(n) queries over a flat list of shapes (ADR-028).
//! Fine below ~300 shapes; spatial indexing is v0.2.

use super::{CollisionMask, PhysicsServer, RayHit, Shape};
use ember_core::math::{Aabb, Circle};
use glam::Vec2;

#[derive(Default)]
pub struct LinearPhysics {
    next: u64,
    shapes: Vec<(u64, Shape, CollisionMask)>,
}

impl LinearPhysics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.shapes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.shapes.is_empty()
    }
}

fn aabb_circle(a: &Aabb, c: &Circle) -> bool {
    let nearest = c.center.clamp(a.min, a.max);
    nearest.distance_squared(c.center) <= c.radius * c.radius
}

fn overlaps(a: &Shape, b: &Shape) -> bool {
    match (a, b) {
        (Shape::Aabb(a), Shape::Aabb(b)) => {
            a.min.x <= b.max.x && a.max.x >= b.min.x && a.min.y <= b.max.y && a.max.y >= b.min.y
        }
        (Shape::Circle(a), Shape::Circle(b)) => {
            let r = a.radius + b.radius;
            a.center.distance_squared(b.center) <= r * r
        }
        (Shape::Aabb(a), Shape::Circle(c)) | (Shape::Circle(c), Shape::Aabb(a)) => {
            aabb_circle(a, c)
        }
    }
}

/// Segment `from + t * d`, `t` in `[0, 1]`, against a box (slab method).
fn ray_aabb(from: Vec2, d: Vec2, b: &Aabb) -> Option<(f32, Vec2)> {
    if from.cmpge(b.min).all() && from.cmple(b.max).all() {
        return Some((0.0, -d.normalize_or_zero()));
    }
    let (mut t_min, mut t_max) = (0.0_f32, 1.0_f32);
    let mut normal = Vec2::ZERO;
    for axis in 0..2 {
        let (o, dir, lo, hi) = (from[axis], d[axis], b.min[axis], b.max[axis]);
        if dir.abs() < f32::EPSILON {
            if o < lo || o > hi {
                return None;
            }
            continue;
        }
        let (mut t0, mut t1) = ((lo - o) / dir, (hi - o) / dir);
        let mut sign = -1.0;
        if t0 > t1 {
            std::mem::swap(&mut t0, &mut t1);
            sign = 1.0;
        }
        if t0 > t_min {
            t_min = t0;
            normal = Vec2::ZERO;
            normal[axis] = sign;
        }
        t_max = t_max.min(t1);
        if t_min > t_max {
            return None;
        }
    }
    Some((t_min, normal))
}

fn ray_circle(from: Vec2, d: Vec2, c: &Circle) -> Option<(f32, Vec2)> {
    let m = from - c.center;
    let (a, b, k) = (d.dot(d), m.dot(d), m.dot(m) - c.radius * c.radius);
    if k <= 0.0 {
        return Some((0.0, -d.normalize_or_zero()));
    }
    if a < f32::EPSILON {
        return None;
    }
    let disc = b * b - a * k;
    if disc < 0.0 {
        return None;
    }
    let t = (-b - disc.sqrt()) / a;
    if !(0.0..=1.0).contains(&t) {
        return None;
    }
    Some((t, (from + d * t - c.center).normalize_or_zero()))
}

impl PhysicsServer for LinearPhysics {
    fn register(&mut self, shape: Shape, mask: CollisionMask) -> u64 {
        self.next += 1;
        self.shapes.push((self.next, shape, mask));
        self.next
    }

    fn unregister(&mut self, handle: u64) {
        self.shapes.retain(|(h, ..)| *h != handle);
    }

    fn update(&mut self, handle: u64, shape: Shape) {
        if let Some(entry) = self.shapes.iter_mut().find(|(h, ..)| *h == handle) {
            entry.1 = shape;
        }
    }

    fn query_overlaps(&self, shape: &Shape, mask: CollisionMask) -> Vec<u64> {
        self.shapes
            .iter()
            .filter(|(_, s, m)| m.overlaps(mask) && overlaps(s, shape))
            .map(|(h, ..)| *h)
            .collect()
    }

    fn raycast(&self, from: Vec2, to: Vec2, mask: CollisionMask) -> Option<RayHit> {
        let d = to - from;
        let length = d.length();
        self.shapes
            .iter()
            .filter(|(_, _, m)| m.overlaps(mask))
            .filter_map(|(_, s, _)| match s {
                Shape::Aabb(b) => ray_aabb(from, d, b),
                Shape::Circle(c) => ray_circle(from, d, c),
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(t, normal)| RayHit {
                point: from + d * t,
                normal,
                distance: t * length,
            })
    }

    /// Nothing to rebuild: there is no index.
    fn rebuild(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bx(x: f32, y: f32, hx: f32, hy: f32) -> Shape {
        Shape::Aabb(Aabb::from_center_half(Vec2::new(x, y), Vec2::new(hx, hy)))
    }

    fn circle(x: f32, y: f32, r: f32) -> Shape {
        Shape::Circle(Circle::new(Vec2::new(x, y), r))
    }

    #[test]
    fn overlap_pairs_and_masks() {
        let mut p = LinearPhysics::new();
        let wall = p.register(bx(10.0, 0.0, 1.0, 5.0), CollisionMask::from_bits(0b01));
        let ball = p.register(circle(0.0, 0.0, 2.0), CollisionMask::from_bits(0b10));
        assert_eq!(
            p.query_overlaps(&circle(9.0, 0.0, 0.5), CollisionMask::ALL),
            [wall]
        );
        assert_eq!(
            p.query_overlaps(&bx(1.0, 1.0, 0.5, 0.5), CollisionMask::ALL),
            [ball]
        );
        assert!(
            p.query_overlaps(&bx(1.0, 1.0, 0.5, 0.5), CollisionMask::from_bits(0b01))
                .is_empty()
        );
        assert!(
            p.query_overlaps(&circle(100.0, 0.0, 1.0), CollisionMask::ALL)
                .is_empty()
        );
    }

    #[test]
    fn unregister_and_update() {
        let mut p = LinearPhysics::new();
        let h = p.register(circle(0.0, 0.0, 1.0), CollisionMask::ALL);
        let probe = circle(10.0, 0.0, 1.0);
        assert!(p.query_overlaps(&probe, CollisionMask::ALL).is_empty());
        p.update(h, circle(10.0, 0.0, 1.0));
        assert_eq!(p.query_overlaps(&probe, CollisionMask::ALL), [h]);
        p.unregister(h);
        assert!(p.is_empty());
        p.update(h, circle(0.0, 0.0, 1.0));
        assert!(p.is_empty());
    }

    #[test]
    fn raycast_hits_nearest_with_normal() {
        let mut p = LinearPhysics::new();
        let far = p.register(bx(20.0, 0.0, 1.0, 1.0), CollisionMask::ALL);
        let near = p.register(circle(10.0, 0.0, 2.0), CollisionMask::ALL);
        let _ = (far, near);
        let hit = p
            .raycast(Vec2::ZERO, Vec2::new(30.0, 0.0), CollisionMask::ALL)
            .unwrap();
        assert!((hit.distance - 8.0).abs() < 1e-4);
        assert!((hit.normal - Vec2::NEG_X).length() < 1e-4);

        let hit = p
            .raycast(
                Vec2::new(15.0, 0.0),
                Vec2::new(30.0, 0.0),
                CollisionMask::ALL,
            )
            .unwrap();
        assert!((hit.point.x - 19.0).abs() < 1e-4);
        assert_eq!(hit.normal, Vec2::NEG_X);
    }

    #[test]
    fn raycast_misses_and_respects_mask_and_range() {
        let mut p = LinearPhysics::new();
        p.register(bx(10.0, 0.0, 1.0, 1.0), CollisionMask::from_bits(0b1));
        assert!(
            p.raycast(Vec2::ZERO, Vec2::new(5.0, 0.0), CollisionMask::ALL)
                .is_none()
        );
        assert!(
            p.raycast(Vec2::ZERO, Vec2::new(0.0, 10.0), CollisionMask::ALL)
                .is_none()
        );
        assert!(
            p.raycast(
                Vec2::ZERO,
                Vec2::new(20.0, 0.0),
                CollisionMask::from_bits(0b10)
            )
            .is_none()
        );
    }
}
