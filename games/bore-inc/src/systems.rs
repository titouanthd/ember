//! The state structs (`AppState` for the app shell, `Game` for the
//! gameplay simulation), the screen enum, and the screen-level input
//! dispatch.

use macroquad::prelude::*;

use crate::atlas::Atlas;
use crate::audio::MusicPlayer;
use crate::background::Background;
use crate::components::{TileKind, TILE_SIZE};
use crate::config::GameContext;
use crate::drop::Drop;
use crate::hq::HqState;
use crate::input::GameInput;
use crate::levelgen::{generate, LevelKind};
use crate::player::{Archie, ARCHIE_HEIGHT};
use crate::progress::Progress;
use crate::sprites::archie::ArchieSprites;
use crate::truck::{Truck, TRUCK_HEIGHT};
use crate::world::World;

const LEVEL_COMPLETE_DELAY: f32 = 1.5;
const MINE_FPS: f32 = 12.0;
const JUMP_VY_RANGE: f32 = 840.0;
const JUMP_VY_OFFSET: f32 = 240.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelExit {
    FinishLine,
    ReturnToHq,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppScreen {
    WebStart,
    Title,
    Intro,
    Game,
    Hq,
    Map,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppAction {
    Continue,
    Quit,
}

pub struct AppState {
    pub screen: AppScreen,
    pub progress: Progress,
    pub audio: MusicPlayer,
    pub time: f32,
    pub screen_elapsed: f32,
    pub game: Option<Game>,
    pub hq: Option<HqState>,
    pub atlas: Atlas,
    pub archie_sprites: ArchieSprites,
    pub background: Background,

    pub current_level: LevelKind,
    pub progress_dirty: bool,
    pub time_since_save: f32,

    #[cfg(debug_assertions)]
    pub debug_open: bool,
    #[cfg(debug_assertions)]
    pub debug_level: LevelKind,
    #[cfg(debug_assertions)]
    pub debug_seed: u32,

    pub truck_sprites: crate::sprites::truck::TruckSprites,
}

impl AppState {
    pub fn new(
        progress: Progress,
        audio: MusicPlayer,
        atlas: Atlas,
        archie_sprites: ArchieSprites,
        truck_sprites: crate::sprites::truck::TruckSprites,
        background: Background,
    ) -> Self {
        Self {
            screen: AppScreen::Title,
            progress,
            audio,
            time: 0.0,
            screen_elapsed: 0.0,
            game: None,
            hq: None,
            atlas,
            archie_sprites,
            truck_sprites,
            background,

            current_level: LevelKind::One,
            progress_dirty: false,
            time_since_save: 0.0,

            #[cfg(debug_assertions)]
            debug_open: false,
            #[cfg(debug_assertions)]
            debug_level: LevelKind::One,
            #[cfg(debug_assertions)]
            debug_seed: 42,
        }
    }

    pub fn goto(&mut self, screen: AppScreen) {
        if screen == AppScreen::Game && self.game.is_none() {
            self.load_level(self.current_level);
        }
        if screen == AppScreen::Hq && self.hq.is_none() {
            self.hq = Some(HqState::new(
                std::collections::HashMap::new(),
                &self.progress,
            ));
        }
        self.screen = screen;
        self.screen_elapsed = 0.0;
    }

    fn load_level(&mut self, kind: LevelKind) {
        let fallback_seed = (self.time * 1_000_000.0) as u32;
        let seed = self.progress.level_seed(kind.id(), fallback_seed);
        let mut game = Game::with_seed(seed, kind);
        game.apply_upgrades(&self.progress);
        game.world.apply_delta(&self.progress.level_delta(kind.id()));
        self.game = Some(game);
        self.current_level = kind;
    }

    pub fn tick(&mut self, _ctx: &GameContext, dt: f32) -> AppAction {
        self.time += dt;
        self.screen_elapsed += dt;
        self.audio.tick(dt);
        self.background.tick(dt);

        #[cfg(debug_assertions)]
        {
            if is_key_pressed(KeyCode::F1) {
                self.debug_open = !self.debug_open;
            }
            if self.screen == AppScreen::Hq && is_key_pressed(KeyCode::F2) {
                self.progress.add_dollars(500);
                self.progress_dirty = true;
                println!("[debug] +$500 (total ${})", self.progress.dollars);
            }
            if self.debug_open {
                self.tick_debug();
                return AppAction::Continue;
            }
            if self.screen == AppScreen::Game
                && is_key_pressed(KeyCode::T)
                && let Some(game) = self.game.as_mut()
            {
                game.teleport_truck_to_finish();
            }
            if let Some(target) = self.dev_shortcut() {
                self.goto(target);
                return AppAction::Continue;
            }
        }

        match self.screen {
            AppScreen::WebStart => self.tick_webstart(),
            AppScreen::Title => self.tick_title(),
            AppScreen::Intro => self.tick_intro(),
            AppScreen::Game => self.tick_game(dt),
            AppScreen::Hq => self.tick_hq(dt),
            AppScreen::Map => self.tick_map(),
        }
    }

    #[cfg(debug_assertions)]
    fn tick_debug(&mut self) {
        if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::Down) {
            self.debug_level = self.debug_level.cycle();
        }

        if is_key_pressed(KeyCode::R) {
            self.debug_seed = (self.time * 1_000_000.0) as u32;
        }

        if is_key_pressed(KeyCode::Enter) {
            self.progress
                .set_level_seed(self.debug_level.id(), self.debug_seed);
            self.progress_dirty = true;
            self.current_level = self.debug_level;
            self.load_level(self.debug_level);
            self.goto(AppScreen::Game);
            self.debug_open = false;
        }

        if is_key_pressed(KeyCode::Escape) {
            self.debug_open = false;
        }
    }

    fn tick_webstart(&mut self) -> AppAction {
        let any_key = get_last_key_pressed().is_some();
        let clicked = is_mouse_button_pressed(MouseButton::Left);
        if any_key || clicked {
            self.audio.start();
            self.goto(AppScreen::Title);
        }
        AppAction::Continue
    }

    fn tick_title(&mut self) -> AppAction {
        if movement_key_pressed() {
            self.goto(AppScreen::Intro);
        }
        AppAction::Continue
    }

    fn tick_intro(&mut self) -> AppAction {
        if movement_key_pressed() {
            self.goto(AppScreen::Game);
        }
        AppAction::Continue
    }

    fn tick_game(&mut self, dt: f32) -> AppAction {
        if is_key_pressed(KeyCode::R) {
            let seed = (self.time * 1000.0) as u32;
            let kind = self.current_level;
            self.progress.set_level_seed(kind.id(), seed);
            self.progress_dirty = true;
            self.load_level(kind);
            return AppAction::Continue;
        }

        let input = GameInput::from_keyboard();
        if let Some(game) = self.game.as_mut() {
            game.tick(dt, input);
        }

        // Transition out of the level. Snapshot what we need and
        // drop the Game so re-entering regenerates from seed + delta.
        let transition = self.game.as_ref().and_then(|g| {
            if let Some(reason) = g.exit
                && g.exit_time > LEVEL_COMPLETE_DELAY
            {
                Some((
                    reason,
                    self.current_level,
                    g.world.seed,
                    g.truck.storage.clone(),
                    g.world.delta.clone(),
                ))
            } else {
                None
            }
        });
        if let Some((reason, kind, seed, cargo, delta)) = transition {
            // Persist the tunnel first (so it survives even if the
            // next thing we do is panic).
            self.progress.set_level_delta(kind.id(), delta);

            if reason == LevelExit::FinishLine {
                self.progress.mark_level_completed(kind.id(), seed);
                // Advance the pointer so the next `Game` entry loads
                // the following level.
                self.current_level = kind.next();
            }

            self.progress_dirty = true;
            self.hq = Some(HqState::new(cargo, &self.progress));
            self.game = None;
            self.goto(AppScreen::Hq);
            return AppAction::Continue;
        }

        if is_key_pressed(KeyCode::Escape) {
            self.goto(AppScreen::Title);
        }
        AppAction::Continue
    }

    fn tick_hq(&mut self, dt: f32) -> AppAction {
        let input = GameInput::from_keyboard();
        if let Some(hq) = self.hq.as_mut() {
            hq.tick(dt, input, &mut self.progress);
        }
        if is_key_pressed(KeyCode::Escape) {
            self.goto(AppScreen::Title);
        }
        AppAction::Continue
    }

    fn tick_map(&mut self) -> AppAction {
        if is_key_pressed(KeyCode::Escape) {
            self.goto(AppScreen::Title);
        }
        AppAction::Continue
    }

    #[cfg(debug_assertions)]
    fn dev_shortcut(&self) -> Option<AppScreen> {
        if self.screen == AppScreen::WebStart {
            return None;
        }
        if is_key_pressed(KeyCode::Key1) {
            Some(AppScreen::Title)
        } else if is_key_pressed(KeyCode::Key2) {
            Some(AppScreen::Game)
        } else if is_key_pressed(KeyCode::Key3) {
            Some(AppScreen::Hq)
        } else if is_key_pressed(KeyCode::Key4) {
            Some(AppScreen::Map)
        } else {
            None
        }
    }

    pub fn draw(&self, ctx: &GameContext) {
        match self.screen {
            AppScreen::WebStart => self.draw_webstart(ctx),
            AppScreen::Title => self.draw_title(ctx),
            AppScreen::Intro => self.draw_placeholder(ctx, "INTRO", "TODO"),
            AppScreen::Game => self.draw_game(ctx),
            AppScreen::Hq => self.draw_hq(ctx),
            AppScreen::Map => self.draw_placeholder(ctx, "MAP", "TODO"),
        }

        #[cfg(debug_assertions)]
        self.draw_debug(ctx);
    }

    fn draw_webstart(&self, ctx: &GameContext) {
        let vw = screen_width();
        let vh = screen_height();
        clear_background(Color::new(0.04, 0.03, 0.016, 1.0));

        let title = "BORE INC.";
        let d = measure_text(title, None, 56, 1.0);
        draw_text(title, vw * 0.5 - d.width * 0.5, vh * 0.42, 56.0, ctx.colors.gold);

        let prompt = "Click anywhere to begin";
        let d = measure_text(prompt, None, 20, 1.0);
        let pulse = 0.55 + 0.45 * (self.time * 2.0).sin().abs();
        draw_text(
            prompt,
            vw * 0.5 - d.width * 0.5,
            vh * 0.72,
            20.0,
            Color::new(ctx.colors.gold.r, ctx.colors.gold.g, ctx.colors.gold.b, pulse),
        );
    }

    fn draw_title(&self, ctx: &GameContext) {
        let vw = screen_width();
        let vh = screen_height();
        clear_background(ctx.colors.bg_bottom);
        draw_rectangle(0.0, 0.0, vw, vh * 0.65, ctx.colors.bg_top);

        let title = "BORE INC.";
        let d = measure_text(title, None, 72, 1.0);
        draw_text(title, vw * 0.5 - d.width * 0.5, vh * 0.36, 72.0, ctx.colors.gold);

        let sub = "An incremental mining game";
        let d = measure_text(sub, None, 20, 1.0);
        draw_text(sub, vw * 0.5 - d.width * 0.5, vh * 0.36 + 38.0, 20.0, ctx.colors.text_dim);

        let prompt = "Press any movement key to begin";
        let d = measure_text(prompt, None, 18, 1.0);
        let pulse = 0.55 + 0.45 * (self.time * 2.0).sin().abs();
        draw_text(
            prompt,
            vw * 0.5 - d.width * 0.5,
            vh * 0.72,
            18.0,
            Color::new(ctx.colors.text.r, ctx.colors.text.g, ctx.colors.text.b, pulse),
        );
    }

    fn draw_placeholder(&self, ctx: &GameContext, big: &str, small: &str) {
        let vw = screen_width();
        let vh = screen_height();
        clear_background(ctx.colors.bg_bottom);
        draw_rectangle(0.0, 0.0, vw, vh * 0.65, ctx.colors.bg_top);

        let d = measure_text(big, None, 64, 1.0);
        draw_text(big, vw * 0.5 - d.width * 0.5, vh * 0.44, 64.0, ctx.colors.gold);

        let d = measure_text(small, None, 18, 1.0);
        draw_text(small, vw * 0.5 - d.width * 0.5, vh * 0.44 + 34.0, 18.0, ctx.colors.text_dim);

        let meta = format!("tile = {TILE_SIZE}px");
        let d = measure_text(&meta, None, 12, 1.0);
        draw_text(&meta, vw * 0.5 - d.width * 0.5, vh * 0.44 + 64.0, 12.0, ctx.colors.text_dim);

        let hint = "[Esc] Title";
        let d = measure_text(hint, None, 14, 1.0);
        draw_text(hint, vw * 0.5 - d.width * 0.5, vh - 32.0, 14.0, ctx.colors.text_dim);
    }

    fn draw_game(&self, ctx: &GameContext) {
        if let Some(game) = self.game.as_ref() {
            game.draw(
                ctx,
                &self.atlas,
                &self.archie_sprites,
                &self.truck_sprites,
                &self.background,
            );
        } else {
            self.draw_placeholder(ctx, "GAME", "no game — press R");
        }
    }

    fn draw_hq(&self, ctx: &GameContext) {
        if let Some(hq) = self.hq.as_ref() {
            hq.draw(ctx, &self.atlas, &self.archie_sprites, &self.progress);
        } else {
            self.draw_placeholder(ctx, "HQ", "no HQ state");
        }
    }

    #[cfg(debug_assertions)]
    fn draw_debug(&self, ctx: &GameContext) {
        if !self.debug_open {
            return;
        }
        let vw = screen_width();
        let panel_w = 290.0;
        let panel_h = 170.0;
        let px = vw - panel_w - 20.0;
        let py = 20.0;

        draw_rectangle(px, py, panel_w, panel_h, Color::new(0.0, 0.0, 0.0, 0.85));
        draw_rectangle_lines(px, py, panel_w, panel_h, 2.0, ctx.colors.gold);

        let tx = px + 14.0;
        let mut ty = py + 28.0;

        draw_text("DEBUG", tx, ty, 20.0, ctx.colors.gold);
        ty += 28.0;

        let level_name = self.debug_level.label();
        draw_text(
            format!("[Up/Down] Level: {level_name}"),
            tx,
            ty,
            16.0,
            ctx.colors.text,
        );
        ty += 22.0;

        draw_text(
            format!("[R]       Seed: {}", self.debug_seed),
            tx,
            ty,
            16.0,
            ctx.colors.text,
        );
        ty += 22.0;

        draw_text("[Enter]   load level", tx, ty, 16.0, ctx.colors.gold);
        ty += 22.0;

        draw_text("[F1/Esc]  close", tx, ty, 16.0, ctx.colors.text_dim);
    }
}

pub fn movement_key_pressed() -> bool {
    is_key_pressed(KeyCode::Left)
        || is_key_pressed(KeyCode::Right)
        || is_key_pressed(KeyCode::Up)
        || is_key_pressed(KeyCode::Down)
        || is_key_pressed(KeyCode::A)
        || is_key_pressed(KeyCode::Q)
        || is_key_pressed(KeyCode::Space)
}

// =========================================================================
// Game
// =========================================================================

pub struct Game {
    pub world: World,
    pub archie: Archie,
    pub truck: Truck,
    pub archie_in_truck: bool,
    pub truck_enter_locked: bool,
    pub drops: Vec<Drop>,
    pub time: f32,
    pub input_horizontal: f32,
    pub jump_pressed_this_frame: bool,
    pub mining_this_frame: bool,

    pub exit: Option<LevelExit>,
    pub exit_time: f32,
}

impl Game {
    pub fn new() -> Self {
        Self::with_world(generate(42, LevelKind::One))
    }

    pub fn with_seed(seed: u32, kind: LevelKind) -> Self {
        Self::with_world(generate(seed, kind))
    }

    pub fn with_world(world: World) -> Self {
        let start_x = 3.0 * TILE_SIZE;
        let start_y = world.surface_row as f32 * TILE_SIZE - ARCHIE_HEIGHT;
        let archie = Archie::new(glam::Vec2::new(start_x, start_y));

        let truck_x = 1.0 * TILE_SIZE;
        let truck_y = world.surface_row as f32 * TILE_SIZE - TRUCK_HEIGHT;
        let truck = Truck::new(glam::Vec2::new(truck_x, truck_y));

        Self {
            world,
            archie,
            truck,
            archie_in_truck: false,
            truck_enter_locked: false,
            drops: Vec::new(),
            time: 0.0,
            input_horizontal: 0.0,
            jump_pressed_this_frame: false,
            mining_this_frame: false,
            exit: None,
            exit_time: 0.0,
        }
    }

    pub fn apply_upgrades(&mut self, progress: &Progress) {
        for def in crate::upgrade::UPGRADES {
            let level = progress.upgrade_level(def.id);
            crate::upgrade::apply(def.id, level, &mut self.archie, &mut self.truck);
        }
    }

    pub fn tick(&mut self, dt: f32, input: GameInput) {
        self.time += dt;

        if self.exit.is_some() {
            self.exit_time += dt;
            return;
        }

        self.input_horizontal = input.horizontal;
        self.jump_pressed_this_frame = input.jump_pressed;

        if self.archie_in_truck {
            self.tick_driving(dt, input);
        } else {
            self.tick_on_foot(dt, input);
        }

        self.tick_drops(dt);
        self.collect_drops();

        self.check_exit();
    }

    fn check_exit(&mut self) {
        if self.exit.is_some() {
            return;
        }

        let finish_px = self.world.finish_col() as f32 * TILE_SIZE;
        let truck_right = self.truck.pos.x + self.truck.size.x;
        if truck_right >= finish_px {
            self.exit = Some(LevelExit::FinishLine);
            self.exit_time = 0.0;
            return;
        }

        if self.truck.pos.x <= 2.0 {
            self.exit = Some(LevelExit::ReturnToHq);
            self.exit_time = 0.0;
        }
    }

    fn tick_on_foot(&mut self, dt: f32, input: GameInput) {
        let break_result = self.archie.tick(dt, input, &mut self.world);
        self.mining_this_frame = self.archie.last_mine_dir.is_some();

        if let Some(b) = break_result {
            self.spawn_drop_from_break(b);
        }

        self.try_enter_truck(input);
    }

    fn tick_driving(&mut self, dt: f32, input: GameInput) {
        self.mining_this_frame = false;
        self.truck.tick(dt, input.horizontal, &self.world);

        if input.jump_pressed {
            self.archie_in_truck = false;
            self.archie.pos = self.truck.pos
                + glam::Vec2::new(
                    (self.truck.size.x - self.archie.size.x) * 0.5,
                    -self.archie.size.y - 2.0,
                );
            self.archie.vy = 0.0;
            self.archie.on_ground = false;
            return;
        }

        self.archie.pos = self.truck.pos
            + glam::Vec2::new(
                (self.truck.size.x - self.archie.size.x) * 0.5,
                (self.truck.size.y - self.archie.size.y) * 0.5,
            );
        self.archie.vy = 0.0;
        self.archie.on_ground = true;
    }

    fn try_enter_truck(&mut self, input: GameInput) {
        if input.horizontal == 0.0 {
            self.truck_enter_locked = false;
            return;
        }
        if self.truck_enter_locked {
            return;
        }
        if !self.archie_overlaps_truck() {
            return;
        }

        let archie_cx = self.archie.pos.x + self.archie.size.x * 0.5;
        let truck_cx = self.truck.center().x;
        let dir_to_truck = (truck_cx - archie_cx).signum();
        if dir_to_truck == 0.0 || input.horizontal.signum() != dir_to_truck {
            return;
        }

        if self.archie.bag_used() > 0 {
            let contents = self.archie.empty_bag();
            for (res, n) in contents {
                let accepted = self.truck.deposit(res, n);
                let leftover = n - accepted;
                if leftover > 0 {
                    self.archie.collect(res, leftover);
                }
            }
            self.truck_enter_locked = true;
            return;
        }

        self.archie_in_truck = true;
        self.archie.pos = self.truck.pos
            + glam::Vec2::new(
                (self.truck.size.x - self.archie.size.x) * 0.5,
                (self.truck.size.y - self.archie.size.y) * 0.5,
            );
        self.archie.vy = 0.0;
        self.archie.on_ground = true;
    }

    fn archie_overlaps_truck(&self) -> bool {
        let a = self.archie.pos;
        let asz = self.archie.size;
        let t = self.truck.pos;
        let tsz = self.truck.size;
        a.x < t.x + tsz.x && t.x < a.x + asz.x && a.y < t.y + tsz.y && t.y < a.y + asz.y
    }

    fn spawn_drop_from_break(&mut self, b: crate::player::BreakResult) {
        let Some(resource) = b.kind.drop() else {
            return;
        };
        let x = b.col as f32 * TILE_SIZE + TILE_SIZE * 0.5;
        let y = b.row as f32 * TILE_SIZE + TILE_SIZE * 0.5;
        self.drops.push(Drop::new(glam::Vec2::new(x, y), resource, 1));
    }

    fn tick_drops(&mut self, dt: f32) {
        for drop in self.drops.iter_mut() {
            drop.tick(dt, &self.world);
        }
    }

    fn collect_drops(&mut self) {
        let mut i = 0;
        while i < self.drops.len() {
            let d = self.drops[i];
            if !d.overlaps_box(self.archie.pos, self.archie.size) {
                i += 1;
                continue;
            }
            let taken = self.archie.collect(d.resource, d.count);
            if taken >= d.count {
                self.drops.swap_remove(i);
                continue;
            }
            self.drops[i].count = d.count - taken;
            i += 1;
        }
    }

    #[cfg(debug_assertions)]
    pub fn teleport_truck_to_finish(&mut self) {
        let finish_col = self.world.finish_col();
        let finish_px = finish_col as f32 * TILE_SIZE;

        let mut surface_row = 0usize;
        for r in 0..self.world.height() {
            if self.world.is_solid(finish_col as isize, r as isize) {
                surface_row = r;
                break;
            }
        }

        self.truck.pos.x = finish_px - self.truck.size.x - 4.0;
        self.truck.pos.y = surface_row as f32 * TILE_SIZE - self.truck.size.y;
        self.truck.blocked = None;

        self.archie_in_truck = true;
        self.archie.pos = self.truck.pos
            + glam::Vec2::new(
                (self.truck.size.x - self.archie.size.x) * 0.5,
                (self.truck.size.y - self.archie.size.y) * 0.5,
            );
        self.archie.vy = 0.0;
        self.archie.on_ground = true;
    }

    pub fn draw(
        &self,
        ctx: &GameContext,
        atlas: &Atlas,
        archie_sprites: &ArchieSprites,
        truck_sprites: &crate::sprites::truck::TruckSprites,
        background: &Background,
    ) {
        let vw = screen_width();
        let vh = screen_height();
        clear_background(ctx.colors.bg_bottom);

        let scale = integer_scale(vw, vh);
        let virt_w = VIRTUAL_W * scale;
        let virt_h = VIRTUAL_H * scale;
        let screen_ox = ((vw - virt_w) * 0.5).floor();
        let screen_oy = ((vh - virt_h) * 0.5).floor();

        let cam_x = clamp_camera(
            self.archie.center().x - VIRTUAL_W * 0.5,
            VIRTUAL_W,
            self.world.width() as f32 * TILE_SIZE,
            CAMERA_OVERSCROLL_X,
        );
        let cam_y = clamp_camera(
            self.archie.center().y - VIRTUAL_H * 0.5,
            VIRTUAL_H,
            self.world.height() as f32 * TILE_SIZE,
            CAMERA_OVERSCROLL_Y,
        );

        draw_sky_gradient(
            screen_oy,
            virt_h,
            self.world.surface_row as f32 * TILE_SIZE,
            cam_y,
            scale,
            ctx.colors.sky_top,
            ctx.colors.sky_bottom,
            ctx.colors.bg_bottom,
        );

        let horizon_screen_y = screen_oy
            + (self.world.surface_row as f32 * TILE_SIZE - cam_y) * scale;

        background.draw(cam_x, horizon_screen_y, screen_ox, vw, scale);

        let sx = |wx: f32| -> f32 { (screen_ox + (wx - cam_x) * scale).floor() };
        let sy = |wy: f32| -> f32 { (screen_oy + (wy - cam_y) * scale).floor() };

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
                let x = sx(col as f32 * TILE_SIZE);
                let y = sy(row as f32 * TILE_SIZE);
                let s = TILE_SIZE * scale;
                draw_rectangle(x, y, s, s, base);

                // Grass tufts on exposed dirt tops.
                if tile.kind == TileKind::Dirt
                    && ru > 0
                    && self.world.tile(cu, ru - 1).is_none()
                {
                    draw_grass_tufts(
                        x,
                        y,
                        s,
                        cu,
                        ru,
                        self.world.seed,
                        ctx.colors.grass,
                        scale,
                    );
                }

                // Damage chips + crack overlay, only for damaged tiles.
                let stage = tile.crack_stage();
                if stage > 0 {
                    draw_tile_damage_chips(
                        x, y, s,
                        cu, ru, self.world.seed,
                        stage,
                        (r, g, b),
                        scale,
                    );
                    let a = 0.10 + 0.06 * stage as f32;
                    draw_rectangle(x, y, s, s, Color::new(0.0, 0.0, 0.0, a));
                }

                // Corner chamfers: soften exposed corners so the
                // terrain reads as slopes, not staircase.
                draw_corner_chamfers(
                    x, y, s, scale,
                    &self.world, cu, ru,
                    ctx.colors.bg_bottom,
                );
            }
        }

        draw_hq_sign(&sx, &sy, scale, self.world.surface_row, ctx);

        {
            let finish_col = self.world.finish_col();
            let finish_x = sx(finish_col as f32 * TILE_SIZE);
            let y_top = screen_oy;
            let y_bot = screen_oy + virt_h;
            let pulse = 0.35 + 0.15 * (self.time * 3.0).sin().abs();

            draw_rectangle(
                finish_x,
                y_top,
                TILE_SIZE * scale,
                y_bot - y_top,
                Color::new(0.83, 0.65, 0.31, pulse * 0.4),
            );
            draw_rectangle(
                finish_x,
                y_top,
                (2.0 * scale).max(2.0),
                y_bot - y_top,
                Color::new(1.0, 0.85, 0.4, pulse),
            );
        }

        for d in &self.drops {
            let (r, g, b) = drop_color(d.resource);
            let c = Color::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0);
            let r_px = crate::drop::DROP_RADIUS;
            draw_rectangle(
                sx(d.pos.x - r_px),
                sy(d.pos.y - r_px),
                r_px * 2.0 * scale,
                r_px * 2.0 * scale,
                c,
            );
        }

        {
            let t = &self.truck;
            let tx = sx(t.pos.x);
            let ty = sy(t.pos.y);
            let tw = t.size.x * scale;
            let th = t.size.y * scale;

            // Wheel-spin frame: one spin step every 4 world px.
            let frame = ((t.distance_traveled / 4.0) as usize) % 2;
            let dir = if t.facing_right { "R" } else { "L" };
            let name = format!("truck_{frame}_{dir}");

            // Sprite cell is 32×32, matches the physics box exactly.
            let sprite_scale = scale;
            let sprite = if t.facing_right {
                &truck_sprites.r[frame]
            } else {
                &truck_sprites.l[frame]
            };
            // Draw via atlas.
            if let Some(rect) = atlas.rect(&name) {
                let _ = sprite;
                atlas.draw_scaled(&name, tx, ty, sprite_scale);
                let _ = rect;
            }

            // Cargo fill bar above the truck.
            let fill = t.storage_used() as f32 / t.storage_capacity.max(1) as f32;
            if fill > 0.0 {
                let bar_w = tw * fill;
                let bar_h = (2.0 * scale).max(2.0);
                let bar_y = ty - bar_h - 2.0 * scale;
                draw_rectangle(
                    tx + 2.0 * scale,
                    bar_y,
                    (tw - 4.0 * scale).max(1.0),
                    bar_h,
                    Color::new(0.0, 0.0, 0.0, 0.55),
                );
                draw_rectangle(
                    tx + 2.0 * scale,
                    bar_y,
                    bar_w,
                    bar_h,
                    ctx.colors.gold,
                );
            }

            // Directional blocked arrow.
            if let Some(reason) = t.blocked {
                draw_block_arrow(
                    t.facing_right,
                    reason,
                    tx,
                    ty,
                    tw,
                    th,
                    scale,
                    ctx.colors.danger,
                );
            }
        }

        if !self.archie_in_truck {
            let sprite_wx = self.archie.pos.x + archie_sprites.offset.0;
            let sprite_wy = self.archie.pos.y + archie_sprites.offset.1;

            let bag_frame = bag_frame_name(&self.archie, archie_sprites);
            atlas.draw_scaled(&bag_frame, sx(sprite_wx), sy(sprite_wy), scale);

            let frame = archie_frame_name(self, archie_sprites);
            atlas.draw_scaled(&frame, sx(sprite_wx), sy(sprite_wy), scale);

            let center_x = sx(sprite_wx + crate::sprites::archie::SHEET_CELL as f32 * 0.5);
            let top_y = sy(sprite_wy);
            crate::hud::draw_bag_readout(&self.archie.bag, center_x, top_y, ctx);
        }

        let state = if self.archie.on_ground { "ground" } else { "air" };
        let mode = if self.archie_in_truck { "DRIVING" } else { "on foot" };
        let line = format!(
            "pos ({:.0}, {:.0})  vy {:+.0}  {}  {}  drops {}  bag {}/{}  truck {}/{}",
            self.archie.pos.x,
            self.archie.pos.y,
            self.archie.vy,
            state,
            mode,
            self.drops.len(),
            self.archie.bag_used(),
            self.archie.bag.capacity(),
            self.truck.storage_used(),
            self.truck.storage_capacity,
        );
        draw_text(&line, 20.0, 30.0, 22.0, ctx.colors.text);

        let hint = if self.archie_in_truck {
            "arrows drive   Space exit   [R] rebuild   [T] tp   [F1] debug   [Esc] title"
        } else {
            "arrows move / aim   A mine   Space jump   push into truck   [F1] debug"
        };
        draw_text(hint, 20.0, vh - 20.0, 16.0, ctx.colors.text_dim);

        if let Some(reason) = self.exit {
            let cy = vh * 0.45;
            draw_rectangle(0.0, cy - 50.0, vw, 100.0, Color::new(0.0, 0.0, 0.0, 0.72));

            let title = match reason {
                LevelExit::FinishLine => "LEVEL COMPLETE",
                LevelExit::ReturnToHq => "HEADED HOME",
            };
            let d = measure_text(title, None, 44, 1.0);
            draw_text(title, vw * 0.5 - d.width * 0.5, cy, 44.0, ctx.colors.gold);

            let sub = "returning to HQ…";
            let d = measure_text(sub, None, 18, 1.0);
            draw_text(
                sub,
                vw * 0.5 - d.width * 0.5,
                cy + 32.0,
                18.0,
                ctx.colors.text_dim,
            );
        }
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

