//! The physics trait and its v0.1 backend, [`LinearPhysics`] (ADR-028).
//!
//! ## Handles, not NodeIds
//!
//! `register` returns an opaque `u64`, not a `NodeId`. `NodeId` lives in
//! `ember_scene` (L3); importing it here would violate the layer rule
//! (I1). The scene layer will map `NodeId ↔ u64` in v0.2 when there is
//! an actual implementation. This is formalized as ADR-017.

mod linear;

pub use linear::LinearPhysics;

use ember_core::math::{Aabb, Circle};
use glam::Vec2;

#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub enum Shape {
    Aabb(Aabb),
    Circle(Circle),
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub struct CollisionMask(pub u32);

impl CollisionMask {
    pub const NONE: Self = Self(0);
    pub const ALL: Self = Self(u32::MAX);

    pub const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    pub const fn bits(self) -> u32 {
        self.0
    }

    /// True when the two masks share at least one layer.
    pub const fn overlaps(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RayHit {
    pub point: Vec2,
    pub normal: Vec2,
    pub distance: f32,
}

pub trait PhysicsServer {
    fn register(&mut self, shape: Shape, mask: CollisionMask) -> u64;
    fn unregister(&mut self, handle: u64);
    /// Moves or reshapes a registered shape. Unknown handles are ignored.
    fn update(&mut self, handle: u64, shape: Shape);
    fn query_overlaps(&self, shape: &Shape, mask: CollisionMask) -> Vec<u64>;
    fn raycast(&self, from: Vec2, to: Vec2, mask: CollisionMask) -> Option<RayHit>;
    fn rebuild(&mut self);
}
