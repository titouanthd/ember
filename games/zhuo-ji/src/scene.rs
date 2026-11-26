//! Zhuo Ji as a node tree: one root node, `ZhuoJi`, mouse plus a few
//! keys. It runs the title menu, the rules overlay and the match
//! (`Game`), and draws the table through the `RenderingServer`.
//!
//! The table lays itself out on the window size (`TableLayout`), so the
//! loop runs with `with_viewport_following_window` and the node reads the
//! size from the viewport; there is no fixed canvas.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::help::{self, HelpAction, HelpState};
use crate::layout::TableLayout;
use crate::menu::{self, MenuAction, MenuState};
use crate::table::{self, ClaimAction, MatchEndTab, OwnAction};
use crate::{Game, GameContext, Phase, TileFont};
use ember::prelude::*;

/// The window the game opens with.
pub const WINDOW_W: f32 = 1440.0;
pub const WINDOW_H: f32 = 900.0;

/// Frame time cap, as before.
const MAX_FRAME_DT: f32 = 1.0 / 30.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppScreen {
    Menu,
    Game,
}

pub fn fresh_seed() -> u32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| (d.as_nanos() ^ (d.as_nanos() >> 32)) as u32)
        .unwrap_or(0x2E5B_1234)
}

pub struct ZhuoJi {
    name: String,
    pub ctx: GameContext,
    pub game: Game,
    pub help_state: HelpState,
    pub menu_state: MenuState,
    pub screen: AppScreen,
    pub match_end_tab: MatchEndTab,
    match_over_time: f32,
    last_was_match_over: bool,
    hand_end_time: f32,
    last_was_hand_end: bool,
    /// Seconds since start, for the menu's pulse (was `get_time`).
    pub time: f32,
    /// The window size the last frame was drawn at.
    size: Vec2,
    /// The pointer, saved in `process` for the hover effects in `draw`.
    mouse: Vec2,
    /// Seed of the next match; `fresh_seed` unless a test fixed it.
    seed: Option<u32>,
}

impl ZhuoJi {
    pub fn new(ctx: GameContext, help_state: HelpState, seed: Option<u32>) -> Self {
        Self {
            name: "ZhuoJi".into(),
            ctx,
            game: Game::new(seed.unwrap_or_else(fresh_seed)),
            help_state,
            menu_state: MenuState::new(),
            screen: AppScreen::Menu,
            match_end_tab: MatchEndTab::Scores,
            match_over_time: 0.0,
            last_was_match_over: false,
            hand_end_time: 0.0,
            last_was_hand_end: false,
            time: 0.0,
            size: Vec2::new(WINDOW_W, WINDOW_H),
            mouse: Vec2::ZERO,
            seed,
        }
    }

    pub fn layout(&self) -> TableLayout {
        TableLayout::new(self.size.x, self.size.y)
    }

    fn new_game(&mut self) {
        self.game = Game::new(self.seed.unwrap_or_else(fresh_seed));
        self.match_end_tab = MatchEndTab::Scores;
        self.match_over_time = 0.0;
        self.last_was_match_over = false;
        self.hand_end_time = 0.0;
        self.last_was_hand_end = false;
        self.screen = AppScreen::Game;
    }

    /// Runs a menu action; `false` means quit.
    fn on_menu(&mut self, action: MenuAction) -> bool {
        match action {
            MenuAction::NewGame => self.new_game(),
            MenuAction::Rules => self.help_state.open(),
            MenuAction::Quit => return false,
            MenuAction::None => {}
        }
        true
    }