fn archie_frame_name(game: &Game, sprites: &ArchieSprites) -> String {
    let dir = if game.archie.facing.is_left() { "L" } else { "R" };

    if game.mining_this_frame {
        let n = sprites.mine_r.len().max(1);
        let idx = ((game.time * MINE_FPS) as usize) % n;
        return format!("archie_mine_{idx}_{dir}");
    }

    if !game.archie.on_ground {
        let n = sprites.jump_r.len().max(1);
        let t = ((game.archie.vy + JUMP_VY_OFFSET) / JUMP_VY_RANGE).clamp(0.0, 0.999);
        let idx = ((t * n as f32) as usize).min(n - 1);
        return format!("archie_jump_{idx}_{dir}");
    }

    if game.input_horizontal.abs() > 0.01 {
        let n = sprites.walk_r.len().max(1);
        let frame = ((game.time * 8.0) as usize) % n;
        return format!("archie_walk_{frame}_{dir}");
    }

    let n = sprites.idle_r.len().max(1);
    let frame = ((game.time * 2.0) as usize) % n;
    format!("archie_idle_{frame}_{dir}")
}

fn bag_frame_name(archie: &Archie, sprites: &ArchieSprites) -> String {
    let n = sprites.bag_r.len().max(1);
    let level = ((archie.bag.fill_ratio() * (n as f32 - 1.0)).round() as usize).min(n - 1);
    let dir = if archie.facing.is_left() { "L" } else { "R" };
    format!("bag_{level}_{dir}")
}

