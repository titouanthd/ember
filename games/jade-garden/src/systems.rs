//! State struct `Game`, state machine, and gameplay logic.
//!
//! Cycle: idle → swap → resolution → cascade → idle.

use ember_core::rng::Rng;
use glam::Vec2;
use macroquad::prelude::*;

use crate::components::{GRID_H, GRID_W};
use crate::config::GameContext;
use crate::grid::Grid;
use crate::juice::Juice;
use crate::level::{LevelConfig, Objective};
use crate::tile_render::jade_color_base;
use crate::scoring::{cascade_tier, score_for_match};

pub const SWAP_DURATION: f32 = 0.15;
pub const SHRINK_DURATION: f32 = 0.25;
pub const FALL_K: f32 = 15.0;
pub const MAX_CASCADE: u32 = 15;
pub const SLOWMO_DURATION: f32 = 0.25;

/// (cell A, cell B, objective gain) for a candidate swap.
type SwapCandidate = ((usize, usize), (usize, usize), u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Playing,
    Won,
    Lost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolveStep {
    Shrinking,
    Falling,
    Checking,
}

#[derive(Debug, Clone)]
pub struct SwapAnim {
    pub a: (usize, usize),
    pub b: (usize, usize),
    pub progress: f32,
    pub forward: bool,
}

/// Active hint: the two tiles to swap, plus a lifetime countdown.
#[derive(Debug, Clone, Copy)]
pub struct HintState {
    pub a: (usize, usize),
    pub b: (usize, usize),
    pub elapsed: f32,
    pub ttl: f32,
}

#[derive(Debug, Clone)]
pub struct Resolution {
    pub to_remove: Vec<(usize, usize)>,
    pub elapsed: f32,
    pub step: ResolveStep,
}

pub struct Game {
    pub phase: Phase,
    pub grid: Grid,
    pub level: LevelConfig,
    pub score: u32,
    pub moves_left: u32,
    pub cascade_level: u32,
    pub target_score: u32,
    pub selected: Option<(usize, usize)>,
    pub hover: Option<(usize, usize)>,
    pub swap_anim: Option<SwapAnim>,
    pub resolution: Option<Resolution>,
    pub juice: Juice,
    pub rng: Rng,
    pub time: f32,
    pub objective_progress: u32,
    pub slowmo_timer: f32,
    pub slowmo_strength: f32,
    pub hint: Option<HintState>,
    pub hint_uses_left: u32,
    /// If true, the level ends as soon as the objective is met.
    /// If false (replay), the player plays until moves run out.
    pub stop_on_objective: bool,
    pub elapsed: f32,
    pub crossed_1_star: Option<f32>,
    pub crossed_2_star: Option<f32>,
    pub crossed_3_star: Option<f32>,
    pub peak_score: u32,
    pub peak_score_time: f32,
}

impl Game {
    /// Test constructor: default level, parameterisable seed.
    pub fn new(seed: u32) -> Self {
        Self::with_level(LevelConfig {
            seed,
            ..Default::default()
        })
    }

    pub fn with_level(level: LevelConfig) -> Self {
        Self::with_level_and_progress(level, true)
    }

    /// Same as `with_level` but the caller can force replay mode
    /// (`stop_on_objective = false`), so the level keeps going after
    /// the 1-star threshold.
    pub fn with_level_and_progress(level: LevelConfig, stop_on_objective: bool) -> Self {
        let seed = level.seed;
        let moves_left = level.moves;
        let target_score = level.star_target;
        Self {
            phase: Phase::Playing,
            grid: Grid::new(seed),
            level,
            score: 0,
            moves_left,
            cascade_level: 0,
            target_score,
            selected: None,
            hover: None,
            swap_anim: None,
            resolution: None,
            juice: Juice::new(),
            rng: Rng::new(seed),
            time: 0.0,
            objective_progress: 0,
            slowmo_timer: 0.0,
            slowmo_strength: 1.0,
            hint: None,
            hint_uses_left: 3,
            stop_on_objective,
            elapsed: 0.0,
            crossed_1_star: None,
            crossed_2_star: None,
            crossed_3_star: None,
            peak_score: 0,
            peak_score_time: 0.0,
        }
    }

    pub fn tick(
        &mut self,
        ctx: &GameContext,
        dt: f32,
        mouse_pos: Vec2,
        mouse_left_pressed: bool,
    ) {
        self.time += dt;
        self.elapsed += dt;
        self.juice.tick(dt);

        if self.slowmo_timer > 0.0 {
            self.slowmo_timer = (self.slowmo_timer - dt).max(0.0);
        }

        if self.phase != Phase::Playing {
            return;
        }

        // Tick the hint timer even during animations so it fades out
        // cleanly.
        if let Some(h) = self.hint.as_mut() {
            h.elapsed += dt;
            if h.elapsed >= h.ttl {
                self.hint = None;
            }
        }

        // Slow-mo on big cascades: scale gameplay dt, not juice / time.
        let dt = dt * self.current_time_scale();

        self.hover = ctx.tile_at(mouse_pos.x, mouse_pos.y);

        if self.swap_anim.is_some() {
            self.tick_swap(dt);
        } else if self.resolution.is_some() {
            self.tick_resolution(dt);
        } else {
            self.tick_visuals(dt);
            self.handle_input(mouse_left_pressed);
        }
    }

    fn handle_input(&mut self, mouse_left_pressed: bool) {
        if !mouse_left_pressed {
            return;
        }
        let Some(clicked) = self.hover else {
            self.selected = None;
            return;
        };

        match self.selected {
            None => self.selected = Some(clicked),
            Some(sel) if sel == clicked => self.selected = None,
            Some(sel) => {
                if is_adjacent(sel, clicked) {
                    self.start_swap(sel, clicked);
                    self.selected = None;
                } else {
                    self.selected = Some(clicked);
                }
            }
        }
    }

    /// Computes the best swap for the current objective and highlights
    /// it for a few seconds. No-op if a swap or resolution is in flight.
    pub fn request_hint(&mut self) {
        if self.phase != Phase::Playing
            || self.swap_anim.is_some()
            || self.resolution.is_some()
            || self.hint_uses_left == 0
        {
            return;
        }
        let Some((a, b)) = self.find_best_swap() else {
            return;
        };
        self.hint = Some(HintState {
            a,
            b,
            elapsed: 0.0,
            ttl: 3.0,
        });
        self.hint_uses_left -= 1;
    }

    /// Returns the swap that maximises progress toward the level's
    /// objective, or `None` if no swap produces any progress.
    fn find_best_swap(&self) -> Option<((usize, usize), (usize, usize))> {
        let mut best: Option<SwapCandidate> = None;

        for row in 0..GRID_H {
            for col in 0..(GRID_W - 1) {
                let a = (row, col);
                let b = (row, col + 1);
                if let Some(gain) = self.eval_swap_gain(a, b)
                    && best.is_none_or(|(_, _, g)| gain > g)
                {
                    best = Some((a, b, gain));
                }
            }
        }
        for row in 0..(GRID_H - 1) {
            for col in 0..GRID_W {
                let a = (row, col);
                let b = (row + 1, col);
                if let Some(gain) = self.eval_swap_gain(a, b)
                    && best.is_none_or(|(_, _, g)| gain > g)
                {
                    best = Some((a, b, gain));
                }
            }
        }

        if let Some((a, b, _)) = best {
            return Some((a, b));
        }

        // Fallback: no objective-progressing swap found (e.g. a
        // FillPoem board where no swap triggers a cascade). Point at
        // any swap that at least produces a match, so the hint is
        // never empty and never silently consumes a use.
        for row in 0..GRID_H {
            for col in 0..(GRID_W - 1) {
                let a = (row, col);
                let b = (row, col + 1);
                let mut test = self.grid;
                test.swap(a, b);
                if test.has_match_at(a.0, a.1) || test.has_match_at(b.0, b.1) {
                    return Some((a, b));
                }
            }
        }
        for row in 0..(GRID_H - 1) {
            for col in 0..GRID_W {
                let a = (row, col);
                let b = (row + 1, col);
                let mut test = self.grid;
                test.swap(a, b);
                if test.has_match_at(a.0, a.1) || test.has_match_at(b.0, b.1) {
                    return Some((a, b));
                }
            }
        }

        None
    }

    /// Estimates the objective gain of swapping `a` and `b`, playing
    /// out all cascades with a throwaway RNG. Returns `None` if the
    /// swap produces no match.
    fn eval_swap_gain(
        &self,
        a: (usize, usize),
        b: (usize, usize),
    ) -> Option<u32> {
        let mut test = self.grid;
        test.swap(a, b);
        if !test.has_match_at(a.0, a.1) && !test.has_match_at(b.0, b.1) {
            return None;
        }

        let mut total = 0u32;
        let mut cascade_level = 0u32;
        loop {
            let matches = test.find_matches();
            if matches.is_empty() {
                break;
            }
            match self.level.objective {
                Objective::Score(_) => {
                    total += score_for_match(matches.len(), cascade_level);
                }
                Objective::ClearJade(jade, _) => {
                    let n = matches
                        .iter()
                        .filter(|&&(r, c)| {
                            test.get(r, c).map(|t| t.jade == jade).unwrap_or(false)
                        })
                        .count() as u32;
                    total += n;
                }
                Objective::FillPoem(_) => {
                    if cascade_level > 0 {
                        total += 1;
                    }
                }
            }
            // Deterministic throwaway RNG so the estimate is stable.
            let mut rng = Rng::new(0xDEAD_BEEF ^ cascade_level);
            test.remove_and_collapse(&matches, &mut rng);
            cascade_level += 1;
            if cascade_level > 15 {
                break;
            }
        }

        if total == 0 { None } else { Some(total) }
    }

    fn start_swap(&mut self, a: (usize, usize), b: (usize, usize)) {
        self.hint = None;
        self.swap_anim = Some(SwapAnim {
            a,
            b,
            progress: 0.0,
            forward: true,
        });
    }

    fn tick_swap(&mut self, dt: f32) {
        let anim = match self.swap_anim.as_mut() {
            Some(a) => a,
            None => return,
        };
        anim.progress = (anim.progress + dt / SWAP_DURATION).min(1.0);

        let t = if anim.forward {
            anim.progress
        } else {
            1.0 - anim.progress
        };
        let a = anim.a;
        let b = anim.b;

        if let Some(ta) = self.grid.get_mut(a.0, a.1) {
            ta.visual_row = lerp(a.0 as f32, b.0 as f32, t);
            ta.visual_col = lerp(a.1 as f32, b.1 as f32, t);
        }
        if let Some(tb) = self.grid.get_mut(b.0, b.1) {
            tb.visual_row = lerp(b.0 as f32, a.0 as f32, t);
            tb.visual_col = lerp(b.1 as f32, a.1 as f32, t);
        }

        if anim.progress < 1.0 {
            return;
        }

        let forward = anim.forward;
        let a = anim.a;
        let b = anim.b;

        if !forward {
            self.swap_anim = None;
            return;
        }

        let creates_match = self.would_match_after_swap(a, b);
        if creates_match {
            self.grid.swap(a, b);
            self.swap_anim = None;
            self.moves_left = self.moves_left.saturating_sub(1);
            self.cascade_level = 0;
            self.begin_resolution();
        } else if let Some(anim) = self.swap_anim.as_mut() {
            anim.forward = false;
            anim.progress = 0.0;
        }
    }

    fn would_match_after_swap(&self, a: (usize, usize), b: (usize, usize)) -> bool {
        let mut test = self.grid;
        test.swap(a, b);
        test.has_match_at(a.0, a.1) || test.has_match_at(b.0, b.1)
    }

    fn begin_resolution(&mut self) {
        let to_remove = self.grid.find_matches();
        if to_remove.is_empty() {
            self.cascade_level = 0;
            return;
        }

        let gained = score_for_match(to_remove.len(), self.cascade_level);
        self.score += gained;

        self.check_time_thresholds();

        match self.level.objective {
            Objective::ClearJade(jade, _) => {
                let n = to_remove
                    .iter()
                    .filter(|&&(r, c)| {
                        self.grid
                            .get(r, c)
                            .map(|t| t.jade == jade)
                            .unwrap_or(false)
                    })
                    .count() as u32;
                self.objective_progress += n;
            }
            Objective::FillPoem(_) => {
                // Only cascades count — the first match of a swap is
                // free, so the player has to actually chain reactions.
                if self.cascade_level > 0 {
                    self.objective_progress += 1;
                    self.juice.spawn_floating_text(
                        self.tile_center_avg(&to_remove),
                        "+1 chain".to_string(),
                        Color::new(0.85, 0.70, 0.45, 1.0),
                        16.0,
                    );
                }
            }
            Objective::Score(_) => {}
        }

        let center = self.tile_center_avg(&to_remove);
        let tier = cascade_tier(self.cascade_level);

        // --- Floating score text (bigger and hotter with tier) ---
        let text_size = 22.0 + tier as f32 * 5.0;
        let text_color = match tier {
            0 => Color::new(0.95, 0.90, 0.72, 1.0),
            1 => Color::new(1.00, 0.85, 0.40, 1.0),
            2 => Color::new(1.00, 0.70, 0.30, 1.0),
            _ => Color::new(1.00, 0.55, 0.20, 1.0),
        };
        self.juice.spawn_floating_text(
            center,
            format!("+{}", gained),
            text_color,
            text_size,
        );

        // --- Shockwave from every match ---
        let shock_radius = 40.0 + tier as f32 * 32.0;
        let shock_dur = 0.35 + tier as f32 * 0.08;
        let shock_color = if tier == 0 {
            Color::new(0.95, 0.90, 0.72, 0.70)
        } else {
            Color::new(1.00, 0.82, 0.35, 0.85)
        };
        let shock_thickness = 1.5 + tier as f32 * 0.8;
        self.juice.spawn_shockwave(
            center,
            shock_radius,
            shock_color,
            shock_dur,
            shock_thickness,
        );

        // --- Radial burst from tier 2 upward ---
        if tier >= 2 {
            let ray_count = 6 + (tier as u32 - 2) * 4;
            let ray_length = 60.0 + tier as f32 * 15.0;
            let ray_color = Color::new(1.0, 0.82, 0.35, 0.85);
            self.juice.spawn_radial_burst(
                center,
                ray_count,
                ray_length,
                ray_color,
                self.rng.next_u32(),
            );
            self.juice.vignette_pulse(
                0.35 + tier as f32 * 0.12,
                0.35 + tier as f32 * 0.05,
            );
        }

        // --- Cascade banner ---
        if self.cascade_level > 0 {
            let label = format!("CASCADE ×{}", self.cascade_level + 1);
            let banner_size = 26.0 + self.cascade_level as f32 * 4.0;
            self.juice.spawn_floating_text(
                Vec2::new(center.x, center.y - 44.0),
                label,
                Color::new(1.0, 0.55, 0.20, 1.0),
                banner_size,
            );
        }

        // --- Screen shake scales with tier ---
        let shake = 2.0 + tier as f32 * 3.0;
        self.juice.shake(shake, 0.15 + tier as f32 * 0.03);

        // --- Full-screen flash from tier 2 upward ---
        if tier >= 2 {
            let flash_color = match tier {
                2 => Color::new(1.0, 0.85, 0.40, 1.0),
                3 => Color::new(1.0, 0.75, 0.35, 1.0),
                _ => Color::new(1.0, 0.60, 0.25, 1.0),
            };
            self.juice.flash(flash_color, 0.20 + tier as f32 * 0.05);
        }

        // --- Slow-mo on huge cascades ---
        if tier >= 3 {
            self.slowmo_strength = (0.30 + (tier - 3) as f32 * 0.10).min(0.55);
            self.slowmo_timer = SLOWMO_DURATION;
        }

        let mut seed = self.rng.next_u32();
        for &(row, col) in &to_remove {
            let (Some(tile), Some(pos)) = (self.grid.get(row, col), self.tile_center(row, col))
            else {
                continue;
            };
            let color = jade_color_base(tile.jade);
            self.juice.spawn_jade_shatter(pos, color, 10, seed);
            seed = seed.wrapping_add(0x9E37);
        }

        self.resolution = Some(Resolution {
            to_remove,
            elapsed: 0.0,
            step: ResolveStep::Shrinking,
        });
    }

    fn tile_center(&self, row: usize, col: usize) -> Option<Vec2> {
        let tile = self.grid.get(row, col)?;
        let (gx, gy) = self.ctx_grid_origin();
        Some(Vec2::new(
            gx + (tile.visual_col + 0.5) * crate::tile_render::TILE_STRIDE,
            gy + (tile.visual_row + 0.5) * crate::tile_render::TILE_STRIDE,
        ))
    }

    fn tile_center_avg(&self, coords: &[(usize, usize)]) -> Vec2 {
        if coords.is_empty() {
            return Vec2::ZERO;
        }
        let mut acc = Vec2::ZERO;
        let mut n = 0u32;
        for &(r, c) in coords {
            if let Some(p) = self.tile_center(r, c) {
                acc += p;
                n += 1;
            }
        }
        if n == 0 { Vec2::ZERO } else { acc / n as f32 }
    }

    fn ctx_grid_origin(&self) -> (f32, f32) {
        use crate::tile_render::{TILE_GAP, TILE_STRIDE};
        let grid_w = GRID_W as f32 * TILE_STRIDE - TILE_GAP;
        let grid_h = GRID_H as f32 * TILE_STRIDE - TILE_GAP;
        let vw = screen_width();
        let vh = screen_height();
        ((vw - grid_w) * 0.5, (vh - grid_h) * 0.5)
    }

    /// Returns the current time multiplier (1.0 = normal, <1.0 = slow-mo).
    fn current_time_scale(&self) -> f32 {
        if self.slowmo_timer <= 0.0 {
            return 1.0;
        }
        let t = (self.slowmo_timer / SLOWMO_DURATION).clamp(0.0, 1.0);
        1.0 - self.slowmo_strength * t
    }

    fn tick_resolution(&mut self, dt: f32) {
        let step = self
            .resolution
            .as_ref()
            .map(|r| r.step)
            .unwrap_or(ResolveStep::Checking);

        match step {
            ResolveStep::Shrinking => {
                let res = self.resolution.as_mut().unwrap();
                res.elapsed += dt;
                if res.elapsed >= SHRINK_DURATION {
                    let to_remove = res.to_remove.clone();
                    self.grid.remove_and_collapse(&to_remove, &mut self.rng);
                    self.resolution.as_mut().unwrap().step = ResolveStep::Falling;
                }
            }
            ResolveStep::Falling => {
                self.tick_visuals(dt);
                if self.all_visuals_converged() {
                    self.resolution.as_mut().unwrap().step = ResolveStep::Checking;
                }
            }
            ResolveStep::Checking => {
                let next_matches = self.grid.find_matches();
                if next_matches.is_empty() {
                    self.cascade_level = 0;
                    self.resolution = None;
                    if !self.grid.has_any_valid_move() {
                        self.grid.reshuffle(&mut self.rng);
                        self.juice.shake(6.0, 0.3);
                    }
                    self.check_end_condition();
                } else {
                    self.cascade_level = (self.cascade_level + 1).min(MAX_CASCADE);
                    self.resolution = None;
                    self.begin_resolution();
                }
            }
        }
    }

    fn tick_visuals(&mut self, dt: f32) {
        let k = FALL_K * dt;
        for row in 0..GRID_H {
            for col in 0..GRID_W {
                if let Some(t) = self.grid.get_mut(row, col) {
                    let target_r = t.row as f32;
                    let target_c = t.col as f32;
                    let dr = target_r - t.visual_row;
                    let dc = target_c - t.visual_col;
                    if dr.abs() > 0.005 {
                        t.visual_row += dr * k.min(1.0);
                    } else {
                        t.visual_row = target_r;
                    }
                    if dc.abs() > 0.005 {
                        t.visual_col += dc * k.min(1.0);
                    } else {
                        t.visual_col = target_c;
                    }
                }
            }
        }
    }

    fn all_visuals_converged(&self) -> bool {
        for row in 0..GRID_H {
            for col in 0..GRID_W {
                if let Some(t) = self.grid.get(row, col)
                    && !t.is_visually_idle()
                {
                    return false;
                }
            }
        }
        true
    }

    pub fn is_objective_met(&self) -> bool {
        match self.level.objective {
            Objective::Score(n) => self.score >= n,
            Objective::ClearJade(_, n) => self.objective_progress >= n,
            Objective::FillPoem(n) => self.objective_progress >= n as u32,
        }
    }

    fn check_end_condition(&mut self) {
        let objective_met = self.is_objective_met();
        let should_stop = objective_met && self.stop_on_objective;

        if should_stop {
            self.phase = Phase::Won;
            self.juice.flash(Color::new(1.0, 0.9, 0.5, 1.0), 0.6);
            self.juice.shake(12.0, 0.5);
        } else if self.moves_left == 0 {
            // In replay mode, this is where the level ends — win or
            // lose is decided by whether the objective is met.
            if objective_met {
                self.phase = Phase::Won;
                self.juice.flash(Color::new(1.0, 0.9, 0.5, 1.0), 0.6);
                self.juice.shake(12.0, 0.5);
            } else {
                self.phase = Phase::Lost;
                self.juice.shake(10.0, 0.4);
            }
        }
    }

    /// Records the first time each star threshold was crossed during
    /// this run, plus the time of the peak score.
    fn check_time_thresholds(&mut self) {
        let target = self.level.star_target as f32;
        let s = self.score as f32;
        if self.crossed_1_star.is_none() && s >= target {
            self.crossed_1_star = Some(self.elapsed);
        }
        if self.crossed_2_star.is_none() && s >= target * 1.5 {
            self.crossed_2_star = Some(self.elapsed);
        }
        if self.crossed_3_star.is_none() && s >= target * 2.0 {
            self.crossed_3_star = Some(self.elapsed);
        }
        if self.score > self.peak_score {
            self.peak_score = self.score;
            self.peak_score_time = self.elapsed;
        }
    }

    pub fn tile_scale(&self, row: usize, col: usize) -> f32 {
        let res = match self.resolution.as_ref() {
            Some(r) => r,
            None => return 1.0,
        };
        if res.step != ResolveStep::Shrinking {
            return 1.0;
        }
        if !res.to_remove.contains(&(row, col)) {
            return 1.0;
        }
        let t = (res.elapsed / SHRINK_DURATION).clamp(0.0, 1.0);
        1.0 - t * t
    }

    pub fn tile_alpha(&self, row: usize, col: usize) -> f32 {
        let _ = (row, col);
        1.0
    }
}

fn is_adjacent(a: (usize, usize), b: (usize, usize)) -> bool {
    let dr = (a.0 as isize - b.0 as isize).abs();
    let dc = (a.1 as isize - b.1 as isize).abs();
    dr + dc == 1
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> GameContext {
        GameContext::defaults()
    }

    #[test]
    fn game_starts_playing() {
        let g = Game::new(1);
        assert_eq!(g.phase, Phase::Playing);
        assert_eq!(g.score, 0);
        assert_eq!(g.moves_left, 20);
    }

    #[test]
    fn is_adjacent_horizontal() {
        assert!(is_adjacent((0, 0), (0, 1)));
        assert!(is_adjacent((0, 1), (0, 0)));
    }

    #[test]
    fn is_adjacent_vertical() {
        assert!(is_adjacent((0, 0), (1, 0)));
        assert!(is_adjacent((1, 0), (0, 0)));
    }

    #[test]
    fn is_adjacent_diagonal_is_false() {
        assert!(!is_adjacent((0, 0), (1, 1)));
    }

    #[test]
    fn is_adjacent_same_tile_is_false() {
        assert!(!is_adjacent((3, 3), (3, 3)));
    }

    #[test]
    fn is_adjacent_far_is_false() {
        assert!(!is_adjacent((0, 0), (0, 2)));
        assert!(!is_adjacent((0, 0), (2, 0)));
    }

    #[test]
    fn click_first_tile_selects_it() {
        let ctx = ctx();
        let mut g = Game::new(1);
        let (gx, gy) = ctx.grid_origin();
        g.tick(&ctx, 0.016, Vec2::new(gx + 30.0, gy + 30.0), true);
        assert_eq!(g.selected, Some((0, 0)));
    }

    #[test]
    fn click_same_tile_deselects() {
        let ctx = ctx();
        let mut g = Game::new(1);
        let (gx, gy) = ctx.grid_origin();
        let p = Vec2::new(gx + 30.0, gy + 30.0);
        g.tick(&ctx, 0.016, p, true);
        g.tick(&ctx, 0.016, p, true);
        assert_eq!(g.selected, None);
    }

    #[test]
    fn click_outside_clears_selection() {
        let ctx = ctx();
        let mut g = Game::new(1);
        let (gx, gy) = ctx.grid_origin();
        g.tick(&ctx, 0.016, Vec2::new(gx + 30.0, gy + 30.0), true);
        assert!(g.selected.is_some());
        g.tick(&ctx, 0.016, Vec2::new(5.0, 5.0), true);
        assert_eq!(g.selected, None);
    }

    #[test]
    fn click_non_adjacent_changes_selection() {
        let ctx = ctx();
        let mut g = Game::new(1);
        let (gx, gy) = ctx.grid_origin();
        g.tick(&ctx, 0.016, Vec2::new(gx + 30.0, gy + 30.0), true);
        let (tx, ty) = ctx.tile_origin(4.0, 4.0);
        g.tick(&ctx, 0.016, Vec2::new(tx + 36.0, ty + 36.0), true);
        assert_eq!(g.selected, Some((4, 4)));
    }

    #[test]
    fn tile_scale_is_one_when_idle() {
        let g = Game::new(1);
        assert!((g.tile_scale(0, 0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn lerp_works() {
        assert!((lerp(0.0, 10.0, 0.0) - 0.0).abs() < 1e-6);
        assert!((lerp(0.0, 10.0, 0.5) - 5.0).abs() < 1e-6);
        assert!((lerp(0.0, 10.0, 1.0) - 10.0).abs() < 1e-6);
    }

    #[test]
    fn swap_anim_forward_progresses() {
        let ctx = ctx();
        let mut g = Game::new(1);
        g.swap_anim = Some(SwapAnim {
            a: (0, 0),
            b: (0, 1),
            progress: 0.0,
            forward: true,
        });
        g.tick(&ctx, 0.05, Vec2::ZERO, false);
        assert!(g.swap_anim.is_some());
        let p = g.swap_anim.as_ref().unwrap().progress;
        assert!(p > 0.0 && p < 1.0);
    }

    #[test]
    fn cascade_level_resets_on_no_next_match() {
        let ctx = ctx();
        let mut g = Game::new(1);
        g.cascade_level = 5;
        g.resolution = Some(Resolution {
            to_remove: vec![],
            elapsed: 0.0,
            step: ResolveStep::Checking,
        });
        g.tick(&ctx, 0.016, Vec2::ZERO, false);
        assert_eq!(g.cascade_level, 0);
        assert!(g.resolution.is_none());
    }

    #[test]
    fn game_wins_at_target_score() {
        let mut g = Game::new(1);
        g.score = 1000;
        g.check_end_condition();
        assert_eq!(g.phase, Phase::Won);
    }

    #[test]
    fn game_loses_at_zero_moves_no_score() {
        let mut g = Game::new(1);
        g.score = 0;
        g.moves_left = 0;
        g.check_end_condition();
        assert_eq!(g.phase, Phase::Lost);
    }

    #[test]
    fn win_takes_priority_over_lost() {
        let mut g = Game::new(1);
        g.score = 1000;
        g.moves_left = 0;
        g.check_end_condition();
        assert_eq!(g.phase, Phase::Won);
    }

    #[test]
    fn tick_outside_playing_does_not_change_score() {
        let ctx = ctx();
        let mut g = Game::new(1);
        g.phase = Phase::Won;
        let before = g.score;
        g.tick(&ctx, 0.1, Vec2::ZERO, true);
        assert_eq!(g.score, before);
    }

    #[test]
    fn clear_jade_objective_progresses() {
        use crate::components::Jade;
        let level = LevelConfig {
            objective: Objective::ClearJade(Jade::Bi, 5),
            moves: 20,
            ..Default::default()
        };
        let mut g = Game::with_level(level);
        g.objective_progress = 5;
        assert!(g.is_objective_met());
    }

    #[test]
    fn fill_poem_objective_progresses() {
        let level = LevelConfig {
            objective: Objective::FillPoem(3),
            moves: 20,
            ..Default::default()
        };
        let mut g = Game::with_level(level);
        g.objective_progress = 2;
        assert!(!g.is_objective_met());
        g.objective_progress = 3;
        assert!(g.is_objective_met());
    }

    #[test]
    fn score_objective_not_met_with_low_score() {
        let g = Game::new(1);
        assert!(!g.is_objective_met());
    }

    #[test]
    fn time_scale_is_one_when_no_slowmo() {
        let g = Game::new(1);
        assert!((g.current_time_scale() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn time_scale_is_reduced_during_slowmo() {
        let mut g = Game::new(1);
        g.slowmo_strength = 0.40;
        g.slowmo_timer = SLOWMO_DURATION;
        let s = g.current_time_scale();
        assert!(s < 1.0, "slow-mo should reduce time scale, got {s}");
        assert!(s > 0.0, "time scale should not be zero or negative");
    }

    #[test]
    fn request_hint_sets_a_hint_when_a_valid_swap_exists() {
        let mut g = Game::new(1);
        g.request_hint();
        assert!(g.hint.is_some(), "hint should be set on a fresh grid");
    }

    #[test]
    fn request_hint_is_noop_during_animation() {
        let mut g = Game::new(1);
        g.swap_anim = Some(SwapAnim {
            a: (0, 0),
            b: (0, 1),
            progress: 0.5,
            forward: true,
        });
        g.request_hint();
        assert!(g.hint.is_none(), "no hint while a swap is animating");
    }

    #[test]
    fn starting_a_swap_clears_the_hint() {
        let mut g = Game::new(1);
        g.request_hint();
        assert!(g.hint.is_some());
        g.start_swap((0, 0), (0, 1));
        assert!(g.hint.is_none(), "the hint should disappear once the player acts");
    }

    #[test]
    fn hint_uses_are_limited_to_three() {
        let mut g = Game::new(1);
        assert_eq!(g.hint_uses_left, 3);
        g.request_hint();
        assert_eq!(g.hint_uses_left, 2);
        g.request_hint();
        g.request_hint();
        assert_eq!(g.hint_uses_left, 0);
        // Further requests should be blocked.
        g.hint = None;
        g.request_hint();
        assert!(g.hint.is_none());
    }

    #[test]
    fn request_hint_works_for_score_objective() {
        let mut g = Game::new(1); // default objective = Score(1000)
        g.request_hint();
        assert!(g.hint.is_some(), "score objective should yield a hint");
    }

    #[test]
    fn request_hint_works_for_clear_jade_objective() {
        use crate::components::Jade;
        let level = LevelConfig {
            objective: Objective::ClearJade(Jade::Bi, 5),
            moves: 20,
            seed: 42,
            ..Default::default()
        };
        let mut g = Game::with_level(level);
        g.request_hint();
        assert!(g.hint.is_some(), "clear-jade objective should yield a hint");
    }

    #[test]
    fn request_hint_works_for_fill_poem_objective() {
        // FillPoem only counts cascades (cascade_level > 0), so a hint
        // exists only if some swap triggers at least one cascade. We test
        // the real level seeds from assets/levels.ron.
        for seed in [9u32, 10, 12] {
            let level = LevelConfig {
                objective: Objective::FillPoem(4),
                moves: 25,
                seed,
                ..Default::default()
            };
            let mut g = Game::with_level(level);
            g.request_hint();
            assert!(
                g.hint.is_some(),
                "fill-poem (seed {seed}) should yield a hint"
            );
        }
    }

    #[test]
    fn hint_points_to_adjacent_tiles() {
        let mut g = Game::new(1);
        g.request_hint();
        let h = g.hint.expect("hint should exist");
        assert!(is_adjacent(h.a, h.b), "hint tiles must be adjacent");
    }

    #[test]
    fn hint_starts_at_t_zero_with_full_ttl() {
        let mut g = Game::new(1);
        g.request_hint();
        let h = g.hint.expect("hint should exist");
        assert_eq!(h.elapsed, 0.0);
        assert!((h.ttl - 3.0).abs() < 1e-6, "ttl should be 3s, got {}", h.ttl);
    }

    #[test]
    fn hint_expires_after_ttl() {
        let ctx = ctx();
        let mut g = Game::new(1);
        g.request_hint();
        assert!(g.hint.is_some());
        // Tick past the TTL. The hint is ticked even when idle.
        g.tick(&ctx, 3.5, Vec2::ZERO, false);
        assert!(g.hint.is_none(), "hint should have expired");
    }
}