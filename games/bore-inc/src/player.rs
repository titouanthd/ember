//! Archie's runtime state: position, physics, mining, bag.
//!
//! The bag itself lives in `bag.rs`. `Archie` exposes convenience
//! passthroughs so call sites don't chain `archie.bag.collect(...)`.

use std::collections::HashMap;

use ember_stdlib::time::Cooldown;
use glam::Vec2;

use crate::bag::Bag;
use crate::components::{ResourceKind, TileKind, TILE_SIZE};
use crate::input::{Facing, GameInput};
use crate::world::World;

/// Width of Archie's bounding box, in pixels.
pub const ARCHIE_WIDTH: f32 = 12.0;
/// Height of Archie's bounding box, in pixels.
pub const ARCHIE_HEIGHT: f32 = 14.0;

// ---------- Physics constants (DESIGN §6) ----------

pub const GRAVITY: f32 = 900.0;
pub const JUMP_VELOCITY: f32 = -240.0;
pub const MAX_FALL_SPEED: f32 = 600.0;
pub const COYOTE_TIME: f32 = 0.10;
pub const JUMP_BUFFER_TIME: f32 = 0.15;

/// Reported by [`Archie::tick`] when a tile breaks this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BreakResult {
    pub col: usize,
    pub row: usize,
    pub kind: TileKind,
}

/// The player character's runtime state.
#[derive(Debug, Clone)]
pub struct Archie {
    pub pos: Vec2,
    pub size: Vec2,
    pub vy: f32,
    pub on_ground: bool,
    pub coyote: Cooldown,
    pub jump_buffer: Cooldown,
    pub facing: Facing,
    pub last_mine_dir: Option<Facing>,

    pub pickaxe_damage: f32,
    pub mining_speed: f32,
    pub move_speed: f32,

    pub bag: Bag,
}

impl Archie {
    pub fn new(pos: Vec2) -> Self {
        Self {
            pos,
            size: Vec2::new(ARCHIE_WIDTH, ARCHIE_HEIGHT),
            vy: 0.0,
            on_ground: false,
            coyote: Cooldown::default(),
            jump_buffer: Cooldown::default(),
            facing: Facing::Right,
            last_mine_dir: None,
            pickaxe_damage: 1.0,
            mining_speed: 1.0,
            move_speed: 100.0,
            bag: Bag::default(),
        }
    }

    pub fn center(&self) -> Vec2 {
        self.pos + self.size * 0.5
    }

    pub fn dps(&self) -> f32 {
        self.pickaxe_damage * self.mining_speed
    }

    // ---------- Bag passthroughs ----------

    pub fn bag_used(&self) -> u32 {
        self.bag.used()
    }

    pub fn bag_is_full(&self) -> bool {
        self.bag.is_full()
    }

    pub fn collect(&mut self, resource: ResourceKind, count: u32) -> u32 {
        self.bag.collect(resource, count)
    }

    pub fn empty_bag(&mut self) -> HashMap<ResourceKind, u32> {
        self.bag.empty()
    }

    // ---------- Simulation ----------

    pub fn tick(
        &mut self,
        dt: f32,
        input: GameInput,
        world: &mut World,
    ) -> Option<BreakResult> {
        self.coyote.tick(dt);
        self.jump_buffer.tick(dt);

        if let Some(f) = input.facing_held {
            self.facing = f;
        }

        if input.jump_pressed {
            self.jump_buffer.trigger(JUMP_BUFFER_TIME);
        }

        if self.jump_buffer.is_active() && (self.on_ground || self.coyote.is_active()) {
            self.vy = JUMP_VELOCITY;
            self.jump_buffer.clear();
            self.coyote.clear();
            self.on_ground = false;
        }

        let was_on_ground = self.on_ground;

        self.vy = (self.vy + GRAVITY * dt).min(MAX_FALL_SPEED);

        self.try_move_horizontal(dt, input, world);

        let break_result = if input.mine_held {
            let dir = self.facing;
            self.last_mine_dir = Some(dir);
            self.mine(dir, dt, world)
        } else {
            self.last_mine_dir = None;
            None
        };

        self.move_vertical(dt, world);

        if was_on_ground && !self.on_ground {
            self.coyote.trigger(COYOTE_TIME);
        } else if self.on_ground {
            self.coyote.clear();
        }

        break_result
    }

    fn try_move_horizontal(&mut self, dt: f32, input: GameInput, world: &World) {
        let dx = input.horizontal * self.move_speed * dt;
        if dx.abs() < f32::EPSILON {
            return;
        }

        let new_x = self.pos.x + dx;
        let sample_x = if dx > 0.0 {
            new_x + self.size.x - 0.01
        } else {
            new_x
        };
        let lead_col = World::pixel_to_col(sample_x);

        let row_top = World::pixel_to_row(self.pos.y + 1.0);
        let row_bot = World::pixel_to_row(self.pos.y + self.size.y - 1.0);

        let mut blocked = false;
        for row in row_top..=row_bot {
            if world.is_solid(lead_col, row) {
                blocked = true;
                break;
            }
        }

        if !blocked {
            self.pos.x = new_x;
            return;
        }

        if input.mine_held {
            return;
        }
        if self.can_step_up(lead_col, world) {
            self.pos.x = new_x;
            self.pos.y -= TILE_SIZE;
        }
    }

