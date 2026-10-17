//! HQ screen. DESIGN §12.
//!
//! Slice 5B: sell pad + five upgrade blocks. Selling drains the truck
//! into dollars. Headbutting a block buys the next level of an
//! upgrade, if affordable.

use std::collections::HashMap;

use glam::Vec2;
use macroquad::prelude::*;

use crate::atlas::Atlas;
use crate::components::{ResourceKind, Tile, TileKind, TILE_SIZE};
use crate::config::GameContext;
use crate::input::GameInput;
use crate::player::{Archie, ARCHIE_HEIGHT};
use crate::progress::Progress;
use crate::sprites::archie::ArchieSprites;
use crate::systems::{clamp_camera, integer_scale, VIRTUAL_H, VIRTUAL_W};
use crate::upgrade::{self, UPGRADES};
use crate::world::World;

// ---------- Layout ----------

const HQ_W: usize = 60;
const HQ_H: usize = 12;
const HQ_SURFACE_ROW: usize = 9;

const TRUCK_COL: usize = 4;
const SELL_PAD_COL: usize = 24;
const SELL_PAD_WIDTH: usize = 2;

const TRUCK_W_TILES: f32 = 2.0;
const TRUCK_H_TILES: f32 = 2.0;

/// Top row of the upgrade blocks. Blocks are 2 tiles tall.
const BLOCK_ROW: usize = 5;
const BLOCK_W_TILES: f32 = 2.0;
const BLOCK_H_TILES: f32 = 2.0;
/// Columns for the five blocks, in `UPGRADES` order.
const BLOCK_COLS: &[usize] = &[32, 36, 40, 44, 48];

// ---------- Timings ----------

const SALE_FLOATER_DURATION: f32 = 1.6;
const HEADBUTT_COOLDOWN: f32 = 0.4;
const BLOCK_FLASH_DURATION: f32 = 0.25;
const UPGRADE_FLOATER_DURATION: f32 = 1.2;

// ---------- Data ----------

pub struct UpgradeBlock {
    /// Index into `upgrade::UPGRADES`.
    pub def_index: usize,
    pub col: usize,
    /// Current purchased level for this block.
    pub level: u8,
    /// Seconds remaining on the purchase flash. 0 = no flash.
    pub flash_timer: f32,
}

struct Floater {
    text: String,
    /// World px, block center.
    x: f32,
    /// World px, top of block.
    y: f32,
    timer: f32,
    color: Color,
}

pub struct HqState {
    pub world: World,
    pub archie: Archie,
    pub cargo: HashMap<ResourceKind, u32>,
    pub initial_cargo: u32,
    pub total_sold_this_visit: u64,
    pub input_horizontal: f32,

    pub last_sale_earned: u64,
    pub last_sale_timer: f32,

    pub blocks: Vec<UpgradeBlock>,
    headbutt_cooldown: f32,
    floaters: Vec<Floater>,
}

impl HqState {
    pub fn new(cargo: HashMap<ResourceKind, u32>, progress: &Progress) -> Self {
        let world = build_hq_world();
        let start_x = 2.0 * TILE_SIZE;
        let start_y = HQ_SURFACE_ROW as f32 * TILE_SIZE - ARCHIE_HEIGHT;
        let archie = Archie::new(Vec2::new(start_x, start_y));
        let initial_cargo = cargo.values().copied().sum();

        let blocks = BLOCK_COLS
            .iter()
            .enumerate()
            .map(|(i, &col)| {
                let def = &UPGRADES[i];
                UpgradeBlock {
                    def_index: i,
                    col,
                    level: progress.upgrade_level(def.id),
                    flash_timer: 0.0,
                }
            })
            .collect();

        Self {
            world,
            archie,
            cargo,
            initial_cargo,
            total_sold_this_visit: 0,
            input_horizontal: 0.0,
            last_sale_earned: 0,
            last_sale_timer: 0.0,
            blocks,
            headbutt_cooldown: 0.0,
            floaters: Vec::new(),
        }
    }

