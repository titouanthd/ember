//! The Bore Inc. truck.

use std::collections::HashMap;

use glam::Vec2;

use crate::components::{ResourceKind, TILE_SIZE};
use crate::world::World;

pub const TRUCK_WIDTH: f32 = TILE_SIZE * 2.0;
pub const TRUCK_HEIGHT: f32 = TILE_SIZE * 2.0;

pub const TRUCK_SPEED: f32 = 70.0;
pub const MAX_STEP_UP: f32 = TILE_SIZE;
pub const MAX_STEP_DOWN: f32 = TILE_SIZE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockReason {
    Wall,
    LowCeiling,
    HoleAhead,
    SteepStep,
}

#[derive(Debug, Clone)]
pub struct Truck {
    pub pos: Vec2,
    pub size: Vec2,

    pub facing_right: bool,

    /// Total distance travelled since spawn. Drives the wheel-spin
    /// animation frame.
    pub distance_traveled: f32,

    pub storage: HashMap<ResourceKind, u32>,
    pub storage_capacity: u32,

    pub move_speed: f32,
    pub max_step_up: f32,

    pub blocked: Option<BlockReason>,
}

impl Truck {
    pub fn new(pos: Vec2) -> Self {
        Self {
            pos,
            size: Vec2::new(TRUCK_WIDTH, TRUCK_HEIGHT),
            facing_right: true,
            distance_traveled: 0.0,
            storage: HashMap::new(),
            storage_capacity: 100,
            move_speed: TRUCK_SPEED,
            max_step_up: MAX_STEP_UP,
            blocked: None,
        }
    }

    pub fn center(&self) -> Vec2 {
        self.pos + self.size * 0.5
    }

    pub fn near(&self, point: Vec2, radius: f32) -> bool {
        (self.center() - point).length() <= radius
    }

    fn body_cols(&self, x: f32) -> (isize, isize) {
        (
            World::pixel_to_col(x),
            World::pixel_to_col(x + self.size.x - 0.01),
        )
    }

    fn body_rows(&self, y: f32) -> (isize, isize) {
        (
            World::pixel_to_row(y),
            World::pixel_to_row(y + self.size.y - 0.01),
        )
    }

    fn body_is_clear(&self, x: f32, y: f32, world: &World) -> bool {
        let (c0, c1) = self.body_cols(x);
        let (r0, r1) = self.body_rows(y);
        for row in r0..=r1 {
            for col in c0..=c1 {
                if world.is_solid(col, row) {
                    return false;
                }
            }
        }
        true
    }

    fn has_ground_at(&self, x: f32, y: f32, world: &World) -> bool {
        let sample_y = y + self.size.y + 1.0;
        let row = World::pixel_to_row(sample_y);
        let (c0, c1) = self.body_cols(x);
        let lead_col = if self.facing_right { c1 } else { c0 };
        world.is_solid(lead_col, row)
    }

    pub fn tick(&mut self, dt: f32, drive: f32, world: &World) {
        self.blocked = None;
        if drive.abs() < f32::EPSILON {
            return;
        }
        self.facing_right = drive > 0.0;

        let dx = drive * self.move_speed * dt;
        let new_x = self.pos.x + dx;

        match self.try_step_to(new_x, world) {
            Ok(new_y) => {
                self.pos.x = new_x;
                self.pos.y = new_y;
                self.distance_traveled += dx.abs();
            }
            Err(reason) => {
                self.blocked = Some(reason);
            }
        }
    }

    fn try_step_to(&self, x: f32, world: &World) -> Result<f32, BlockReason> {
        let candidates = [
            self.pos.y,
            self.pos.y - self.max_step_up,
            self.pos.y + MAX_STEP_DOWN,
        ];

        for &y in &candidates {
            if !self.has_ground_at(x, y, world) {
                continue;
            }
            if !self.body_is_clear(x, y, world) {
                continue;
            }
            return Ok(y);
        }

        let y_down = self.pos.y + MAX_STEP_DOWN;
        if self.has_ground_at(x, y_down, world)
            && self.body_is_clear(x, self.pos.y, world)
        {
            return Ok(self.pos.y);
        }

        if !self.has_ground_at(x, self.pos.y, world) {
            return Err(BlockReason::HoleAhead);
        }
        if !self.body_is_clear(x, self.pos.y, world) {
            return Err(self.diagnose_body_block(x, self.pos.y, world));
        }
        Err(BlockReason::SteepStep)
    }

