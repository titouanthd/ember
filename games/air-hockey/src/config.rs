//! Game configuration resolved from `.env`.
//!
//! `GameContext` is the single source of truth for layout, palette, and
//! tuning. It is built once in `main.rs` and passed by reference everywhere.
//! No game code reads `std::env` directly.

use std::path::Path;

use ember_stdlib::config::{env_color, env_f32, env_u32, load_dotenv_once};
use macroquad::prelude::Color;

use crate::arena::Arena;
use crate::components::Side;

/// All colours used by the renderer.
#[derive(Debug, Clone, Copy)]
pub struct Colors {
    pub table_bg: Color,
    pub table_border: Color,
    pub grid: Color,
    pub midline: Color,

    pub p1: Color,
    pub p2: Color,

    pub puck: Color,
    pub puck_trail: Color,

    pub goal_p1: Color,
    pub goal_p2: Color,
    pub goal_flash: Color,

    pub text: Color,
    pub text_shadow: Color,
}

impl Colors {
    /// The colour associated with a side (used for paddles and their goal).
    #[inline]
    pub fn for_side(&self, side: Side) -> Color {
        match side {
            Side::Left => self.p1,
            Side::Right => self.p2,
        }
    }

    /// The goal-glow colour associated with a side.
    #[inline]
    pub fn goal_for_side(&self, side: Side) -> Color {
        match side {
            Side::Left => self.goal_p1,
            Side::Right => self.goal_p2,
        }
    }
}

/// Physics and gameplay tuning. Defaults match DESIGN.md §12 and are
/// overridable per-key from `.env` (see `AH_*` keys).
#[derive(Debug, Clone, Copy)]
pub struct Tuning {
    pub paddle_speed_max: f32,
    pub paddle_accel: f32,

    pub puck_speed_max: f32,
    pub puck_friction: f32,
    pub restitution: f32,
    pub transfer: f32,
    pub curve_factor: f32,

    pub substeps: u32,
    pub dt_max: f32,

    pub goals_to_win_round: u32,
    pub rounds_to_win_match: u32,

    pub goal_pause: f32,
    pub countdown: f32,

    pub shake_duration: f32,
    pub shake_amplitude: f32,
    pub hitstop: f32,
}

impl Tuning {
    /// Default values, identical to DESIGN.md §12.
    /// Used directly in tests so they never depend on a local `.env`.
    pub const fn defaults() -> Self {
        Self {
            paddle_speed_max: 900.0,
            paddle_accel: 9000.0,

            puck_speed_max: 1400.0,
            puck_friction: 0.9995,
            restitution: 1.0,
            transfer: 0.6,
            curve_factor: 0.35,

            substeps: 4,
            dt_max: 1.0 / 30.0,

            goals_to_win_round: 7,
            rounds_to_win_match: 2,

            goal_pause: 1.2,
            countdown: 3.0,

            shake_duration: 0.25,
            shake_amplitude: 6.0,
            hitstop: 0.04,
        }
    }

    fn from_env() -> Self {
        let d = Self::defaults();
        Self {
            paddle_speed_max: env_f32("AH_PADDLE_SPEED_MAX", d.paddle_speed_max),
            paddle_accel: env_f32("AH_PADDLE_ACCEL", d.paddle_accel),

            puck_speed_max: env_f32("AH_PUCK_SPEED_MAX", d.puck_speed_max),
            puck_friction: env_f32("AH_PUCK_FRICTION", d.puck_friction),
            restitution: env_f32("AH_RESTITUTION", d.restitution),
            transfer: env_f32("AH_TRANSFER", d.transfer),
            curve_factor: env_f32("AH_CURVE_FACTOR", d.curve_factor),

            substeps: env_u32("AH_SUBSTEPS", d.substeps).max(1),
            dt_max: env_f32("AH_DT_MAX", d.dt_max).max(1.0 / 240.0),

            goals_to_win_round: env_u32("AH_GOALS_TO_WIN_ROUND", d.goals_to_win_round).max(1),
            rounds_to_win_match: env_u32("AH_ROUNDS_TO_WIN_MATCH", d.rounds_to_win_match).max(1),

            goal_pause: env_f32("AH_GOAL_PAUSE", d.goal_pause).max(0.0),
            countdown: env_f32("AH_COUNTDOWN", d.countdown).max(0.0),

            shake_duration: env_f32("AH_SHAKE_DURATION", d.shake_duration).max(0.0),
            shake_amplitude: env_f32("AH_SHAKE_AMPLITUDE", d.shake_amplitude).max(0.0),
            hitstop: env_f32("AH_HITSTOP", d.hitstop).max(0.0),
        }
    }
}

/// Everything the game needs to run. Built once, passed by reference.
#[derive(Debug, Clone)]
pub struct GameContext {
    pub arena: Arena,
    pub colors: Colors,
    pub tuning: Tuning,
}

impl GameContext {
    /// Build a context with the built-in defaults, *without* reading `.env`.
    /// Intended for tests and for callers that want hermetic behaviour.
    pub fn default_hermetic() -> Self {
        Self {
            arena: Arena::default_layout(),
            colors: Colors::default_hermetic(),
            tuning: Tuning::defaults(),
        }
    }

