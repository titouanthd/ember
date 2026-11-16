//! The physics trait. v0.1 ships the shape only — see
//! `12-servers.md §6.1`. `ember-stdlib::collider` remains the actual
//! implementation until v0.2.
//!
//! ## Handles, not NodeIds
//!
//! `register` returns an opaque `u64`, not a `NodeId`. `NodeId` lives in
//! `ember_scene` (L3); importing it here would violate the layer rule
//! (I1). The scene layer will map `NodeId ↔ u64` in v0.2 when there is
//! an actual implementation. This is formalized as ADR-017.

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
    fn query_overlaps(&self, shape: &Shape, mask: CollisionMask) -> Vec<u64>;
    fn raycast(&self, from: Vec2, to: Vec2, mask: CollisionMask) -> Option<RayHit>;
    fn rebuild(&mut self);
}