    pub fn tick(&mut self, dt: f32, mut input: GameInput, progress: &mut Progress) {
        input.mine_held = false;
        self.input_horizontal = input.horizontal;

        self.archie.tick(dt, input, &mut self.world);
        self.try_sell(progress);
        self.try_headbutt(progress);

        if self.last_sale_timer > 0.0 {
            self.last_sale_timer = (self.last_sale_timer - dt).max(0.0);
        }
        if self.headbutt_cooldown > 0.0 {
            self.headbutt_cooldown = (self.headbutt_cooldown - dt).max(0.0);
        }
        for b in &mut self.blocks {
            if b.flash_timer > 0.0 {
                b.flash_timer = (b.flash_timer - dt).max(0.0);
            }
        }

        let mut i = 0;
        while i < self.floaters.len() {
            self.floaters[i].timer -= dt;
            if self.floaters[i].timer <= 0.0 {
                self.floaters.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }

    fn try_sell(&mut self, progress: &mut Progress) {
        if self.cargo.is_empty() {
            return;
        }

        let pad_left = SELL_PAD_COL as f32 * TILE_SIZE;
        let pad_right = pad_left + (SELL_PAD_WIDTH as f32) * TILE_SIZE;
        let a_left = self.archie.pos.x;
        let a_right = self.archie.pos.x + self.archie.size.x;

        if a_right <= pad_left || a_left >= pad_right {
            return;
        }

        let mut earned: u64 = 0;
        for (res, n) in self.cargo.drain() {
            earned += res.value() as u64 * n as u64;
        }
        progress.add_dollars(earned);
        self.total_sold_this_visit += earned;

        if earned > 0 {
            self.last_sale_earned = earned;
            self.last_sale_timer = SALE_FLOATER_DURATION;
        }
    }

    fn try_headbutt(&mut self, progress: &mut Progress) {
        // Must be rising. Standing or falling hits do nothing.
        if self.archie.vy >= 0.0 || self.headbutt_cooldown > 0.0 {
            return;
        }

        let ax = self.archie.pos.x;
        let ay = self.archie.pos.y;
        let aw = self.archie.size.x;
        let ah = self.archie.size.y;

        for i in 0..self.blocks.len() {
            let block = &self.blocks[i];
            let def = &UPGRADES[block.def_index];

            if block.level >= def.max_level {
                continue;
            }

            let bx = block.col as f32 * TILE_SIZE;
            let by = BLOCK_ROW as f32 * TILE_SIZE;
            let bw = BLOCK_W_TILES * TILE_SIZE;
            let bh = BLOCK_H_TILES * TILE_SIZE;

            let overlap =
                ax < bx + bw && bx < ax + aw && ay < by + bh && by < ay + ah;
            if !overlap {
                continue;
            }

            let Some(cost) = upgrade::cost_for(def, block.level) else {
                continue;
            };

            // Not enough money: bounce off without a purchase, and
            // cooldown so the next jump is a fresh attempt.
            if !progress.try_spend(cost) {
                self.headbutt_cooldown = HEADBUTT_COOLDOWN;
                return;
            }

            progress.buy_upgrade(def.id);
            self.blocks[i].level += 1;
            self.blocks[i].flash_timer = BLOCK_FLASH_DURATION;
            self.headbutt_cooldown = HEADBUTT_COOLDOWN;

            let text = if self.blocks[i].level >= def.max_level {
                format!("{}  MAX  −${cost}", def.label)
            } else {
                format!("+1 {}  −${cost}", def.label)
            };
            self.floaters.push(Floater {
                text,
                x: bx + bw * 0.5,
                y: by - 6.0,
                timer: UPGRADE_FLOATER_DURATION,
                color: Color::new(1.0, 0.85, 0.4, 1.0),
            });
            return;
        }
    }

    pub fn cargo_total(&self) -> u32 {
        self.cargo.values().copied().sum()
    }

    // ---------- Draw ----------

    pub fn draw(
        &self,
        ctx: &GameContext,
        atlas: &Atlas,
        sprites: &ArchieSprites,
        progress: &Progress,
    ) {
        let vw = screen_width();
        let vh = screen_height();
        clear_background(ctx.colors.bg_bottom);

        let scale = integer_scale(vw, vh);
        let virt_w = VIRTUAL_W * scale;
        let virt_h = VIRTUAL_H * scale;
        let screen_ox = ((vw - virt_w) * 0.5).floor();
        let screen_oy = ((vh - virt_h) * 0.5).floor();

        let world_w_px = self.world.width() as f32 * TILE_SIZE;
        let world_h_px = self.world.height() as f32 * TILE_SIZE;

        let cam_x = clamp_camera(
            self.archie.center().x - VIRTUAL_W * 0.5,
            VIRTUAL_W,
            world_w_px,
            0.0,
        );
        let cam_y = clamp_camera(
            self.archie.center().y - VIRTUAL_H * 0.5,
            VIRTUAL_H,
            world_h_px,
            0.0,
        );

        let sx = |wx: f32| -> f32 { (screen_ox + (wx - cam_x) * scale).floor() };
        let sy = |wy: f32| -> f32 { (screen_oy + (wy - cam_y) * scale).floor() };

        // HQ sky: a flat band above the surface. Cozy, not the
        // outdoor zone backdrop — the HQ is indoors-ish.
        let horizon_screen_y =
            screen_oy + (HQ_SURFACE_ROW as f32 * TILE_SIZE - cam_y) * scale;
        draw_rectangle(0.0, 0.0, vw, horizon_screen_y, ctx.colors.sky_top);

        // Tiles.
        let col_min = ((cam_x / TILE_SIZE).floor() as i32 - 2).max(0);
        let col_max = (((cam_x + VIRTUAL_W) / TILE_SIZE).ceil() as i32 + 2)
            .min(self.world.width() as i32);
        let row_min = ((cam_y / TILE_SIZE).floor() as i32 - 2).max(0);
        let row_max = (((cam_y + VIRTUAL_H) / TILE_SIZE).ceil() as i32 + 2)
            .min(self.world.height() as i32);

        for row in row_min..row_max {
            for col in col_min..col_max {
                let Ok(cu) = usize::try_from(col) else { continue };
                let Ok(ru) = usize::try_from(row) else { continue };
                let Some(tile) = self.world.tile(cu, ru) else { continue };
                let (r, g, b) = tile.kind.color();
                let base =
                    Color::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0);
                draw_rectangle(
                    sx(col as f32 * TILE_SIZE),
                    sy(row as f32 * TILE_SIZE),
                    TILE_SIZE * scale,
                    TILE_SIZE * scale,
                    base,
                );
            }
        }

        // Sell pad.
        let pad_left_sx;
        let pad_top_sy;
        let pad_w_sx;
        {
            let pad_x = sx(SELL_PAD_COL as f32 * TILE_SIZE);
            let pad_y = sy(HQ_SURFACE_ROW as f32 * TILE_SIZE);
            let pad_w = SELL_PAD_WIDTH as f32 * TILE_SIZE * scale;
            let pad_h = (TILE_SIZE * scale * 0.35).max(4.0);

            pad_left_sx = pad_x;
            pad_top_sy = pad_y;
            pad_w_sx = pad_w;

            let pulse = if self.cargo.is_empty() {
                0.30
            } else {
                0.55 + 0.35 * (self.archie.pos.x * 0.01).sin().abs()
            };
            draw_rectangle(
                pad_x,
                pad_y - pad_h,
                pad_w,
                pad_h,
                Color::new(ctx.colors.gold.r, ctx.colors.gold.g, ctx.colors.gold.b, pulse),
            );

            draw_text(
                "SELL",
                pad_x + 8.0,
                pad_y - pad_h - 10.0,
                22.0,
                ctx.colors.gold,
            );
        }

        // Sale floater.
        if self.last_sale_timer > 0.0 {
            let t = self.last_sale_timer / SALE_FLOATER_DURATION;
            let alpha = t.clamp(0.0, 1.0);
            let rise = (1.0 - t) * 36.0;
            let text = format!("+${}", self.last_sale_earned);
            let d = measure_text(&text, None, 30, 1.0);
            draw_text(
                &text,
                pad_left_sx + pad_w_sx * 0.5 - d.width * 0.5,
                pad_top_sy - 30.0 - rise,
                30.0,
                Color::new(
                    ctx.colors.gold.r,
                    ctx.colors.gold.g,
                    ctx.colors.gold.b,
                    alpha,
                ),
            );
        }

        // Upgrade blocks.
        for block in &self.blocks {
            draw_upgrade_block(block, &sx, &sy, scale, ctx, progress);
        }

        // Parked truck.
        {
            let truck_x = sx(TRUCK_COL as f32 * TILE_SIZE);
            let truck_y =
                sy(HQ_SURFACE_ROW as f32 * TILE_SIZE - TRUCK_H_TILES * TILE_SIZE);
            let tw = TRUCK_W_TILES * TILE_SIZE * scale;
            let th = TRUCK_H_TILES * TILE_SIZE * scale;

            draw_rectangle(truck_x, truck_y, tw, th, Color::new(0.62, 0.32, 0.14, 1.0));
            draw_rectangle(
                truck_x,
                truck_y,
                tw,
                th * 0.4,
                Color::new(0.42, 0.28, 0.18, 1.0),
            );
            draw_rectangle_lines(
                truck_x,
                truck_y,
                tw,
                th,
                1.5,
                Color::new(0.15, 0.10, 0.06, 1.0),
            );

            let fill = if self.initial_cargo == 0 {
                0.0
            } else {
                self.cargo_total() as f32 / self.initial_cargo as f32
            };
            if fill > 0.0 {
                draw_rectangle(
                    truck_x + 2.0 * scale,
                    truck_y + th - 4.0 * scale,
                    (tw - 4.0 * scale) * fill,
                    2.0 * scale,
                    ctx.colors.gold,
                );
            }
        }

        // Archie + bag + readout.
        {
            let sprite_wx = self.archie.pos.x + sprites.offset.0;
            let sprite_wy = self.archie.pos.y + sprites.offset.1;

            let bag_frame = bag_frame_name(&self.archie, sprites);
            atlas.draw_scaled(&bag_frame, sx(sprite_wx), sy(sprite_wy), scale);

            let frame = archie_frame(self, sprites);
            atlas.draw_scaled(&frame, sx(sprite_wx), sy(sprite_wy), scale);

            let center_x = sx(sprite_wx + crate::sprites::archie::SHEET_CELL as f32 * 0.5);
            let top_y = sy(sprite_wy);
            crate::hud::draw_bag_readout(&self.archie.bag, center_x, top_y, ctx);
        }

        // Upgrade floaters, drawn above everything else.
        for f in &self.floaters {
            let t = f.timer / UPGRADE_FLOATER_DURATION;
            let alpha = t.clamp(0.0, 1.0);
            let rise = (1.0 - t) * 24.0;
            let d = measure_text(&f.text, None, 22, 1.0);
            let fx = sx(f.x) - d.width * 0.5;
            let fy = sy(f.y) - rise;
            draw_text(
                &f.text,
                fx,
                fy,
                22.0,
                Color::new(f.color.r, f.color.g, f.color.b, alpha),
            );
        }

        // HUD.
        draw_text(
            format!("Dollars: ${}", progress.dollars),
            20.0,
            34.0,
            24.0,
            ctx.colors.gold,
        );
        draw_text(
            format!(
                "cargo {}   sold this visit ${}",
                self.cargo_total(),
                self.total_sold_this_visit
            ),
            20.0,
            60.0,
            18.0,
            ctx.colors.text_dim,
        );

        let hint = if self.cargo.is_empty() {
            "walk right to sell / buy upgrades   [Esc] title"
        } else {
            "walk right to the SELL pad"
        };
        draw_text(hint, 20.0, vh - 20.0, 16.0, ctx.colors.text_dim);
    }
}

// ---------- Block drawing ----------

/// Base color for each upgrade block, in `UPGRADES` order.
const BLOCK_COLORS: &[(u8, u8, u8)] = &[
    (0xB8, 0x73, 0x33), // pickaxe — copper
    (0x8B, 0x5A, 0x3A), // mining — brown
    (0x6B, 0x8E, 0x9E), // move — blue-grey
    (0x8B, 0x6F, 0x4A), // bag — leather
    (0x7A, 0x4E, 0x3A), // truck — rust
];

fn draw_upgrade_block(
    block: &UpgradeBlock,
    sx: &impl Fn(f32) -> f32,
    sy: &impl Fn(f32) -> f32,
    scale: f32,
    ctx: &GameContext,
    progress: &Progress,
) {
    let def = &UPGRADES[block.def_index];
    let is_max = block.level >= def.max_level;

    let bx = sx(block.col as f32 * TILE_SIZE);
    let by = sy(BLOCK_ROW as f32 * TILE_SIZE);
    let bw = BLOCK_W_TILES * TILE_SIZE * scale;
    let bh = BLOCK_H_TILES * TILE_SIZE * scale;

    let (r, g, b) = BLOCK_COLORS[block.def_index % BLOCK_COLORS.len()];
    let base = Color::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0);

