//! Game state machine and phase transitions.
//!
//! Design refs: DESIGN.md §3 (boucle de jeu), §7.5 (sub-steps), §10 (architecture).
//!
//! `Game` owns all mutable state. `Game::update` returns a `Vec<GameEvent>`
//! that `main.rs` translates into audio; the state machine itself is fully
//! self-contained (events never need to be fed back in).

use ember_core::rng::Rng;
use ember_stdlib::input::Input;
use glam::Vec2;
use macroquad::prelude::KeyCode;

use crate::components::{Paddle, Puck, Side, Trail};
use crate::config::GameContext;
use crate::effects::{Hitstop, Particles, Shake};
use crate::physics::{step_world_substepped, CollisionEvent};
use crate::ai::AiState;

/// High-level phase of a game.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Phase {
    Menu,
    Countdown { t: f32 },
    Playing,
    GoalPause { t: f32, scorer: Side },
    RoundOver { t: f32, winner: Side },
    MatchOver { winner: Side },
}

/// Who controls Player 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GameMode {
    /// Two human players, split keyboard.
    #[default]
    Pvp,
    /// Player 1 is human, Player 2 is the AI.
    PvAi,
}

/// Events emitted by one `Game::update`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GameEvent {
    Wall,
    Paddle { impact_speed: f32 },
    Goal(Side),
    CountdownBeep,
    CountdownGo,
    RoundWon(Side),
    MatchWon(Side),
}

/// Goals scored in the current round.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Score {
    pub p1: u32,
    pub p2: u32,
}

impl Score {
    #[inline]
    pub fn get(&self, side: Side) -> u32 {
        match side {
            Side::Left => self.p1,
            Side::Right => self.p2,
        }
    }

    #[inline]
    pub fn add(&mut self, side: Side) {
        match side {
            Side::Left => self.p1 += 1,
            Side::Right => self.p2 += 1,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.p1 = 0;
        self.p2 = 0;
    }
}

/// Rounds won in the current match (best of 3, first to 2).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RoundsWon {
    pub p1: u32,
    pub p2: u32,
}

impl RoundsWon {
    #[inline]
    pub fn get(&self, side: Side) -> u32 {
        match side {
            Side::Left => self.p1,
            Side::Right => self.p2,
        }
    }

    #[inline]
    pub fn add(&mut self, side: Side) {
        match side {
            Side::Left => self.p1 += 1,
            Side::Right => self.p2 += 1,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.p1 = 0;
        self.p2 = 0;
    }
}

/// Duration of the visual goal flash (seconds).
const GOAL_FLASH_TIME: f32 = 0.2;

/// Freeze between a round ending and the next countdown (seconds).
const ROUND_OVER_TIME: f32 = 1.5;

/// Impact speed above which a paddle hit triggers hitstop.
const HARD_HIT_SPEED: f32 = 900.0;

// Player 1 controls. We map **both** the QWERTY position and the AZERTY
// physical key, so the game is playable on either layout without a setting.
// `W` (QWERTY pos) and `Z` (AZERTY label) share position 1,2 — see DESIGN §4.
const P1_UP: &[KeyCode] = &[KeyCode::W, KeyCode::Z];
const P1_DOWN: &[KeyCode] = &[KeyCode::S];
const P1_LEFT: &[KeyCode] = &[KeyCode::A, KeyCode::Q];
const P1_RIGHT: &[KeyCode] = &[KeyCode::D];

// Player 2 uses arrows, which are layout-stable.
const P2_UP: &[KeyCode] = &[KeyCode::Up];
const P2_DOWN: &[KeyCode] = &[KeyCode::Down];
const P2_LEFT: &[KeyCode] = &[KeyCode::Left];
const P2_RIGHT: &[KeyCode] = &[KeyCode::Right];

/// Full game state.
pub struct Game {
    pub phase: Phase,
    pub paused: bool,
    pub mode: GameMode,
    pub ai: AiState, 
    pub paddles: [Paddle; 2],
    pub puck: Puck,
    pub trail: Trail,
    pub score: Score,
    pub rounds: RoundsWon,
    pub shake: Shake,
    pub hitstop: Hitstop,
    pub particles: Particles,
    /// Goal flash timer, indexed `[left_goal, right_goal]`.
    pub goal_flash: [f32; 2],
    /// Used for particle bursts. `Shake` has its own RNG.
    pub rng: Rng,
    /// Cached per-frame offset for the renderer.
    pub shake_offset: Vec2,
}

impl Game {
    pub fn new(ctx: &GameContext) -> Self {
        let a = &ctx.arena;
        let r = 28.0_f32;
        let p1 = Paddle::new(Vec2::new(a.w * 0.25, a.mid_y()), r, Side::Left);
        let p2 = Paddle::new(Vec2::new(a.w * 0.75, a.mid_y()), r, Side::Right);
        let puck = Puck::new(Vec2::new(a.mid_x(), a.mid_y()), 14.0);
        Self {
            phase: Phase::Menu,
            paused: false,
            mode: GameMode::default(),
            ai: AiState::new(),
            paddles: [p1, p2],
            puck,
            trail: Trail::new(),
            score: Score::default(),
            rounds: RoundsWon::default(),
            shake: Shake::new(0xCAFE_BABE),
            hitstop: Hitstop::new(),
            particles: Particles::new(),
            goal_flash: [0.0, 0.0],
            rng: Rng::new(0xBEEF_1234),
            shake_offset: Vec2::ZERO,
        }
    }

