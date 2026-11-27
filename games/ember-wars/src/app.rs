//! The campaign: menu (with the upgrade modal), progress and saving.
//! Was `main.rs`'s loop. A level is a `World` node (`world.rs`); the
//! root (`scene.rs`) spawns it from `App::update` and hands its signals
//! back here.

use crate::config::GameContext;
use crate::menu::{self, AppScreen, MenuState};
use crate::progress::{Progress, stars_for_victory};
use crate::systems::Game;
use crate::ui::upgrade_modal;
use crate::world::LevelFinished;
use ember::prelude::*;

/// Where progress goes: the game's `progress::Store` when playing,
/// something else in tests.
pub type Saver = Box<dyn FnMut(&Progress) + Send + Sync>;

/// What a menu frame asks of the root.
pub enum MenuOutcome {
    Stay,
    Quit,
    /// A level was picked; the root turns it into a `World`.
    Play(Box<Game>),
}

pub struct App {
    pub screen: AppScreen,
    pub menu_state: MenuState,
    pub progress: Progress,
    save: Saver,
    /// Level seeds (the time, as before; fixed in tests).
    seed: fn() -> u32,
}

impl App {
    pub fn new(progress: Progress, save: Saver, seed: fn() -> u32) -> Self {
        Self {
            screen: AppScreen::Menu,
            menu_state: MenuState::new(),
            progress,
            save,
            seed,
        }
    }

    /// One menu frame, in the old loop's order. The level's own frames
    /// run in `World`.
    pub fn update(&mut self, keys: &Input, ctx: &GameContext) -> MenuOutcome {
        let mut outcome = MenuOutcome::Stay;
        if self.screen == AppScreen::Menu
            && let Some(game) = self.update_menu(keys, ctx)
        {
            outcome = MenuOutcome::Play(Box::new(game));
        }

        if keys.is_key_pressed(Key::Escape) && self.screen == AppScreen::Menu {
            if self.menu_state.upgrade_modal_open {
                self.menu_state.upgrade_modal_open = false;
            } else {
                return MenuOutcome::Quit;
            }
        }
        outcome
    }

    /// A fresh battle on `level_id` with the current upgrades.
    pub fn new_game(&self, ctx: &GameContext, level_id: &str) -> Result<Game, String> {
        Game::new(ctx, level_id, (self.seed)(), &self.progress)
    }

    fn update_menu(&mut self, keys: &Input, ctx: &GameContext) -> Option<Game> {
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
            match self.new_game(ctx, &level_id) {
                Ok(g) => {
                    self.screen = AppScreen::Game;
                    return Some(g);
                }
                Err(e) => {
                    eprintln!("Game::new failed: {e}");
                }
            }
        }
        None
    }

    /// Bank the battle's gold and reward and the stars, and save.
    pub fn finish_level(&mut self, ev: &LevelFinished) {
        self.progress.tree.gold = ev.gold + ev.reward;
        if ev.won {
            let stars = stars_for_victory(ev.tower_hp_fraction);
            let prev = self.progress.stars_for(&ev.level_id);
            if stars > prev {
                self.progress.stars.insert(ev.level_id.clone(), stars);
            }
        }
        (self.save)(&self.progress);
    }

    /// The menu frame, drawn as the old loop drew it (a level draws
    /// itself, as `World`).
    pub fn draw(&self, gfx: &mut dyn RenderingServer, ctx: &GameContext) {
        gfx.set_clear_color(ctx.colors.bg);
        if self.screen == AppScreen::Menu {
            menu::draw_menu(gfx, ctx, &self.menu_state, &self.progress);
            if self.menu_state.upgrade_modal_open {
                upgrade_modal::draw_modal(gfx, ctx, &self.progress.tree, &self.menu_state);
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