    fn diagnose_body_block(&self, x: f32, y: f32, world: &World) -> BlockReason {
        let (c0, c1) = self.body_cols(x);
        let bottom_row = World::pixel_to_row(y + self.size.y - 0.01);
        let top_row = World::pixel_to_row(y);

        let bottom_blocked = (c0..=c1).any(|col| world.is_solid(col, bottom_row));
        if bottom_blocked {
            return BlockReason::Wall;
        }
        let top_blocked = (c0..=c1).any(|col| world.is_solid(col, top_row));
        if top_blocked {
            return BlockReason::LowCeiling;
        }
        BlockReason::Wall
    }

    pub fn storage_used(&self) -> u32 {
        self.storage.values().copied().sum()
    }

    pub fn storage_free(&self) -> u32 {
        self.storage_capacity.saturating_sub(self.storage_used())
    }

    pub fn deposit(&mut self, resource: ResourceKind, n: u32) -> u32 {
        let accepted = n.min(self.storage_free());
        if accepted > 0 {
            *self.storage.entry(resource).or_insert(0) += accepted;
        }
        accepted
    }

    pub fn empty_storage(&mut self) -> HashMap<ResourceKind, u32> {
        std::mem::take(&mut self.storage)
    }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{Tile, TileKind};

    fn build_world<F>(width: usize, height: usize, f: F) -> World
    where
        F: Fn(usize, usize) -> bool,
    {
        let mut cells = Vec::with_capacity(width * height);
        for row in 0..height {
            for col in 0..width {
                cells.push(if f(col, row) {
                    Some(Tile::new(TileKind::Stone))
                } else {
                    None
                });
            }
        }
        World::from_cells(width, height, cells, 0, 0)
    }

    fn flat_world(ground_row: usize) -> World {
        build_world(30, 20, |_, row| row >= ground_row)
    }

    #[test]
    fn new_truck_has_expected_defaults() {
        let t = Truck::new(Vec2::new(0.0, 0.0));
        assert_eq!(t.size, Vec2::new(32.0, 32.0));
        assert!(t.storage.is_empty());
        assert_eq!(t.storage_capacity, 100);
        assert_eq!(t.move_speed, TRUCK_SPEED);
        assert!(t.blocked.is_none());
        assert_eq!(t.distance_traveled, 0.0);
    }

    #[test]
    fn distance_accumulates_when_moving() {
        let world = flat_world(15);
        let mut t = Truck::new(Vec2::new(32.0, 208.0));
        let dt = 1.0 / 60.0;
        for _ in 0..30 {
            t.tick(dt, 1.0, &world);
        }
        // ~35 px moved.
        assert!((t.distance_traveled - 35.0).abs() < 1.5, "d = {}", t.distance_traveled);
    }

    #[test]
    fn distance_does_not_accumulate_when_blocked() {
        let world = build_world(30, 20, |col, row| {
            row >= 15 || (col == 10 && (13..=14).contains(&row))
        });
        let mut t = Truck::new(Vec2::new(32.0, 208.0));
        let dt = 1.0 / 60.0;
        for _ in 0..200 {
            t.tick(dt, 1.0, &world);
        }
        // Hits the wall at col 10, distance stops there.
        // Start x=32, wall at 160, truck width 32 → max x = 128.
        // Distance = 128 - 32 = 96.
        assert!(
            (t.distance_traveled - 96.0).abs() < 2.0,
            "d = {}",
            t.distance_traveled,
        );
    }

    #[test]
    fn truck_does_not_move_without_drive_input() {
        let world = flat_world(15);
        let mut t = Truck::new(Vec2::new(64.0, 208.0));
        let dt = 1.0 / 60.0;
        for _ in 0..60 {
            t.tick(dt, 0.0, &world);
        }
        assert_eq!(t.pos.x, 64.0);
        assert!(t.blocked.is_none());
    }

    #[test]
    fn truck_moves_right_on_flat_ground() {
        let world = flat_world(15);
        let mut t = Truck::new(Vec2::new(32.0, 208.0));
        let dt = 1.0 / 60.0;
        for _ in 0..30 {
            t.tick(dt, 1.0, &world);
        }
        assert!((t.pos.x - 67.0).abs() < 1.0, "x = {}", t.pos.x);
        assert!(t.blocked.is_none());
        assert!(t.facing_right);
    }

    #[test]
    fn truck_moves_left_on_flat_ground() {
        let world = flat_world(15);
        let mut t = Truck::new(Vec2::new(80.0, 208.0));
        let dt = 1.0 / 60.0;
        for _ in 0..30 {
            t.tick(dt, -1.0, &world);
        }
        assert!((t.pos.x - 45.0).abs() < 1.0, "x = {}", t.pos.x);
        assert!(t.blocked.is_none());
        assert!(!t.facing_right);
    }