    /// Advance the game by `dt` seconds and return the events of the frame.
    pub fn update(
        &mut self,
        input: &Input,
        ctx: &GameContext,
        dt: f32,
    ) -> Vec<GameEvent> {
        // 1. Effects that always run, even when paused or during hitstop.
        self.shake.update(dt);
        self.hitstop.update(dt);
        self.particles.update(dt);
        self.trail.update(dt);
        for f in self.goal_flash.iter_mut() {
            *f = (*f - dt).max(0.0);
        }
        self.shake_offset = self.shake.current();

        // 2. Pause toggle. Only meaningful in Playing.
        if matches!(self.phase, Phase::Playing) && input.is_key_pressed(KeyCode::P) {
            self.paused = !self.paused;
        }

        // 3. Effective dt for physics and phase timers.
        let dt_eff = if self.paused || self.hitstop.active() {
            0.0
        } else {
            dt
        };

        // 4. Phase dispatch.
        let mut events = Vec::new();
        let phase = self.phase;
        match phase {
            Phase::Menu => {
                if input.is_key_pressed(KeyCode::Space) {
                    self.mode = GameMode::Pvp;
                    self.enter_countdown(ctx);
                    events.push(GameEvent::CountdownBeep);
                } else if input.is_key_pressed(KeyCode::Enter) {
                    self.mode = GameMode::PvAi;
                    self.enter_countdown(ctx);
                    events.push(GameEvent::CountdownBeep);
                }
            }

            Phase::Countdown { t } => {
                let new_t = t - dt_eff;
                if new_t <= 0.0 {
                    self.phase = Phase::Playing;
                    events.push(GameEvent::CountdownGo);
                } else {
                    if new_t.ceil() < t.ceil() {
                        events.push(GameEvent::CountdownBeep);
                    }
                    self.phase = Phase::Countdown { t: new_t };
                }
            }

            Phase::Playing => {
                if dt_eff > 0.0 {
                    let mut inputs = gather_paddles(input);
                    if self.mode == GameMode::PvAi {
                        // Split borrows so the immutable refs to paddles/puck/arena and the
                        // mutable ref to self.ai can coexist.
                        let pad = &self.paddles[1];
                        let puck = &self.puck;
                        let arena = &ctx.arena;
                        let tuning = &ctx.tuning;
                        inputs[1] = crate::ai::direction(pad, puck, arena, tuning, &mut self.ai, dt_eff);
                    }

                    let physics_events = step_world_substepped(
                        &mut self.paddles,
                        &mut self.puck,
                        inputs,
                        ctx,
                        dt_eff,
                    );
                    for ev in physics_events {
                        match ev {
                            CollisionEvent::Wall => events.push(GameEvent::Wall),
                            CollisionEvent::Paddle { impact_speed } => {
                                if impact_speed > HARD_HIT_SPEED {
                                    self.hitstop.trigger(ctx.tuning.hitstop);
                                }
                                events.push(GameEvent::Paddle { impact_speed });
                            }
                            CollisionEvent::Goal(scorer) => {
                                self.on_goal(scorer, ctx, &mut events);
                            }
                        }
                    }
                    self.trail.push(self.puck.pos);
                }
            }

            Phase::GoalPause { t, scorer } => {
                let new_t = t - dt_eff;
                if new_t <= 0.0 {
                    let g1 = self.score.p1 >= ctx.tuning.goals_to_win_round;
                    let g2 = self.score.p2 >= ctx.tuning.goals_to_win_round;
                    if g1 || g2 {
                        let winner = if self.score.p1 > self.score.p2 {
                            Side::Left
                        } else {
                            Side::Right
                        };
                        self.rounds.add(winner);
                        events.push(GameEvent::RoundWon(winner));
                        if self.rounds.get(winner) >= ctx.tuning.rounds_to_win_match {
                            self.phase = Phase::MatchOver { winner };
                            events.push(GameEvent::MatchWon(winner));
                        } else {
                            self.phase = Phase::RoundOver { t: ROUND_OVER_TIME, winner };
                        }
                    } else {
                        self.enter_countdown(ctx);
                    }
                } else {
                    self.phase = Phase::GoalPause { t: new_t, scorer };
                }
            }

            Phase::RoundOver { t, winner } => {
                let new_t = t - dt_eff;
                if new_t <= 0.0 {
                    self.score.reset();
                    self.enter_countdown(ctx);
                } else {
                    self.phase = Phase::RoundOver { t: new_t, winner };
                }
            }

            Phase::MatchOver { winner } => {
                if input.is_key_pressed(KeyCode::Space) {
                    self.reset_match(ctx);
                    self.enter_countdown(ctx);
                    events.push(GameEvent::CountdownBeep);
                }
                let _ = winner;
            }
        }

        events
    }