    /// One frame of the old loop, up to (not including) the drawing.
    /// Returns `false` to quit.
    fn step(&mut self, input: &Input, dt: f32) -> bool {
        let layout = self.layout();

        // ─── Keyboard ─────────────────────────────────────────────────
        if input.is_key_pressed(Key::Escape) {
            if self.help_state.visible {
                self.help_state.close();
            } else if self.screen == AppScreen::Menu {
                return false;
            } else {
                self.screen = AppScreen::Menu;
            }
        }

        if input.is_key_pressed(Key::H) {
            self.help_state.toggle();
        }

        if self.help_state.visible {
            if input.is_key_pressed(Key::Left) {
                self.help_state.prev_tab();
            }
            if input.is_key_pressed(Key::Right) {
                self.help_state.next_tab();
            }
        } else if self.screen == AppScreen::Menu {
            if input.is_key_pressed(Key::Up) {
                self.menu_state.up();
            }
            if input.is_key_pressed(Key::Down) {
                self.menu_state.down();
            }
            if (input.is_key_pressed(Key::Enter) || input.is_key_pressed(Key::Space))
                && !self.on_menu(self.menu_state.action())
            {
                return false;
            }
        }

        // ─── Screen-specific timers ───────────────────────────────────
        if self.screen == AppScreen::Game {
            let is_mo = matches!(self.game.phase, Phase::MatchOver { .. });
            if is_mo && !self.last_was_match_over {
                self.match_over_time = 0.0;
                self.match_end_tab = MatchEndTab::Scores;
            } else if is_mo {
                self.match_over_time += dt;
            }
            self.last_was_match_over = is_mo;

            let is_hand_end = matches!(self.game.phase, Phase::Hu { .. } | Phase::HuangZhuang);
            if is_hand_end && !self.last_was_hand_end {
                self.hand_end_time = 0.0;
            } else if is_hand_end {
                self.hand_end_time += dt;
            }
            self.last_was_hand_end = is_hand_end;
        }

        // ─── Mouse ────────────────────────────────────────────────────
        let mouse = input.mouse_pos;
        if input.mouse_left_pressed {
            if self.help_state.visible {
                match help::hit_test(&layout, &self.help_state, mouse) {
                    HelpAction::Close | HelpAction::GotIt => self.help_state.close(),
                    HelpAction::Tab(t) => self.help_state.tab = t,
                    HelpAction::SuitFilter(s) => self.help_state.suit_filter = s,
                    HelpAction::RankFilter(r) => self.help_state.rank_filter = r,
                    HelpAction::None => {}
                }
            } else if self.screen == AppScreen::Menu {
                if !self.on_menu(menu::hit_test(&layout, mouse)) {
                    return false;
                }
            } else {
                let ctx = &self.ctx;
                let game = &mut self.game;
                let is_mo = matches!(game.phase, Phase::MatchOver { .. });
                let is_hand_end = matches!(game.phase, Phase::Hu { .. } | Phase::HuangZhuang);
                if is_mo {
                    if let Some(tab) = table::hit_match_end_tab(&layout, mouse) {
                        self.match_end_tab = tab;
                    }
                } else if !is_hand_end {
                    if let Some(action) = table::hit_claim_button(game, &layout, mouse) {
                        match action {
                            ClaimAction::Claim(kind) => {
                                game.human_claim(kind);
                            }
                            ClaimAction::Pass => {
                                game.human_pass(ctx);
                            }
                        }
                    } else if let Some(action) = table::hit_own_turn_button(game, &layout, mouse) {
                        match action {
                            OwnAction::Zimo => {
                                game.human_zimo();
                            }
                            OwnAction::AnGang(t) => {
                                game.human_an_gang(t);
                            }
                        }
                    } else if let Some(idx) = table::hit_hand_tile(game, &layout, mouse) {
                        game.human_discard(idx, ctx);
                    }
                }
            }
        }

        // Menu hover updates selection (when menu is active, help closed).
        if self.screen == AppScreen::Menu
            && !self.help_state.visible
            && let Some(idx) = menu::hit_button_idx(&layout, mouse)
        {
            self.menu_state.selected = idx;
        }

        // ─── Game update ──────────────────────────────────────────────
        if self.screen == AppScreen::Game {
            let game_dt = if self.help_state.visible { 0.0 } else { dt };
            let _ = self.game.update(input, &self.ctx, game_dt);
        }
        true
    }
}

impl Object for ZhuoJi {
    fn class_name() -> &'static str {
        "ZhuoJi"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl NodeName for ZhuoJi {
    fn name(&self) -> &str {
        &self.name
    }
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

impl Node for ZhuoJi {
    fn ready(&mut self, ctx: &mut NodeContext) {
        self.ctx.font = TileFont::load(ctx.render());
    }

    fn process(&mut self, dt: f32, ctx: &mut NodeContext) {
        self.time += dt;
        let dt = dt.min(MAX_FRAME_DT);
        // The last frame's viewport is the window (zero before the first
        // frame, when the opening size stands).
        let viewport = ctx.render().viewport();
        if viewport.w > 0.0 && viewport.h > 0.0 {
            self.size = Vec2::new(viewport.w, viewport.h);
        }
        let input = ctx.input().clone();
        self.mouse = input.mouse_pos;
        if !self.step(&input, dt) {
            ctx.quit();
        }
    }

    fn draw(&self, ctx: &mut DrawContext) {
        let layout = self.layout();
        let gfx = ctx.render();
        match self.screen {
            AppScreen::Menu => {
                table::draw_background(gfx, &layout, &self.ctx);
                menu::draw_menu(
                    gfx,
                    &layout,
                    &self.ctx,
                    &self.menu_state,
                    self.mouse,
                    self.time,
                );
            }
            AppScreen::Game => table::draw_game(
                gfx,
                &self.game,
                &layout,
                &self.ctx,
                self.mouse,
                &self.help_state,
                self.match_end_tab,
                self.match_over_time,
                self.hand_end_time,
            ),
        }

        table::draw_premium_border(gfx, &layout, &self.ctx);

        if self.help_state.visible {
            help::draw_help_overlay(gfx, &layout, &self.ctx, &self.help_state, self.mouse);
        }
    }
}

/// The whole game: a single `ZhuoJi` root.
pub fn build_tree(ctx: GameContext, help_state: HelpState, seed: Option<u32>) -> NodeTree {
    NodeTree::with_root(ZhuoJi::new(ctx, help_state, seed))
}

/// Opens the window (resizable; the table follows its size) and plays.
pub fn run() {
    let game = SceneLoop::new(
        build_tree(GameContext::load(), HelpState::new(), None),
        MacroquadRenderer::new(),
        MacroquadAudio::new(),
        Rect::new(0.0, 0.0, WINDOW_W, WINDOW_H),
    )
    .with_viewport_following_window();
    run_window(
        "Zhuo Ji — 贵阳捉鸡麻将",
        WINDOW_W as i32,
        WINDOW_H as i32,
        game,
    );
}