    #[test]
    fn truck_climbs_a_one_tile_step() {
        let world = build_world(30, 20, |col, row| {
            if col >= 10 { row >= 14 } else { row >= 15 }
        });
        let mut t = Truck::new(Vec2::new(32.0, 208.0));
        let dt = 1.0 / 60.0;
        for _ in 0..200 {
            t.tick(dt, 1.0, &world);
        }
        assert!((t.pos.y - 192.0).abs() < 1.0, "y = {}", t.pos.y);
        assert!(t.pos.x > 100.0);
        assert!(t.blocked.is_none());
    }

    #[test]
    fn truck_descends_a_one_tile_step() {
        let world = build_world(30, 20, |col, row| {
            if col < 10 { row >= 14 } else { row >= 15 }
        });
        let mut t = Truck::new(Vec2::new(32.0, 192.0));
        let dt = 1.0 / 60.0;
        for _ in 0..200 {
            t.tick(dt, 1.0, &world);
        }
        assert!((t.pos.y - 208.0).abs() < 1.0, "y = {}", t.pos.y);
        assert!(t.pos.x > 100.0);
    }

    #[test]
    fn truck_is_blocked_by_a_wall() {
        let world = build_world(30, 20, |col, row| {
            row >= 15 || (col == 10 && (13..=14).contains(&row))
        });
        let mut t = Truck::new(Vec2::new(32.0, 208.0));
        let dt = 1.0 / 60.0;
        for _ in 0..200 {
            t.tick(dt, 1.0, &world);
        }
        assert!(t.pos.x + t.size.x <= 160.01);
        assert_eq!(t.blocked, Some(BlockReason::Wall));
    }

    #[test]
    fn truck_is_blocked_by_a_low_ceiling() {
        let world = build_world(30, 20, |col, row| {
            row >= 15 || (col >= 10 && row == 13)
        });
        let mut t = Truck::new(Vec2::new(32.0, 208.0));
        let dt = 1.0 / 60.0;
        for _ in 0..200 {
            t.tick(dt, 1.0, &world);
        }
        assert!(t.pos.x + t.size.x <= 160.01);
        assert_eq!(t.blocked, Some(BlockReason::LowCeiling));
    }

    #[test]
    fn truck_is_blocked_by_a_hole() {
        let world = build_world(30, 20, |col, row| col < 10 && row >= 15);
        let mut t = Truck::new(Vec2::new(32.0, 208.0));
        let dt = 1.0 / 60.0;
        for _ in 0..200 {
            t.tick(dt, 1.0, &world);
        }
        assert!(t.pos.x + t.size.x <= 160.5);
        assert_eq!(t.blocked, Some(BlockReason::HoleAhead));
    }

    #[test]
    fn truck_refuses_a_two_tile_step_up() {
        let world = build_world(30, 20, |col, row| {
            if col >= 10 { row >= 13 } else { row >= 15 }
        });
        let mut t = Truck::new(Vec2::new(32.0, 208.0));
        let dt = 1.0 / 60.0;
        for _ in 0..200 {
            t.tick(dt, 1.0, &world);
        }
        assert!(t.pos.x + t.size.x <= 160.5);
        assert!(t.blocked.is_some());
    }

    #[test]
    fn deposit_caps_at_capacity() {
        let mut t = Truck::new(Vec2::new(0.0, 0.0));
        t.storage_capacity = 10;
        assert_eq!(t.deposit(ResourceKind::Dirt, 7), 7);
        assert_eq!(t.deposit(ResourceKind::Iron, 5), 3);
        assert_eq!(t.storage_used(), 10);
        assert_eq!(t.storage_free(), 0);
    }

    #[test]
    fn storage_used_sums_all_resources() {
        let mut t = Truck::new(Vec2::new(0.0, 0.0));
        t.deposit(ResourceKind::Stone, 3);
        t.deposit(ResourceKind::Coal, 2);
        t.deposit(ResourceKind::Crystal, 1);
        assert_eq!(t.storage_used(), 6);
    }

    #[test]
    fn empty_storage_returns_contents() {
        let mut t = Truck::new(Vec2::new(0.0, 0.0));
        t.deposit(ResourceKind::Stone, 4);
        let out = t.empty_storage();
        assert_eq!(out.get(&ResourceKind::Stone), Some(&4));
        assert_eq!(t.storage_used(), 0);
    }

    #[test]
    fn near_detects_proximity() {
        let t = Truck::new(Vec2::new(100.0, 100.0));
        assert!(t.near(Vec2::new(116.0, 116.0), 5.0));
        assert!(!t.near(Vec2::new(200.0, 200.0), 5.0));
    }
}