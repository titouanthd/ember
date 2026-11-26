//! The app: menu (with the upgrade modal) and the level. Was `main.rs`'s
//! loop; `scene.rs` runs it as a node. `App::update` is the loop's logic
//! in its original order and `App::draw` its drawing.

use crate::catapult::ShotKind;
use crate::config::GameContext;
use crate::frame::mouse_position;
use crate::menu::{self, AppScreen, MenuState};
use crate::progress::{Progress, stars_for_victory};
use crate::render;
use crate::systems::{Game, Phase};
use crate::ui::spawn_bar::SpawnBar;
use crate::ui::upgrade_modal;
use ember::prelude::*;

/// Where progress goes: the game's `progress::Store` when playing,
/// something else in tests.
pub type Saver = Box<dyn FnMut(&Progress) + Send + Sync>;

pub struct App {
    pub screen: AppScreen,
    pub menu_state: MenuState,
    pub game: Option<Game>,
    pub progress: Progress,
    spawn_bar: SpawnBar,
    last_level_id: Option<String>,
    level_resolved: bool,
    /// This frame's screen-shake offset, taken once per frame in
    /// `update` (it advances the shake's seed).
    shake: Vec2,
    save: Saver,
    /// Level seeds (the time, as before; fixed in tests).
    seed: fn() -> u32,
}

impl App {
    pub fn new(progress: Progress, save: Saver, seed: fn() -> u32) -> Self {
        Self {
            screen: AppScreen::Menu,
            menu_state: MenuState::new(),
            game: None,
            progress,
            spawn_bar: SpawnBar::new(),
            last_level_id: None,
            level_resolved: false,
            shake: Vec2::ZERO,
            save,
            seed,
        }
    }

    /// One frame of the old loop's logic. Returns `false` to quit.
    pub fn update(&mut self, keys: &Input, ctx: &GameContext, dt: f32) -> bool {
        match self.screen {
            AppScreen::Menu => self.update_menu(keys, ctx),
            AppScreen::Game => self.update_game(keys, ctx, dt),
        }

        if keys.is_key_pressed(Key::Escape) {
            match self.screen {
                AppScreen::Menu => {
                    if self.menu_state.upgrade_modal_open {
                        self.menu_state.upgrade_modal_open = false;
                    } else {
                        return false;
                    }
                }
                AppScreen::Game => {
                    self.screen = AppScreen::Menu;
                    self.game = None;
                }
            }
        }

        if keys.is_key_pressed(Key::R)
            && self.screen == AppScreen::Game
            && let Some(level_id) = self.last_level_id.as_ref()
            && let Ok(new_game) = Game::new(ctx, level_id, (self.seed)(), &self.progress)
        {
            self.game = Some(new_game);
            self.level_resolved = false;
        }

        if keys.is_key_pressed(Key::Enter)
            && self.screen == AppScreen::Game
            && let Some(g) = self.game.as_ref()
            && g.phase != Phase::Playing
        {
            self.screen = AppScreen::Menu;
            self.game = None;
        }

        if self.screen == AppScreen::Game
            && let Some(g) = self.game.as_mut()
        {
            if keys.is_key_pressed(Key::Num1) {
                g.catapult.shot_kind = ShotKind::Basic;
            }
            if keys.is_key_pressed(Key::Num2) {
                g.catapult.shot_kind = ShotKind::Piercing;
            }
            if keys.is_key_pressed(Key::Num3) {
                g.catapult.shot_kind = ShotKind::Explosive;
            }
        }

        // The shake offset advances its seed, so it is taken once here
        // (it was taken while drawing).
        self.shake = match (self.screen, self.game.as_mut()) {
            (AppScreen::Game, Some(g)) => g.juice.screen_shake.offset(),
            _ => Vec2::ZERO,
        };
        true
    }

