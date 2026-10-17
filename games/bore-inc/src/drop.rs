//! Resource drops. A broken tile leaves behind a `Drop` that falls
//! under gravity, lands on solid ground, and waits to be picked up by
//! Archie.
//!
//! Drops are simple: no rolling, no sliding, no stacking. They fall,
//! land, and sit.

use glam::Vec2;

use crate::components::{ResourceKind, TILE_SIZE};
use crate::world::World;

/// Downward acceleration for drops, px/s². Same as Archie's gravity
/// so their fall reads as part of the same world.
pub const DROP_GRAVITY: f32 = 900.0;
/// Terminal fall speed for drops, px/s.
pub const DROP_MAX_FALL: f32 = 400.0;
/// Half-size of a drop's bounding box, in pixels. Small — they should
/// read as pellets, not tiles.
pub const DROP_RADIUS: f32 = 4.0;

/// One drop on the ground.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Drop {
    /// Center position, in world pixels.
    pub pos: Vec2,
    /// Vertical velocity, px/s. Positive = falling.
    pub vy: f32,
    /// What resource this drop yields.
    pub resource: ResourceKind,
    /// How many units it still holds. A drop with `count == 0` should
    /// be removed by the caller.
    pub count: u32,
    /// True if the drop has landed on solid ground this tick.
    pub on_ground: bool,
}

impl Drop {
    /// A drop at `center` with zero velocity.
    pub fn new(center: Vec2, resource: ResourceKind, count: u32) -> Self {
        Self {
            pos: center,
            vy: 0.0,
            resource,
            count,
            on_ground: false,
        }
    }

    /// Fall under gravity, land on solid tiles.
    pub fn tick(&mut self, dt: f32, world: &World) {
        if self.on_ground {
            return;
        }
        self.vy = (self.vy + DROP_GRAVITY * dt).min(DROP_MAX_FALL);

        let new_y = self.pos.y + self.vy * dt;
        // The drop's bottom edge, sampled 0.01 px above so a flush
        // landing isn't read as a row-below.
        let sample_y = new_y + DROP_RADIUS - 0.01;
        let row = World::pixel_to_row(sample_y);
        let col = World::pixel_to_col(self.pos.x);

        if world.is_solid(col, row) {
            let tile_top = row as f32 * TILE_SIZE;
            self.pos.y = tile_top - DROP_RADIUS;
            self.vy = 0.0;
            self.on_ground = true;
        } else {
            self.pos.y = new_y;
        }
    }

    /// AABB-vs-square overlap against an Archie-sized box at `p` with
    /// size `s`. Used by `Game` to detect pickup.
    pub fn overlaps_box(&self, p: Vec2, s: Vec2) -> bool {
        let r = DROP_RADIUS;
        self.pos.x + r >= p.x
            && self.pos.x - r <= p.x + s.x
            && self.pos.y + r >= p.y
            && self.pos.y - r <= p.y + s.y
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{Tile, TileKind};

    fn empty_world(w: usize, h: usize) -> World {
        World::from_cells(w, h, vec![None; w * h], 0, 0)
    }

    #[test]
    fn new_drop_has_zero_velocity() {
        let d = Drop::new(Vec2::new(50.0, 50.0), ResourceKind::Stone, 1);
        assert_eq!(d.vy, 0.0);
        assert!(!d.on_ground);
    }

    #[test]
    fn drop_falls_under_gravity() {
        let world = empty_world(8, 20);
        let mut d = Drop::new(Vec2::new(32.0, 32.0), ResourceKind::Stone, 1);
        let dt = 1.0 / 60.0;
        for _ in 0..30 {
            d.tick(dt, &world);
        }
        assert!(d.pos.y > 32.0);
        assert!(d.vy > 0.0);
    }

    #[test]
    fn drop_lands_on_solid_ground() {
        let mut world = empty_world(8, 20);
        for col in 0..8 {
            world.set(col, 10, Some(Tile::new(TileKind::Stone)));
        }
        let mut d = Drop::new(Vec2::new(32.0, 32.0), ResourceKind::Coal, 1);
        let dt = 1.0 / 60.0;
        for _ in 0..300 {
            d.tick(dt, &world);
        }
        assert!(d.on_ground);
        // Row 10 top = 160. Drop bottom at 160, center at 156.
        assert!((d.pos.y - 156.0).abs() < 0.5, "y = {}", d.pos.y);
    }

    #[test]
    fn drop_does_not_fall_through_bedrock() {
        let mut world = empty_world(8, 20);
        for col in 0..8 {
            world.set(col, 15, Some(Tile::new(TileKind::Bedrock)));
        }
        let mut d = Drop::new(Vec2::new(32.0, 32.0), ResourceKind::Iron, 1);
        let dt = 1.0 / 60.0;
        for _ in 0..600 {
            d.tick(dt, &world);
        }
        // Bottom at 240 (row 15 top).
        assert!(d.pos.y + DROP_RADIUS <= 240.5);
    }

    #[test]
    fn drop_does_not_move_once_on_ground() {
        let mut world = empty_world(8, 20);
        for col in 0..8 {
            world.set(col, 10, Some(Tile::new(TileKind::Stone)));
        }
        let mut d = Drop::new(Vec2::new(32.0, 32.0), ResourceKind::Stone, 1);
        let dt = 1.0 / 60.0;
        for _ in 0..300 {
            d.tick(dt, &world);
        }
        let y = d.pos.y;
        for _ in 0..60 {
            d.tick(dt, &world);
        }
        assert_eq!(d.pos.y, y);
    }

    #[test]
    fn overlaps_box_detects_containment() {
        let d = Drop::new(Vec2::new(50.0, 50.0), ResourceKind::Stone, 1);
        assert!(d.overlaps_box(Vec2::new(40.0, 40.0), Vec2::new(20.0, 20.0)));
    }

    #[test]
    fn overlaps_box_rejects_far_apart() {
        let d = Drop::new(Vec2::new(50.0, 50.0), ResourceKind::Stone, 1);
        assert!(!d.overlaps_box(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0)));
    }

    #[test]
    fn overlaps_box_detects_touching_edges() {
        let d = Drop::new(Vec2::new(50.0, 50.0), ResourceKind::Stone, 1);
        // Archie's box right edge at 50 - 4 = 46, exactly touching.
        assert!(d.overlaps_box(Vec2::new(36.0, 36.0), Vec2::new(10.0, 10.0)));
    }
}