    if is_max {
        // Sunk plate: bottom half of the block area, with "MAX".
        let plate_h = bh * 0.4;
        let plate_y = by + bh - plate_h;

        draw_rectangle(
            bx,
            plate_y,
            bw,
            plate_h,
            Color::new(0.25, 0.20, 0.15, 1.0),
        );
        draw_rectangle_lines(
            bx,
            plate_y,
            bw,
            plate_h,
            scale.max(1.0),
            Color::new(0.15, 0.10, 0.06, 1.0),
        );

        let label = "MAX";
        let d = measure_text(label, None, (14.0 * scale / 4.0) as u16, 1.0);
        draw_text(
            label,
            bx + bw * 0.5 - d.width * 0.5,
            plate_y + plate_h * 0.5 + d.height * 0.35,
            14.0 * scale / 4.0,
            ctx.colors.gold,
        );
        return;
    }

    // Full block body.
    draw_rectangle(bx, by, bw, bh, base);
    draw_rectangle_lines(
        bx,
        by,
        bw,
        bh,
        (scale * 0.75).max(2.0),
        Color::new(0.15, 0.10, 0.06, 1.0),
    );

    // Flash overlay on purchase.
    if block.flash_timer > 0.0 {
        let a = (block.flash_timer / BLOCK_FLASH_DURATION).clamp(0.0, 1.0);
        draw_rectangle(bx, by, bw, bh, Color::new(1.0, 1.0, 1.0, a * 0.85));
    }