fn drop_color(r: crate::components::ResourceKind) -> (u8, u8, u8) {
    use crate::components::ResourceKind::*;
    match r {
        Dirt => (0x8B, 0x5A, 0x3A),
        Stone => (0x7A, 0x6A, 0x5A),
        Coal => (0x1E, 0x20, 0x2A),
        Iron => (0xB8, 0x9A, 0x6A),
        Rock => (0x7A, 0x4E, 0x3A),
        Crystal => (0x5F, 0xD4, 0xD4),
    }
}

// =========================================================================
// Tile damage & corner chamfer helpers
// =========================================================================

/// Draw "effritement" chips on a damaged tile. Deterministic per
/// `(col, row, seed)` so the same tile always shows the same pattern.
/// Chip color is a darker shade of the tile, reading as a pitted /
/// crumbling surface.
#[allow(clippy::too_many_arguments)]
fn draw_tile_damage_chips(
    x: f32,
    y: f32,
    s: f32,
    col: usize,
    row: usize,
    seed: u32,
    stage: u8,
    tile_color: (u8, u8, u8),
    scale: f32,
) {
    let chip_count: u32 = match stage {
        1 => 4,
        2 => 10,
        3 => 20,
        _ => return,
    };

    let tile_px = (s / scale).round() as i32;
    if tile_px <= 0 {
        return;
    }

    let mut h = seed
        .wrapping_add(col as u32)
        .wrapping_mul(0x9E37_79B9)
        .wrapping_add(row as u32)
        .wrapping_mul(0x85EB_CA6B);
    h ^= h >> 13;
    h = h.wrapping_mul(0xC2B2_AE35);
    h ^= h >> 16;
    let mut next = |salt: u32| -> u32 {
        h = h.wrapping_mul(1_664_525).wrapping_add(1_013_904_223 + salt);
        (h >> 16) & 0x7FFF
    };

    let (r, g, b) = tile_color;
    let factor = 0.35;
    let chip = Color::new(
        (r as f32 / 255.0) * factor,
        (g as f32 / 255.0) * factor,
        (b as f32 / 255.0) * factor,
        1.0,
    );

    for i in 0..chip_count {
        let edge = next(i * 2) % 4;
        let pos = (next(i * 2 + 1) % tile_px as u32) as i32;
        let (cx, cy) = match edge {
            0 => (pos, 0),
            1 => (pos, tile_px - 1),
            2 => (0, pos),
            3 => (tile_px - 1, pos),
            _ => (0, 0),
        };
        draw_rectangle(
            x + cx as f32 * scale,
            y + cy as f32 * scale,
            scale,
            scale,
            chip,
        );
    }
}