    /// Load from `.env` in this crate's manifest directory.
    ///
    /// The `manifest_dir` argument to `load_dotenv_once` must come from
    /// `env!("CARGO_MANIFEST_DIR")` of the **calling crate**, not of the
    /// stdlib (see recap piège #27).
    pub fn load() -> Self {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        load_dotenv_once(manifest_dir, "air-hockey");
        Self {
            arena: Arena::default_layout(),
            colors: Colors::from_env(),
            tuning: Tuning::from_env(),
        }
    }
}

impl Colors {
    /// Defaults identical to DESIGN.md §8.1 / `.env`.
    /// Built without reading `.env`, safe in tests.
    pub const fn default_hermetic() -> Self {
        Self {
            table_bg: Color::new(0.08, 0.09, 0.14, 1.0),
            table_border: Color::new(0.24, 0.26, 0.38, 1.0),
            grid: Color::new(0.16, 0.17, 0.24, 0.6),
            midline: Color::new(0.35, 0.38, 0.55, 0.7),
            p1: Color::new(0.45, 0.85, 1.00, 1.0),
            p2: Color::new(1.00, 0.55, 0.75, 1.0),
            puck: Color::new(1.00, 0.95, 0.75, 1.0),
            puck_trail: Color::new(1.00, 0.85, 0.55, 0.35),
            goal_p1: Color::new(0.45, 0.85, 1.00, 0.55),
            goal_p2: Color::new(1.00, 0.55, 0.75, 0.55),
            goal_flash: Color::new(1.00, 1.00, 1.00, 0.9),
            text: Color::new(0.92, 0.94, 1.00, 1.0),
            text_shadow: Color::new(0.05, 0.05, 0.08, 1.0),
        }
    }

    fn from_env() -> Self {
        let d = Self::default_hermetic();
        Self {
            table_bg: env_color("COLOR_TABLE_BG", d.table_bg),
            table_border: env_color("COLOR_TABLE_BORDER", d.table_border),
            grid: env_color("COLOR_GRID", d.grid),
            midline: env_color("COLOR_MIDLINE", d.midline),
            p1: env_color("COLOR_P1", d.p1),
            p2: env_color("COLOR_P2", d.p2),
            puck: env_color("COLOR_PUCK", d.puck),
            puck_trail: env_color("COLOR_PUCK_TRAIL", d.puck_trail),
            goal_p1: env_color("COLOR_GOAL_P1", d.goal_p1),
            goal_p2: env_color("COLOR_GOAL_P2", d.goal_p2),
            goal_flash: env_color("COLOR_GOAL_FLASH", d.goal_flash),
            text: env_color("COLOR_TEXT", d.text),
            text_shadow: env_color("COLOR_TEXT_SHADOW", d.text_shadow),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults_match_design() {
        let t = Tuning::defaults();
        assert_eq!(t.paddle_speed_max, 900.0);
        assert_eq!(t.paddle_accel, 9000.0);
        assert_eq!(t.puck_speed_max, 1400.0);
        assert_eq!(t.puck_friction, 0.9995);
        assert_eq!(t.restitution, 1.0);
        assert_eq!(t.transfer, 0.6);
        assert_eq!(t.curve_factor, 0.35);
        assert_eq!(t.substeps, 4);
        assert_eq!(t.goals_to_win_round, 7);
        assert_eq!(t.rounds_to_win_match, 2);
    }

    #[test]
    fn test_tuning_positive() {
        let t = Tuning::defaults();
        assert!(t.paddle_speed_max > 0.0);
        assert!(t.paddle_accel > 0.0);
        assert!(t.puck_speed_max > 0.0);
        assert!(t.substeps >= 1);
        assert!(t.dt_max > 0.0);
        assert!(t.goal_pause >= 0.0);
        assert!(t.countdown >= 0.0);
        assert!(t.hitstop >= 0.0);
        assert!(t.shake_amplitude >= 0.0);
        // These are ratios/factors, not speeds: must be positive.
        assert!(t.puck_friction > 0.0);
        assert!(t.restitution >= 0.0);
        assert!(t.transfer >= 0.0);
        assert!(t.curve_factor >= 0.0);
    }

    #[test]
    fn test_colors_alpha_ranges() {
        let c = Colors::default_hermetic();
        for col in [
            c.table_bg, c.table_border, c.grid, c.midline,
            c.p1, c.p2, c.puck, c.puck_trail,
            c.goal_p1, c.goal_p2, c.goal_flash,
            c.text, c.text_shadow,
        ] {
            assert!((0.0..=1.0).contains(&col.a));
            assert!((0.0..=1.0).contains(&col.r));
            assert!((0.0..=1.0).contains(&col.g));
            assert!((0.0..=1.0).contains(&col.b));
        }
    }

    #[test]
    fn test_hermetic_context_has_expected_layout() {
        let ctx = GameContext::default_hermetic();
        assert_eq!(ctx.arena.w, 1280.0);
        assert_eq!(ctx.arena.h, 720.0);
        // Left/right palettes must be distinct.
        assert_ne!(ctx.colors.p1.r, ctx.colors.p2.r);
    }

    #[test]
    fn test_colors_for_side() {
        let c = Colors::default_hermetic();
        assert_eq!(c.for_side(Side::Left).r, c.p1.r);
        assert_eq!(c.for_side(Side::Right).r, c.p2.r);
        assert_eq!(c.goal_for_side(Side::Left).r, c.goal_p1.r);
        assert_eq!(c.goal_for_side(Side::Right).r, c.goal_p2.r);
    }
}