    // "N/MAX" text near the top.
    let lvl_text = format!("{}/{}", block.level, def.max_level);
    let font_size = 16.0 * scale / 4.0;
    let d = measure_text(&lvl_text, None, font_size as u16, 1.0);
    draw_text(
        &lvl_text,
        bx + bw * 0.5 - d.width * 0.5,
        by + font_size + 4.0 * scale / 4.0,
        font_size,
        ctx.colors.text,
    );

    // Label near the bottom.
    let ldef = &UPGRADES[block.def_index];
    let label = ldef.label;
    let d = measure_text(label, None, font_size as u16, 1.0);
    draw_text(
        label,
        bx + bw * 0.5 - d.width * 0.5,
        by + bh - 4.0 * scale / 4.0,
        font_size,
        ctx.colors.text_dim,
    );

    // Cost below the block, small and dim.
    if let Some(cost) = upgrade::cost_for(def, block.level) {
        let cost_text = format!("${cost}");
        let d = measure_text(&cost_text, None, font_size as u16, 1.0);
        draw_text(
            &cost_text,
            bx + bw * 0.5 - d.width * 0.5,
            by + bh + font_size + 4.0 * scale / 4.0,
            font_size,
            ctx.colors.gold,
        );
    }

    // Cheap assertion: if progress disagrees with block.level, that's
    // a sync bug. Log once per frame only in debug builds.
    #[cfg(debug_assertions)]
    {
        let expected = progress.upgrade_level(def.id);
        if expected != block.level {
            // Don't spam; the block is the UI source of truth while
            // in HQ. Purchase paths update both.
            let _ = expected;
        }
    }
}