/// Round the corners of a tile wherever two adjacent neighbours are
/// air. Reveals the background color in the corner, breaking the
/// staircase silhouette into a softer slope.
#[allow(clippy::too_many_arguments)]
fn draw_corner_chamfers(
    x: f32,
    y: f32,
    s: f32,
    scale: f32,
    world: &World,
    col: usize,
    row: usize,
    bg: Color,
) {
    let w = world.width();
    let h = world.height();
    let top_air = row == 0 || world.tile(col, row - 1).is_none();
    let bot_air = row + 1 >= h || world.tile(col, row + 1).is_none();
    let left_air = col == 0 || world.tile(col - 1, row).is_none();
    let right_air = col + 1 >= w || world.tile(col + 1, row).is_none();

    // Chamfer size in native px. 2 virtual pixels reads as a subtle
    // bevel without eating the tile.
    let cs = (2.0 * scale).round().max(1.0);
    let xr = x + s;
    let yb = y + s;

    if top_air && left_air {
        draw_triangle(
            Vec2::new(x, y),
            Vec2::new(x + cs, y),
            Vec2::new(x, y + cs),
            bg,
        );
    }
    if top_air && right_air {
        draw_triangle(
            Vec2::new(xr, y),
            Vec2::new(xr - cs, y),
            Vec2::new(xr, y + cs),
            bg,
        );
    }
    if bot_air && left_air {
        draw_triangle(
            Vec2::new(x, yb),
            Vec2::new(x + cs, yb),
            Vec2::new(x, yb - cs),
            bg,
        );
    }
    if bot_air && right_air {
        draw_triangle(
            Vec2::new(xr, yb),
            Vec2::new(xr - cs, yb),
            Vec2::new(xr, yb - cs),
            bg,
        );
    }
}