    fn can_step_up(&self, front_col: isize, world: &World) -> bool {
        let body_top_row = World::pixel_to_row(self.pos.y);
        let body_bot_row = World::pixel_to_row(self.pos.y + self.size.y - 0.01);
        if body_top_row != body_bot_row {
            return false;
        }
        !world.is_solid(front_col, body_top_row - 1)
    }

    pub fn mine(
        &mut self,
        dir: Facing,
        dt: f32,
        world: &mut World,
    ) -> Option<BreakResult> {
        let (col, row) = self.tile_in_direction(dir);
        self.mine_tile_at(col, row, dt, world)
    }

    fn tile_in_direction(&self, dir: Facing) -> (isize, isize) {
        let center_col = World::pixel_to_col(self.pos.x + self.size.x * 0.5);
        let center_row = World::pixel_to_row(self.pos.y + self.size.y * 0.5);
        let top_row = World::pixel_to_row(self.pos.y);
        let bottom_row = World::pixel_to_row(self.pos.y + self.size.y - 0.01);
        let left_col = World::pixel_to_col(self.pos.x);
        let right_col = World::pixel_to_col(self.pos.x + self.size.x - 0.01);

        match dir {
            Facing::Up => (center_col, top_row - 1),
            Facing::Down => (center_col, bottom_row + 1),
            Facing::Left => (left_col - 1, center_row),
            Facing::Right => (right_col + 1, center_row),
            Facing::UpLeft => (left_col - 1, top_row - 1),
            Facing::UpRight => (right_col + 1, top_row - 1),
            Facing::DownLeft => (left_col - 1, bottom_row + 1),
            Facing::DownRight => (right_col + 1, bottom_row + 1),
        }
    }

    fn mine_tile_at(
        &mut self,
        col: isize,
        row: isize,
        dt: f32,
        world: &mut World,
    ) -> Option<BreakResult> {
        if col < 0 || row < 0 {
            return None;
        }
        let (col, row) = (col as usize, row as usize);
        let dps = self.dps();

        let broken_kind = if let Some(tile) = world.tile_mut(col, row) {
            tile.damage(dps * dt);
            if tile.is_broken() { Some(tile.kind) } else { None }
        } else {
            None
        };

        let kind = broken_kind?;
        world.set(col, row, None);
        Some(BreakResult { col, row, kind })
    }

    fn move_vertical(&mut self, dt: f32, world: &World) {
        if self.vy > 0.0 {
            self.move_down(dt, world);
        } else if self.vy < 0.0 {
            self.move_up(dt, world);
        }
    }

    fn move_down(&mut self, dt: f32, world: &World) {
        let new_y = self.pos.y + self.vy * dt;
        let feet_y = new_y + self.size.y;

        let col_left = World::pixel_to_col(self.pos.x + 0.01);
        let col_right = World::pixel_to_col(self.pos.x + self.size.x - 0.01);

        // Collision: do the feet enter a new solid row this frame?
        // The `- 0.01` puts the sample *inside* the row the feet
        // land in, not on the boundary with the row above.
        let coll_row = World::pixel_to_row(feet_y - 0.01);
        let collided = (col_left..=col_right)
            .any(|col| world.is_solid(col, coll_row));

        // Ground snap: even if no new row was entered, if the row
        // directly under the feet is solid, we're supported. This
        // is what fixes the idle-frame `on_ground` flicker: sub-pixel
        // vy never crosses the tile boundary, but the feet are
        // touching the tile top.
        let snap_row = World::pixel_to_row(feet_y);
        let snapped = (col_left..=col_right)
            .any(|col| world.is_solid(col, snap_row));

        let landed_row = if collided {
            Some(coll_row)
        } else if snapped {
            Some(snap_row)
        } else {
            None
        };

        if let Some(row) = landed_row {
            let tile_top = row as f32 * TILE_SIZE;
            self.pos.y = tile_top - self.size.y;
            self.vy = 0.0;
            self.on_ground = true;
        } else {
            self.pos.y = new_y;
            self.on_ground = false;
        }
    }

    fn move_up(&mut self, dt: f32, world: &World) {
        let new_y = self.pos.y + self.vy * dt;
        let row = World::pixel_to_row(new_y);

        let col_left = World::pixel_to_col(self.pos.x + 0.01);
        let col_right = World::pixel_to_col(self.pos.x + self.size.x - 0.01);

        let mut hit = false;
        for col in col_left..=col_right {
            if world.is_solid(col, row) {
                hit = true;
                break;
            }
        }

        if hit {
            let tile_bottom = (row + 1) as f32 * TILE_SIZE;
            self.pos.y = tile_bottom;
            self.vy = 0.0;
        } else {
            self.pos.y = new_y;
        }
        self.on_ground = false;
    }
}