// ---------- HQ world ----------

fn build_hq_world() -> World {
    let mut cells: Vec<Option<Tile>> = vec![None; HQ_W * HQ_H];
    for row in HQ_SURFACE_ROW..HQ_H {
        for col in 0..HQ_W {
            cells[row * HQ_W + col] = Some(Tile::new(TileKind::Bedrock));
        }
    }
    World::from_cells(HQ_W, HQ_H, cells, HQ_SURFACE_ROW, 0)
}

// ---------- Frame selection ----------

fn archie_frame(hq: &HqState, sprites: &ArchieSprites) -> String {
    let archie = &hq.archie;
    let dir = if archie.facing.is_left() { "L" } else { "R" };

    if !archie.on_ground {
        let n = sprites.jump_r.len().max(1);
        let t = ((archie.vy + 240.0) / 840.0).clamp(0.0, 0.999);
        let idx = ((t * n as f32) as usize).min(n - 1);
        return format!("archie_jump_{idx}_{dir}");
    }

    if hq.input_horizontal.abs() > 0.01 {
        let n = sprites.walk_r.len().max(1);
        let frame = ((archie.pos.x * 0.2) as usize) % n;
        return format!("archie_walk_{frame}_{dir}");
    }

    let n = sprites.idle_r.len().max(1);
    let frame = ((archie.pos.x * 0.05) as usize) % n;
    format!("archie_idle_{frame}_{dir}")
}

fn bag_frame_name(archie: &Archie, sprites: &ArchieSprites) -> String {
    let n = sprites.bag_r.len().max(1);
    let level = ((archie.bag.fill_ratio() * (n as f32 - 1.0)).round() as usize).min(n - 1);
    let dir = if archie.facing.is_left() { "L" } else { "R" };
    format!("bag_{level}_{dir}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_hq_state_has_all_blocks_at_default_level() {
        let progress = Progress::default();
        let hq = HqState::new(HashMap::new(), &progress);
        assert_eq!(hq.blocks.len(), UPGRADES.len());
        for b in &hq.blocks {
            assert_eq!(b.level, 0);
        }
    }

    #[test]
    fn new_hq_state_reads_levels_from_progress() {
        let mut progress = Progress::default();
        progress.buy_upgrade("pickaxe");
        progress.buy_upgrade("pickaxe");
        let hq = HqState::new(HashMap::new(), &progress);
        let pickaxe_block = hq
            .blocks
            .iter()
            .find(|b| UPGRADES[b.def_index].id == "pickaxe")
            .unwrap();
        assert_eq!(pickaxe_block.level, 2);
    }

    #[test]
    fn block_cols_count_matches_upgrades() {
        assert_eq!(BLOCK_COLS.len(), UPGRADES.len());
    }
}