// =========================================================================
// HQ sign
// =========================================================================

fn draw_hq_sign(
    sx: &impl Fn(f32) -> f32,
    sy: &impl Fn(f32) -> f32,
    scale: f32,
    surface_row: usize,
    ctx: &GameContext,
) {
    let base_wx = 5.0 * TILE_SIZE + TILE_SIZE * 0.5;
    let base_wy = surface_row as f32 * TILE_SIZE;
    let s = scale;

    let post_w = 1.0;
    let post_h = 14.0;
    let post_sx = sx(base_wx) - (post_w * s * 0.5).round();
    let post_sy = sy(base_wy) - (post_h * s).round();
    draw_rectangle(
        post_sx,
        post_sy,
        (post_w * s).max(2.0),
        post_h * s,
        Color::new(0.35, 0.22, 0.13, 1.0),
    );

    let board_w = 14.0;
    let board_h = 8.0;
    let gap = 1.0;
    let board_sx = sx(base_wx) - (board_w * s * 0.5).round();
    let board_sy = post_sy - (board_h * s).round() - (gap * s).round();

    draw_rectangle(
        board_sx,
        board_sy,
        board_w * s,
        board_h * s,
        Color::new(0.55, 0.38, 0.22, 1.0),
    );
    draw_rectangle_lines(
        board_sx,
        board_sy,
        board_w * s,
        board_h * s,
        s.max(1.0),
        Color::new(0.20, 0.12, 0.06, 1.0),
    );

    let arrow = ctx.colors.text;
    let arrow_cy = board_sy + (board_h * s * 0.5).round();
    let arrow_cx = board_sx + (board_w * s * 0.5).round();

    let shaft_w = 6.0 * s;
    let shaft_h = s.max(2.0);
    let tip_size = 2.0 * s;

    let shaft_sx = arrow_cx - shaft_w * 0.5;
    let shaft_sy = arrow_cy - shaft_h * 0.5;
    draw_rectangle(shaft_sx, shaft_sy, shaft_w, shaft_h, arrow);

    let tip_x = shaft_sx;
    draw_triangle(
        Vec2::new(tip_x, arrow_cy),
        Vec2::new(tip_x + tip_size, arrow_cy - tip_size),
        Vec2::new(tip_x + tip_size, arrow_cy + tip_size),
        arrow,
    );
}