    fn update_menu(&mut self, keys: &Input, ctx: &GameContext) {
        // Tab : toggle la modal d'upgrades dans le menu.
        if keys.is_key_pressed(Key::Tab) {
            self.menu_state.upgrade_modal_open = !self.menu_state.upgrade_modal_open;
            if self.menu_state.upgrade_modal_open && self.menu_state.upgrade_focus.is_none() {
                self.menu_state.upgrade_focus = Some(0);
            }
        }
        if self.menu_state.upgrade_modal_open && keys.is_key_pressed(Key::Escape) {
            self.menu_state.upgrade_modal_open = false;
        }

        if self.menu_state.upgrade_modal_open {
            let bought = upgrade_modal::handle_modal_input(
                keys,
                &mut self.progress.tree,
                &mut self.menu_state,
            );
            if bought {
                (self.save)(&self.progress);
            }
        } else if let Some(level_id) =
            menu::handle_menu_input(keys, &mut self.menu_state, &self.progress)
        {
            match Game::new(ctx, &level_id, (self.seed)(), &self.progress) {
                Ok(g) => {
                    self.last_level_id = Some(level_id);
                    self.level_resolved = false;
                    self.game = Some(g);
                    self.screen = AppScreen::Game;
                }
                Err(e) => {
                    eprintln!("Game::new failed: {e}");
                }
            }
        }
    }

    fn update_game(&mut self, keys: &Input, ctx: &GameContext, dt: f32) {
        let Some(g) = self.game.as_mut() else {
            return;
        };
        g.camera.viewport_w = ctx.viewport_w;

        if g.phase == Phase::Playing {
            let mut cam_dir = 0.0;
            if keys.is_key_down(Key::Left) {
                cam_dir -= 1.0;
            }
            if keys.is_key_down(Key::Right) {
                cam_dir += 1.0;
            }

            let mouse_screen = Vec2::new(mouse_position().0, mouse_position().1);
            let mouse_world = Vec2::new(g.camera.screen_to_world(mouse_screen.x), mouse_screen.y);

            let mut consumed = false;
            if keys.mouse_left_pressed
                && let Some(kind) = self.spawn_bar.handle_click(keys, g)
            {
                g.try_player_spawn(&kind);
                consumed = true;
            }

            let fire_click = keys.mouse_left_pressed && !consumed;
            g.tick(dt, mouse_world, fire_click, cam_dir);
        }

        // Résolution de fin de niveau (une seule fois).
        if g.phase != Phase::Playing && !self.level_resolved {
            self.level_resolved = true;
            let level_id = g.level.id.clone();
            let reward = if g.phase == Phase::Won {
                g.level.reward_gold
            } else {
                0.0
            };
            // Sync bank (gold total) + reward + stars.
            self.progress.tree.gold = g.player_upgrades.gold + reward;
            if g.phase == Phase::Won {
                let stars = stars_for_victory(g.tower_hp_fraction());
                let prev = self.progress.stars_for(&level_id);
                if stars > prev {
                    self.progress.stars.insert(level_id, stars);
                }
            }
            (self.save)(&self.progress);
        }
    }

    /// The frame, drawn as the old loop drew it.
    pub fn draw(&self, gfx: &mut dyn RenderingServer, ctx: &GameContext) {
        gfx.set_clear_color(ctx.colors.bg);
        match self.screen {
            AppScreen::Menu => {
                menu::draw_menu(gfx, ctx, &self.menu_state, &self.progress);
                if self.menu_state.upgrade_modal_open {
                    upgrade_modal::draw_modal(gfx, ctx, &self.progress.tree, &self.menu_state);
                }
            }
            AppScreen::Game => {
                if let Some(g) = self.game.as_ref() {
                    render::draw_game(gfx, ctx, g, self.shake);
                    self.spawn_bar.draw(gfx, ctx, g);
                    render::draw_end_overlay(gfx, ctx, g);
                }
            }
        }
    }
}

/// A level seed from the clock, as before.
pub fn seed_from_time() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as u32 ^ d.subsec_nanos())
        .unwrap_or(0xC0FFEE)
}