    /// Transition into a countdown. Always resets positions and unpauses.
    fn enter_countdown(&mut self, ctx: &GameContext) {
        self.paused = false;
        self.ai.reset();
        self.reset_positions(ctx);
        self.phase = Phase::Countdown { t: ctx.tuning.countdown };
    }

    /// Reset paddle and puck positions to their starting spots.
    fn reset_positions(&mut self, ctx: &GameContext) {
        let a = &ctx.arena;
        let y = a.mid_y();
        self.paddles[0].pos = Vec2::new(a.w * 0.25, y);
        self.paddles[0].vel = Vec2::ZERO;
        self.paddles[1].pos = Vec2::new(a.w * 0.75, y);
        self.paddles[1].vel = Vec2::ZERO;
        self.puck.pos = Vec2::new(a.mid_x(), y);
        self.puck.vel = Vec2::ZERO;
        self.puck.spin = 0.0;
        self.trail.clear();
    }

    /// Reset everything for a fresh match.
    fn reset_match(&mut self, ctx: &GameContext) {
        self.score.reset();
        self.rounds.reset();
        self.reset_positions(ctx);
    }

    /// Handle a goal: bump score, flash, shake, particles, change phase.
    fn on_goal(&mut self, scorer: Side, ctx: &GameContext, events: &mut Vec<GameEvent>) {
        self.score.add(scorer);
        // Flash the goal that was scored into, not the scorer's own goal.
        let goal_idx = match scorer {
            Side::Left => 1,
            Side::Right => 0,
        };
        self.goal_flash[goal_idx] = GOAL_FLASH_TIME;
        self.shake
            .trigger(ctx.tuning.shake_amplitude, ctx.tuning.shake_duration);
        let origin = match goal_idx {
            0 => Vec2::new(ctx.arena.wall, ctx.arena.mid_y()),
            _ => Vec2::new(ctx.arena.w - ctx.arena.wall, ctx.arena.mid_y()),
        };
        self.particles.burst(origin, 24, 400.0, &mut self.rng);
        events.push(GameEvent::Goal(scorer));
        self.phase = Phase::GoalPause {
            t: ctx.tuning.goal_pause,
            scorer,
        };
    }
}

/// Combine directional keys into a normalized-ish direction vector.
fn gather_dir(
    input: &Input,
    up: &[KeyCode],
    down: &[KeyCode],
    left: &[KeyCode],
    right: &[KeyCode],
) -> Vec2 {
    let mut d = Vec2::ZERO;
    if up.iter().any(|k| input.is_key_down(*k)) {
        d.y -= 1.0;
    }
    if down.iter().any(|k| input.is_key_down(*k)) {
        d.y += 1.0;
    }
    if left.iter().any(|k| input.is_key_down(*k)) {
        d.x -= 1.0;
    }
    if right.iter().any(|k| input.is_key_down(*k)) {
        d.x += 1.0;
    }
    d
}

/// Gather both players' movement inputs, in `[p1, p2]` order.
pub fn gather_paddles(input: &Input) -> [Vec2; 2] {
    [
        gather_dir(input, P1_UP, P1_DOWN, P1_LEFT, P1_RIGHT),
        gather_dir(input, P2_UP, P2_DOWN, P2_LEFT, P2_RIGHT),
    ]
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GameContext;
    use macroquad::prelude::KeyCode;

    fn ctx() -> GameContext {
        GameContext::default_hermetic()
    }

    fn empty_input() -> Input {
        Input {
            mouse_pos: Vec2::ZERO,
            mouse_left_pressed: false,
            mouse_left_down: false,
            mouse_left_released: false,
            mouse_right_pressed: false,
            mouse_right_down: false,
            mouse_right_released: false,
            mouse_middle_pressed: false,
            mouse_middle_down: false,
            mouse_middle_released: false,
            keys_pressed: Vec::new(),
            keys_down: Vec::new(),
            keys_released: Vec::new(),
        }
    }

    fn input_with(pressed: Vec<KeyCode>, down: Vec<KeyCode>) -> Input {
        let mut i = empty_input();
        i.keys_pressed = pressed;
        i.keys_down = down;
        i
    }

    fn input_space() -> Input {
        input_with(vec![KeyCode::Space], vec![])
    }

    fn input_p() -> Input {
        input_with(vec![KeyCode::P], vec![])
    }

    /// Build a game that is already in `Playing`.
    fn game_in_playing(ctx: &GameContext) -> Game {
        let mut g = Game::new(ctx);
        g.update(&input_space(), ctx, 0.0);
        assert!(matches!(g.phase, Phase::Countdown { .. }));
        g.update(&empty_input(), ctx, ctx.tuning.countdown + 0.1);
        assert!(matches!(g.phase, Phase::Playing));
        g
    }

    #[test]
    fn test_new_starts_in_menu() {
        let ctx = ctx();
        let g = Game::new(&ctx);
        assert!(matches!(g.phase, Phase::Menu));
        assert_eq!(g.score.p1, 0);
        assert_eq!(g.score.p2, 0);
        assert_eq!(g.rounds.p1, 0);
        assert_eq!(g.rounds.p2, 0);
    }

    #[test]
    fn test_menu_to_countdown_on_space() {
        let ctx = ctx();
        let mut g = Game::new(&ctx);
        let events = g.update(&input_space(), &ctx, 1.0 / 60.0);
        assert!(matches!(g.phase, Phase::Countdown { .. }));
        assert!(events.contains(&GameEvent::CountdownBeep));
    }

    #[test]
    fn test_countdown_advances_to_playing() {
        let ctx = ctx();
        let mut g = Game::new(&ctx);
        g.update(&input_space(), &ctx, 0.0);
        let events = g.update(&empty_input(), &ctx, ctx.tuning.countdown + 0.1);
        assert!(matches!(g.phase, Phase::Playing));
        assert!(events.contains(&GameEvent::CountdownGo));
    }

    #[test]
    fn test_goal_triggers_goal_pause() {
        let ctx = ctx();
        let mut g = game_in_playing(&ctx);
        // Place the puck near the right goal, moving right. Left will score.
        g.puck.pos = Vec2::new(ctx.arena.w - 20.0, ctx.arena.mid_y());
        g.puck.vel = Vec2::new(500.0, 0.0);
        let events = g.update(&empty_input(), &ctx, 1.0 / 60.0);
        assert!(matches!(
            g.phase,
            Phase::GoalPause {
                scorer: Side::Left,
                ..
            }
        ));
        assert_eq!(g.score.p1, 1);
        assert!(events.contains(&GameEvent::Goal(Side::Left)));
    }

    #[test]
    fn test_goal_pause_resets_and_returns_to_countdown() {
        let ctx = ctx();
        let mut g = game_in_playing(&ctx);
        g.puck.pos = Vec2::new(ctx.arena.w - 20.0, ctx.arena.mid_y());
        g.puck.vel = Vec2::new(500.0, 0.0);
        g.update(&empty_input(), &ctx, 1.0 / 60.0);
        assert!(matches!(g.phase, Phase::GoalPause { .. }));
        // Advance past the pause.
        g.update(&empty_input(), &ctx, ctx.tuning.goal_pause + 0.1);
        assert!(matches!(g.phase, Phase::Countdown { .. }));
        let center = Vec2::new(ctx.arena.mid_x(), ctx.arena.mid_y());
        assert!((g.puck.pos - center).length() < 1e-4);
        assert_eq!(g.puck.vel, Vec2::ZERO);
    }

    #[test]
    fn test_round_over_after_max_goals() {
        let ctx = ctx();
        let mut g = game_in_playing(&ctx);
        g.score.p1 = ctx.tuning.goals_to_win_round - 1;
        g.puck.pos = Vec2::new(ctx.arena.w - 20.0, ctx.arena.mid_y());
        g.puck.vel = Vec2::new(500.0, 0.0);
        g.update(&empty_input(), &ctx, 1.0 / 60.0);
        assert!(matches!(g.phase, Phase::GoalPause { .. }));
        let events = g.update(&empty_input(), &ctx, ctx.tuning.goal_pause + 0.1);
        assert!(matches!(
            g.phase,
            Phase::RoundOver {
                winner: Side::Left,
                ..
            }
        ));
        assert!(events.contains(&GameEvent::RoundWon(Side::Left)));
        assert_eq!(g.rounds.p1, 1);
    }

    #[test]
    fn test_match_over_after_2_rounds() {
        let ctx = ctx();
        let mut g = game_in_playing(&ctx);
        g.rounds.p1 = 1;
        g.score.p1 = ctx.tuning.goals_to_win_round - 1;
        g.puck.pos = Vec2::new(ctx.arena.w - 20.0, ctx.arena.mid_y());
        g.puck.vel = Vec2::new(500.0, 0.0);
        g.update(&empty_input(), &ctx, 1.0 / 60.0);
        let events = g.update(&empty_input(), &ctx, ctx.tuning.goal_pause + 0.1);
        assert!(matches!(
            g.phase,
            Phase::MatchOver {
                winner: Side::Left
            }
        ));
        assert!(events.contains(&GameEvent::MatchWon(Side::Left)));
        assert_eq!(g.rounds.p1, 2);
    }

    #[test]
    fn test_rematch_resets_everything() {
        let ctx = ctx();
        let mut g = game_in_playing(&ctx);
        g.score.p1 = 3;
        g.score.p2 = 2;
        g.rounds.p1 = 1;
        g.rounds.p2 = 1;
        g.phase = Phase::MatchOver { winner: Side::Left };

        g.update(&input_space(), &ctx, 0.0);
        assert!(matches!(g.phase, Phase::Countdown { .. }));
        assert_eq!(g.score.p1, 0);
        assert_eq!(g.score.p2, 0);
        assert_eq!(g.rounds.p1, 0);
        assert_eq!(g.rounds.p2, 0);
    }

    #[test]
    fn test_pause_freezes_physics() {
        let ctx = ctx();
        let mut g = game_in_playing(&ctx);
        g.puck.vel = Vec2::new(500.0, 0.0);
        let before = g.puck.pos;
        // First update: toggles pause on. Physics runs with dt_eff = 0.
        g.update(&input_p(), &ctx, 1.0 / 60.0);
        assert!(g.paused);
        // Second update: still paused. Puck must not move.
        g.update(&empty_input(), &ctx, 1.0 / 60.0);
        assert!((g.puck.pos - before).length() < 1e-6);
    }

    #[test]
    fn test_pause_toggle_on_p() {
        let ctx = ctx();
        let mut g = game_in_playing(&ctx);
        assert!(!g.paused);
        g.update(&input_p(), &ctx, 1.0 / 60.0);
        assert!(g.paused);
        g.update(&input_p(), &ctx, 1.0 / 60.0);
        assert!(!g.paused);
    }
}