// =========================================================================
// Sky gradient and grass
// =========================================================================

/// Vertical sky gradient, drawn one rectangle per **native screen
/// pixel** so the transition is perfectly smooth — no visible
/// banding even at large scales.
#[allow(clippy::too_many_arguments)]
fn draw_sky_gradient(
    screen_oy: f32,
    virt_h: f32,
    surface_wy: f32,
    cam_y: f32,
    scale: f32,
    sky_top: Color,
    sky_bottom: Color,
    below: Color,
) {
    let vw = screen_width();
    let vh = screen_height();

    let surface_screen_y = screen_oy + (surface_wy - cam_y) * scale;
    let sky_end = surface_screen_y.clamp(screen_oy, screen_oy + virt_h);
    let sky_h = sky_end - screen_oy;

    if sky_h > 0.0 {
        // One band per screen pixel — a 400-px tall sky becomes 400
        // rects, all batched into a single draw call by macroquad.
        let bands = (sky_h.ceil() as u32).max(1);
        let step = sky_h / bands as f32;
        for i in 0..bands {
            let t = i as f32 / (bands - 1).max(1) as f32;
            let c = Color::new(
                sky_top.r + (sky_bottom.r - sky_top.r) * t,
                sky_top.g + (sky_bottom.g - sky_top.g) * t,
                sky_top.b + (sky_bottom.b - sky_top.b) * t,
                1.0,
            );
            draw_rectangle(0.0, screen_oy + i as f32 * step, vw, step + 1.0, c);
        }
    }

    let below_y = screen_oy + sky_h;
    if below_y < vh {
        draw_rectangle(0.0, below_y, vw, vh - below_y, below);
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_grass_tufts(
    x: f32,
    y: f32,
    s: f32,
    col: usize,
    row: usize,
    seed: u32,
    color: Color,
    scale: f32,
) {
    let mut h = seed
        .wrapping_add(col as u32)
        .wrapping_mul(0x9E37_79B9)
        .wrapping_add(row as u32)
        .wrapping_mul(0x85EB_CA6B);
    h ^= h >> 13;
    h = h.wrapping_mul(0xC2B2_AE35);
    h ^= h >> 16;

    let mut next = |salt: u32| -> u32 {
        h = h.wrapping_mul(1_664_525).wrapping_add(1_013_904_223 + salt);
        (h >> 16) & 0x7FFF
    };

    let tile_px = (s / scale).round() as i32;
    let mut x_off = 0;
    while x_off < tile_px {
        let tuft_w = 1 + (next(0) % 3) as i32;
        let tuft_h = 2 + (next(1) % 5) as i32;
        draw_rectangle(
            x + x_off as f32 * scale,
            y,
            tuft_w as f32 * scale,
            tuft_h as f32 * scale,
            color,
        );
        x_off += tuft_w;
    }
}

/// Small red triangle above the truck, pointing toward the blocked
/// direction. `Wall` and `HoleAhead` point left/right; `LowCeiling`
/// and `SteepStep` point up.
#[allow(clippy::too_many_arguments)]
fn draw_block_arrow(
    facing_right: bool,
    reason: crate::truck::BlockReason,
    tx: f32,
    ty: f32,
    tw: f32,
    _th: f32,
    scale: f32,
    color: Color,
) {
    use crate::truck::BlockReason;
    // Center of the truck's top edge.
    let cx = tx + tw * 0.5;
    let cy = ty - 6.0 * scale;
    let r = 4.0 * scale;

    match reason {
        BlockReason::Wall | BlockReason::HoleAhead => {
            // Points in the direction the truck is trying to move.
            let s = if facing_right { 1.0 } else { -1.0 };
            draw_triangle(
                Vec2::new(cx + s * r, cy),
                Vec2::new(cx - s * r * 0.5, cy - r),
                Vec2::new(cx - s * r * 0.5, cy + r),
                color,
            );
        }
        BlockReason::LowCeiling | BlockReason::SteepStep => {
            // Points up.
            draw_triangle(
                Vec2::new(cx, cy - r),
                Vec2::new(cx - r, cy + r * 0.5),
                Vec2::new(cx + r, cy + r * 0.5),
                color,
            );
        }
    }
}

// =========================================================================
// Pixel-perfect rendering helpers
// =========================================================================

pub const VIRTUAL_W: f32 = 320.0;
pub const VIRTUAL_H: f32 = 180.0;

const CAMERA_OVERSCROLL_X: f32 = 80.0;
const CAMERA_OVERSCROLL_Y: f32 = 40.0;

pub(crate) fn integer_scale(vw: f32, vh: f32) -> f32 {
    let s = (vw / VIRTUAL_W).min(vh / VIRTUAL_H).floor();
    s.max(1.0)
}

pub(crate) fn clamp_camera(target: f32, viewport: f32, world: f32, overscroll: f32) -> f32 {
    let min = -overscroll;
    let max = (world - viewport).max(0.0) + overscroll;
    target.clamp(min, max)
}