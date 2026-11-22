// games/pagoda/src/systems.rs
use crate::AppState;
use crate::components::{Block, SlidingBlock};
use crate::config::{GameContext, block_color};
use crate::controls::confirm_pressed;
use crate::effects::Effects;
use ember_stdlib::input::Input;
use glam::Vec2;
use macroquad::prelude::*;

/// Fraction of the screen height at which the top block should settle.
const CAMERA_TOP_FRACTION: f32 = 0.7;
/// Camera follow speed, in "fraction of the gap per second".
const CAMERA_FOLLOW_SPEED: f32 = 10.0;
/// How fast the perfect-drop camera punch returns to rest, per second.
const CAMERA_PUNCH_DECAY: f32 = 3.5;
/// One frame at 60 Hz. The perfect window is at least this much travel.
const PERFECT_FRAME: f32 = 1.0 / 60.0;
/// How fast the gold landing outline fades, per second.
const LAND_PULSE_DECAY: f32 = 5.0;

/// Events emitted by one simulation update.
///
/// `main.rs` translates these into sound. Keeping them as data means the
/// gameplay logic stays testable without an audio context.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GameEvent {
    /// A block landed successfully (perfect or not).
    Land,
    /// An overhang was cut off on a normal landing. `cut_width` is the
    /// total width removed, in virtual pixels; the audio layer uses it
    /// to keep small shaves quiet.
    Slice { cut_width: f32 },
    /// A perfect drop, with the combo after the drop.
    Perfect { combo: u32 },
    /// The sliding block missed the tower; the run is over.
    Miss,
}

/// All mutable state for one run.
pub struct MatchState {
    pub tower: Vec<Block>,
    pub sliding: SlidingBlock,
    pub camera_y: f32,
    /// True when the last successful drop landed within the perfect threshold.
    pub last_drop_perfect: bool,
    /// Consecutive perfect drops. Shown in the HUD.
    pub combo: u32,
    pub juice: Effects,
    /// 1.0 right after a perfect drop, decays to 0.0. Drives the camera punch.
    pub camera_punch: f32,
    /// 1.0 right after any landing, decays to 0.0. Drives the gold
    /// outline on the newest block.
    pub land_pulse: f32,
}

impl MatchState {
    pub fn new(ctx: &GameContext) -> Self {
        let start_y = ctx.screen_h - 100.0;
        let start_x = ctx.screen_w / 2.0 - ctx.base_block_width / 2.0;

        let base_block = Block::new(
            start_x,
            start_y,
            ctx.base_block_width,
            ctx.block_height,
            block_color(0),
        );

        let sliding = SlidingBlock {
            x: 0.0,
            width: ctx.base_block_width,
            direction: 1.0,
            speed: ctx.base_speed,
            color: block_color(1),
        };

        let mut state = Self {
            tower: vec![base_block],
            sliding,
            // Start already framed: avoids a visible drift on the first frames.
            camera_y: camera_target(start_y, ctx.screen_h),
            last_drop_perfect: false,
            combo: 0,
            juice: Effects::new(),
            camera_punch: 0.0,
            land_pulse: 0.0,
        };

        state.reset_sliding(ctx);
        state
    }

    /// Place the sliding block at the edge matching its direction.
    ///
    /// Does not change speed: speed is bumped explicitly after a successful
    /// drop, so a fresh match starts at `base_speed`.
    pub fn reset_sliding(&mut self, ctx: &GameContext) {
        self.sliding.x = start_x(self.sliding.direction, self.sliding.width, ctx.screen_w);
        self.sliding.color = block_color(self.tower.len() as u32);
    }

    /// Increase the sliding speed after a successful drop.
    fn bump_speed(&mut self, ctx: &GameContext) {
        self.sliding.speed = next_speed(self.sliding.speed, ctx.speed_increment, ctx.max_speed);
    }
}

/// Where the sliding block starts for a given direction.
pub fn start_x(direction: f32, width: f32, screen_w: f32) -> f32 {
    if direction > 0.0 {
        0.0
    } else {
        (screen_w - width).max(0.0)
    }
}

/// 1-D AABB intersection: returns `(x, width)` when the blocks overlap strictly.
///
/// Edge-touching blocks count as a miss (`width == 0.0`).
pub fn slice_x(slide_x: f32, slide_w: f32, top_x: f32, top_w: f32) -> Option<(f32, f32)> {
    if slide_w <= 0.0 || top_w <= 0.0 {
        return None;
    }

    let left = slide_x.max(top_x);
    let right = (slide_x + slide_w).min(top_x + top_w);
    let width = right - left;

    if width > 0.0 {
        Some((left, width))
    } else {
        None
    }
}

/// Perfect drop when the block centers are within `threshold` pixels.
pub fn is_perfect(slide_x: f32, slide_w: f32, top_x: f32, top_w: f32, threshold: f32) -> bool {
    let slide_center = slide_x + slide_w / 2.0;
    let top_center = top_x + top_w / 2.0;
    (slide_center - top_center).abs() <= threshold.max(0.0)
}

/// Radius of the live perfect-drop window for a block moving at `speed`.
///
/// A fixed 4 px window is narrower than one 60 Hz frame of travel
/// (8–13 px at Pagoda's speeds), so whether a drop could even *reach*
/// the window was largely frame luck. Widening it to one frame of
/// travel keeps the perfect achievable at every speed while remaining
/// a small fraction of the block width. `base_threshold` is the floor
/// for slow blocks.
pub fn perfect_window(speed: f32, base_threshold: f32) -> f32 {
    (speed.abs() * PERFECT_FRAME).max(base_threshold.max(0.0))
}

/// Placement of a perfect drop: centered on the block below and grown
/// symmetrically, capped at `max_width`.
pub fn perfect_placement(top_x: f32, top_w: f32, growth: f32, max_width: f32) -> (f32, f32) {
    let width = (top_w + growth.max(0.0)).min(max_width.max(0.0));
    let center = top_x + top_w / 2.0;
    (center - width / 2.0, width)
}

/// Deterministic seed for one drop's visual effects.
fn fx_seed(height: usize, combo: u32) -> u32 {
    (height as u32)
        .wrapping_mul(0x9E37_79B9)
        .wrapping_add(combo.wrapping_mul(0x85EB_CA6B))
        ^ 0xA5A5_5A5A
}

/// Next sliding speed, clamped to the configured maximum.
pub fn next_speed(current: f32, increment: f32, max_speed: f32) -> f32 {
    (current + increment).clamp(0.0, max_speed.max(0.0))
}

/// Camera offset that puts `top_y` at [`CAMERA_TOP_FRACTION`] of the screen.
pub fn camera_target(top_y: f32, screen_h: f32) -> f32 {
    screen_h * CAMERA_TOP_FRACTION - top_y
}

/// Framerate-independent camera follow. A large `dt` snaps to the target.
pub fn smooth_camera(current: f32, target: f32, dt: f32) -> f32 {
    let t = (CAMERA_FOLLOW_SPEED * dt).clamp(0.0, 1.0);
    current + (target - current) * t
}

pub fn update(
    state: &mut MatchState,
    app: &mut crate::App,
    ctx: &GameContext,
    dt: f32,
    input: &Input,
) -> Vec<GameEvent> {
    let mut events = Vec::with_capacity(2);

    // Effects keep animating on the game-over screen.
    state.juice.tick(dt);
    state.camera_punch = (state.camera_punch - CAMERA_PUNCH_DECAY * dt).max(0.0);
    state.land_pulse = (state.land_pulse - LAND_PULSE_DECAY * dt).max(0.0);

    if app.state != AppState::Playing {
        return events;
    }

    // 1. Move the sliding block.
    state.sliding.x += state.sliding.speed * state.sliding.direction * dt;

    // 2. Bounce on screen edges.
    if state.sliding.x < 0.0 {
        state.sliding.x = 0.0;
        state.sliding.direction = 1.0;
    } else if state.sliding.x + state.sliding.width > ctx.screen_w {
        state.sliding.x = ctx.screen_w - state.sliding.width;
        state.sliding.direction = -1.0;
    }

    // 3. Drop the block.
    if confirm_pressed(input) {
        let (top_x, top_y, top_w) = {
            let top_block = state.tower.last().expect("tower always has a base block");
            (
                top_block.transform.position.x,
                top_block.transform.position.y,
                top_block.transform.scale.x,
            )
        };
        let slide_x = state.sliding.x;
        let slide_w = state.sliding.width;

        match slice_x(slide_x, slide_w, top_x, top_w) {
            Some((overlap_x, overlap_w)) => {
                events.push(GameEvent::Land);
                state.land_pulse = 1.0;

                let window = perfect_window(state.sliding.speed, ctx.perfect_threshold);
                let perfect = is_perfect(slide_x, slide_w, top_x, top_w, window);
                state.last_drop_perfect = perfect;

                let new_y = top_y - ctx.block_height;
                let (new_x, new_w) = if perfect {
                    perfect_placement(top_x, top_w, ctx.perfect_growth, ctx.base_block_width)
                } else {
                    (overlap_x, overlap_w)
                };

                let new_block =
                    Block::new(new_x, new_y, new_w, ctx.block_height, state.sliding.color);
                state.tower.push(new_block);

                let center = Vec2::new(new_x + new_w / 2.0, new_y + ctx.block_height / 2.0);
                let seed = fx_seed(state.tower.len(), state.combo);

                if perfect {
                    state.combo = state.combo.saturating_add(1);
                    state.camera_punch = 1.0;
                    state.juice.spawn_perfect(center, state.combo, seed);
                    events.push(GameEvent::Perfect { combo: state.combo });
                } else {
                    state.combo = 0;
                    state.juice.spawn_land(center, state.sliding.color, seed);

                    // The overhang is cut from the *sliding* block and falls
                    // away on each side that extended past the block below.
                    let cut_left = overlap_x - slide_x;
                    let cut_right = (slide_x + slide_w) - (overlap_x + overlap_w);
                    if cut_left > 0.0 {
                        state.juice.spawn_slice_piece(
                            Vec2::new(slide_x, new_y),
                            Vec2::new(cut_left, ctx.block_height),
                            state.sliding.color,
                            -1.0,
                            seed ^ 0x1234_5678,
                        );
                    }
                    if cut_right > 0.0 {
                        state.juice.spawn_slice_piece(
                            Vec2::new(overlap_x + overlap_w, new_y),
                            Vec2::new(cut_right, ctx.block_height),
                            state.sliding.color,
                            1.0,
                            seed ^ 0x8765_4321,
                        );
                    }
                    if cut_left > 0.0 || cut_right > 0.0 {
                        events.push(GameEvent::Slice {
                            cut_width: cut_left + cut_right,
                        });
                    }
                }

                state.sliding.width = new_w;
                state.sliding.direction *= -1.0;
                state.bump_speed(ctx);
                state.reset_sliding(ctx);

                app.score += 1;
            }
            None => {
                state.last_drop_perfect = false;
                state.combo = 0;
                state.juice.shake(8.0, 0.4);
                app.state = AppState::GameOver;
                events.push(GameEvent::Miss);
            }
        }
    }

    // 4. Camera smooth pan (always runs while playing).
    if let Some(top_block) = state.tower.last() {
        let target = camera_target(top_block.transform.position.y, ctx.screen_h);
        state.camera_y = smooth_camera(state.camera_y, target, dt);
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;

    fn ctx() -> GameContext {
        GameContext::default()
    }

    fn press(key: KeyCode) -> Input {
        Input {
            keys_pressed: vec![key],
            ..Default::default()
        }
    }

    fn click() -> Input {
        Input {
            mouse_left_pressed: true,
            ..Default::default()
        }
    }

    fn playing_app() -> App {
        let mut app = App::new();
        app.state = AppState::Playing;
        app
    }

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 1e-4,
            "expected {expected}, got {actual}"
        );
    }

    fn drop_at(state: &mut MatchState, app: &mut App, ctx: &GameContext, x: f32) {
        state.sliding.x = x;
        update(state, app, ctx, 0.0, &press(KeyCode::Space));
    }

    // ------------------------------------------------------------------
    // Pure helpers
    // ------------------------------------------------------------------

    #[test]
    fn slice_full_overlap_keeps_width() {
        assert_eq!(slice_x(100.0, 200.0, 100.0, 200.0), Some((100.0, 200.0)));
    }

    #[test]
    fn slice_left_overhang_returns_intersection() {
        assert_eq!(slice_x(0.0, 200.0, 100.0, 200.0), Some((100.0, 100.0)));
    }

    #[test]
    fn slice_right_overhang_returns_intersection() {
        assert_eq!(slice_x(100.0, 200.0, 0.0, 200.0), Some((100.0, 100.0)));
    }

    #[test]
    fn slice_slide_inside_top_keeps_slide_width() {
        assert_eq!(slice_x(100.0, 50.0, 0.0, 200.0), Some((100.0, 50.0)));
    }

    #[test]
    fn slice_top_inside_slide_returns_top_width() {
        assert_eq!(slice_x(0.0, 300.0, 100.0, 50.0), Some((100.0, 50.0)));
    }

    #[test]
    fn slice_touching_edges_is_a_miss() {
        assert_eq!(slice_x(0.0, 100.0, 100.0, 100.0), None);
        assert_eq!(slice_x(200.0, 100.0, 100.0, 100.0), None);
    }

    #[test]
    fn slice_no_overlap_is_a_miss() {
        assert_eq!(slice_x(0.0, 100.0, 300.0, 200.0), None);
    }

    #[test]
    fn slice_zero_width_is_a_miss() {
        assert_eq!(slice_x(100.0, 0.0, 100.0, 200.0), None);
        assert_eq!(slice_x(100.0, 200.0, 100.0, 0.0), None);
    }

    #[test]
    fn perfect_centered_is_true() {
        assert!(is_perfect(100.0, 200.0, 100.0, 200.0, 4.0));
    }

    #[test]
    fn perfect_threshold_is_inclusive() {
        assert!(is_perfect(100.0, 200.0, 96.0, 200.0, 4.0));
        assert!(!is_perfect(100.0, 200.0, 95.9, 200.0, 4.0));
    }

    #[test]
    fn perfect_uses_centers_not_edges() {
        // Slide 300 wide and top 200 wide, both centered at x = 200.
        assert!(is_perfect(50.0, 300.0, 100.0, 200.0, 0.0));
        assert!(!is_perfect(50.0, 300.0, 101.0, 200.0, 0.0));
    }

    #[test]
    fn perfect_negative_threshold_only_accepts_exact_centers() {
        assert!(is_perfect(100.0, 200.0, 100.0, 200.0, -1.0));
        assert!(!is_perfect(100.0, 200.0, 101.0, 200.0, -1.0));
    }

    #[test]
    fn speed_increments() {
        assert_close(next_speed(500.0, 20.0, 800.0), 520.0);
    }

    #[test]
    fn speed_is_capped() {
        assert_close(next_speed(790.0, 20.0, 800.0), 800.0);
        assert_close(next_speed(800.0, 20.0, 800.0), 800.0);
    }

    #[test]
    fn speed_never_becomes_negative() {
        assert_close(next_speed(0.0, -10.0, 800.0), 0.0);
    }

    #[test]
    fn start_alternates_between_screen_edges() {
        assert_eq!(start_x(1.0, 200.0, 800.0), 0.0);
        assert_eq!(start_x(-1.0, 200.0, 800.0), 600.0);
    }

    #[test]
    fn start_clamps_when_block_is_wider_than_screen() {
        assert_eq!(start_x(-1.0, 900.0, 800.0), 0.0);
    }

    #[test]
    fn camera_target_puts_top_at_seventy_percent() {
        assert_close(camera_target(500.0, 600.0), -80.0);
        assert_close(camera_target(420.0, 600.0), 0.0);
    }

    #[test]
    fn smooth_camera_interpolates_and_snaps() {
        assert_close(smooth_camera(0.0, 100.0, 0.0), 0.0);
        assert_close(smooth_camera(0.0, 100.0, 0.05), 50.0);
        assert_close(smooth_camera(0.0, 100.0, 1.0), 100.0);
    }

    #[test]
    fn perfect_placement_grows_symmetrically() {
        let (x, w) = perfect_placement(100.0, 80.0, 10.0, 200.0);
        assert_eq!(w, 90.0);
        assert_close(x, 95.0);
        assert_close(x + w / 2.0, 140.0);
    }

    #[test]
    fn perfect_placement_caps_at_base_width() {
        let (x, w) = perfect_placement(100.0, 198.0, 10.0, 200.0);
        assert_eq!(w, 200.0);
        assert_close(x, 99.0);
        assert_close(x + w / 2.0, 199.0);
    }

    // ------------------------------------------------------------------
    // Match state
    // ------------------------------------------------------------------

    #[test]
    fn new_match_has_one_base_block_and_full_speed() {
        let ctx = ctx();
        let state = MatchState::new(&ctx);

        assert_eq!(state.tower.len(), 1);
        assert_eq!(state.tower[0].transform.scale.x, ctx.base_block_width);
        assert_eq!(state.sliding.width, ctx.base_block_width);
        assert_eq!(state.sliding.speed, ctx.base_speed);
        assert_eq!(state.sliding.x, 0.0);
        assert!(!state.last_drop_perfect);
        assert_eq!(state.combo, 0);
        assert_eq!(state.camera_punch, 0.0);
        assert_eq!(state.land_pulse, 0.0);
        assert_eq!(state.tower[0].color, block_color(0));
        assert!(!state.juice.screen_shake.is_active());
    }

    #[test]
    fn new_match_starts_with_camera_already_framed() {
        let ctx = ctx();
        let state = MatchState::new(&ctx);
        let base_y = state.tower[0].transform.position.y;
        assert_close(state.camera_y, camera_target(base_y, ctx.screen_h));
    }

    #[test]
    fn centered_drop_stacks_full_width_and_scores() {
        let ctx = ctx();
        let mut app = playing_app();
        let mut state = MatchState::new(&ctx);
        let top_x = state.tower[0].transform.position.x;

        drop_at(&mut state, &mut app, &ctx, top_x);

        assert_eq!(app.score, 1);
        assert_eq!(state.tower.len(), 2);
        assert_eq!(state.tower[1].transform.position.x, top_x);
        assert_eq!(state.tower[1].transform.scale.x, ctx.base_block_width);
        assert!(state.last_drop_perfect);
        assert_eq!(state.sliding.width, ctx.base_block_width);
        assert_eq!(state.sliding.direction, -1.0);
        assert_close(state.sliding.speed, ctx.base_speed + ctx.speed_increment);
        assert_eq!(state.sliding.x, ctx.screen_w - ctx.base_block_width);
        assert_eq!(state.combo, 1);
        assert_close(state.camera_punch, 1.0);
    }

    #[test]
    fn partial_drop_slices_the_overhang() {
        let ctx = ctx();
        let mut app = playing_app();
        let mut state = MatchState::new(&ctx);
        let top_x = state.tower[0].transform.position.x;

        drop_at(&mut state, &mut app, &ctx, top_x + 50.0);

        assert_eq!(app.score, 1);
        assert_eq!(state.tower[1].transform.position.x, top_x + 50.0);
        assert_eq!(
            state.tower[1].transform.scale.x,
            ctx.base_block_width - 50.0
        );
        assert!(!state.last_drop_perfect);
        assert_eq!(state.sliding.width, ctx.base_block_width - 50.0);
        assert_eq!(state.sliding.x, ctx.screen_w - state.sliding.width);
        assert_eq!(state.combo, 0);
        assert_eq!(state.juice.pieces.len(), 1);
        assert_eq!(state.juice.pieces[0].size.x, 50.0);
    }

    #[test]
    fn perfect_threshold_is_recorded_on_the_drop() {
        let ctx = ctx();
        let mut app = playing_app();
        let mut state = MatchState::new(&ctx);
        let top_x = state.tower[0].transform.position.x;

        drop_at(&mut state, &mut app, &ctx, top_x + ctx.perfect_threshold);
        assert!(state.last_drop_perfect);
    }

    #[test]
    fn just_above_the_live_perfect_window_is_not_perfect() {
        let ctx = ctx();
        let mut app = playing_app();
        let mut state = MatchState::new(&ctx);
        let top_x = state.tower[0].transform.position.x;
        let window = perfect_window(state.sliding.speed, ctx.perfect_threshold);

        drop_at(&mut state, &mut app, &ctx, top_x + window + 0.1);
        assert!(!state.last_drop_perfect);
    }

    #[test]
    fn one_frame_of_travel_is_inside_the_perfect_window() {
        let ctx = ctx();
        let mut app = playing_app();
        let mut state = MatchState::new(&ctx);
        let top_x = state.tower[0].transform.position.x;
        let frame_travel = state.sliding.speed / 60.0;

        drop_at(&mut state, &mut app, &ctx, top_x + frame_travel - 0.25);
        assert!(state.last_drop_perfect);
    }

    #[test]
    fn perfect_window_scales_with_speed_and_has_a_floor() {
        assert_close(perfect_window(500.0, 4.0), 500.0 / 60.0);
        assert_close(perfect_window(800.0, 4.0), 800.0 / 60.0);
        assert_close(perfect_window(100.0, 4.0), 4.0);
        assert_close(perfect_window(-500.0, 4.0), 500.0 / 60.0);
    }

    #[test]
    fn perfect_drop_increments_the_combo() {
        let ctx = ctx();
        let mut app = playing_app();
        let mut state = MatchState::new(&ctx);
        let top_x = state.tower[0].transform.position.x;

        drop_at(&mut state, &mut app, &ctx, top_x);
        drop_at(&mut state, &mut app, &ctx, top_x);

        assert_eq!(state.combo, 2);
        assert_eq!(app.score, 2);
        assert!(state.camera_punch > 0.0);
    }

    #[test]
    fn perfect_drop_spawns_feedback() {
        let ctx = ctx();
        let mut app = playing_app();
        let mut state = MatchState::new(&ctx);
        let top_x = state.tower[0].transform.position.x;

        drop_at(&mut state, &mut app, &ctx, top_x);

        assert_eq!(state.juice.floating_texts.len(), 1);
        assert_eq!(state.juice.floating_texts[0].text, "PERFECT");
        assert_eq!(state.juice.shockwaves.len(), 1);
        assert_eq!(state.juice.flashes.len(), 1);
        assert!(state.juice.screen_shake.is_active());
    }

    #[test]
    fn normal_drop_resets_the_combo() {
        let ctx = ctx();
        let mut app = playing_app();
        let mut state = MatchState::new(&ctx);
        let top_x = state.tower[0].transform.position.x;

        drop_at(&mut state, &mut app, &ctx, top_x);
        drop_at(&mut state, &mut app, &ctx, top_x + 50.0);

        assert_eq!(state.combo, 0);
        assert!(!state.last_drop_perfect);
    }

    #[test]
    fn perfect_growth_restores_a_shrunken_block() {
        let ctx = ctx();
        let mut app = playing_app();
        let mut state = MatchState::new(&ctx);
        let top_x = state.tower[0].transform.position.x;

        drop_at(&mut state, &mut app, &ctx, top_x + 50.0);
        let shrunken_x = state.tower[1].transform.position.x;
        let shrunken_w = state.tower[1].transform.scale.x;
        assert_eq!(shrunken_w, ctx.base_block_width - 50.0);

        drop_at(&mut state, &mut app, &ctx, shrunken_x);

        let grown = state.tower.last().unwrap();
        assert_eq!(grown.transform.scale.x, shrunken_w + ctx.perfect_growth);
        assert_close(
            grown.transform.position.x + grown.transform.scale.x / 2.0,
            shrunken_x + shrunken_w / 2.0,
        );
        assert_eq!(state.sliding.width, shrunken_w + ctx.perfect_growth);
    }

    #[test]
    fn perfect_growth_is_capped_to_base_width() {
        let ctx = ctx();
        let mut app = playing_app();
        let mut state = MatchState::new(&ctx);
        let top_x = state.tower[0].transform.position.x;

        // Shrink first, then chain perfect drops until the cap is reached.
        drop_at(&mut state, &mut app, &ctx, top_x + 50.0);
        for _ in 0..30 {
            let x = state.tower.last().unwrap().transform.position.x;
            drop_at(&mut state, &mut app, &ctx, x);
        }

        assert_eq!(
            state.tower.last().unwrap().transform.scale.x,
            ctx.base_block_width
        );
        assert!(state.combo >= 25);
    }

    #[test]
    fn wide_slide_spawns_two_overhang_pieces() {
        let ctx = ctx();
        let mut app = playing_app();
        let mut state = MatchState::new(&ctx);
        let top_x = state.tower[0].transform.position.x;

        state.sliding.width = 300.0;
        drop_at(&mut state, &mut app, &ctx, top_x - 60.0);

        assert!(!state.last_drop_perfect);
        assert_eq!(state.juice.pieces.len(), 2);
        assert_eq!(state.tower[1].transform.scale.x, ctx.base_block_width);
    }

    #[test]
    fn camera_punch_decays_over_time() {
        let ctx = ctx();
        let mut app = playing_app();
        let mut state = MatchState::new(&ctx);
        let top_x = state.tower[0].transform.position.x;

        drop_at(&mut state, &mut app, &ctx, top_x);
        update(&mut state, &mut app, &ctx, 0.1, &Input::default());

        assert_close(state.camera_punch, 1.0 - CAMERA_PUNCH_DECAY * 0.1);
    }

    #[test]
    fn landing_pulse_fires_and_decays() {
        let ctx = ctx();
        let mut app = playing_app();
        let mut state = MatchState::new(&ctx);
        let top_x = state.tower[0].transform.position.x;

        drop_at(&mut state, &mut app, &ctx, top_x);
        assert_close(state.land_pulse, 1.0);

        update(&mut state, &mut app, &ctx, 0.1, &Input::default());
        assert_close(state.land_pulse, 1.0 - LAND_PULSE_DECAY * 0.1);

        update(&mut state, &mut app, &ctx, 1.0, &Input::default());
        assert_eq!(state.land_pulse, 0.0);
    }

    #[test]
    fn effects_keep_ticking_after_game_over() {
        let ctx = ctx();
        let mut app = playing_app();
        let mut state = MatchState::new(&ctx);
        let top_x = state.tower[0].transform.position.x;

        drop_at(&mut state, &mut app, &ctx, top_x);
        drop_at(&mut state, &mut app, &ctx, 0.0);
        assert_eq!(app.state, AppState::GameOver);

        let remaining = state.juice.screen_shake.remaining;
        update(&mut state, &mut app, &ctx, 0.05, &Input::default());

        assert!(state.juice.screen_shake.remaining < remaining);
        assert!(state.camera_punch < 1.0);
    }

    #[test]
    fn miss_resets_combo_and_shakes_hard() {
        let ctx = ctx();
        let mut app = playing_app();
        let mut state = MatchState::new(&ctx);
        let top_x = state.tower[0].transform.position.x;

        drop_at(&mut state, &mut app, &ctx, top_x);
        assert_eq!(state.combo, 1);

        drop_at(&mut state, &mut app, &ctx, 0.0);

        assert_eq!(state.combo, 0);
        assert_eq!(app.state, AppState::GameOver);
        assert!(state.juice.screen_shake.intensity >= 8.0);
    }

    #[test]
    fn missing_the_tower_ends_the_run() {
        let ctx = ctx();
        let mut app = playing_app();
        let mut state = MatchState::new(&ctx);

        drop_at(&mut state, &mut app, &ctx, 0.0);

        assert_eq!(app.state, AppState::GameOver);
        assert_eq!(app.score, 0);
        assert_eq!(state.tower.len(), 1);
        assert!(!state.last_drop_perfect);
    }

    #[test]
    fn touching_the_tower_edge_ends_the_run() {
        let ctx = ctx();
        let mut app = playing_app();
        let mut state = MatchState::new(&ctx);
        let top_x = state.tower[0].transform.position.x;

        state.sliding.width = 100.0;
        drop_at(&mut state, &mut app, &ctx, top_x - 100.0);

        assert_eq!(app.state, AppState::GameOver);
        assert_eq!(state.tower.len(), 1);
    }

    #[test]
    fn direction_alternates_after_each_successful_drop() {
        let ctx = ctx();
        let mut app = playing_app();
        let mut state = MatchState::new(&ctx);
        let top_x = state.tower[0].transform.position.x;

        drop_at(&mut state, &mut app, &ctx, top_x);
        assert_eq!(state.sliding.direction, -1.0);
        assert_eq!(state.sliding.x, ctx.screen_w - state.sliding.width);

        drop_at(&mut state, &mut app, &ctx, top_x);
        assert_eq!(state.sliding.direction, 1.0);
        assert_eq!(state.sliding.x, 0.0);
    }

    #[test]
    fn left_click_drops_the_block() {
        let ctx = ctx();
        let mut app = playing_app();
        let mut state = MatchState::new(&ctx);
        let top_x = state.tower[0].transform.position.x;

        state.sliding.x = top_x;
        update(&mut state, &mut app, &ctx, 0.0, &click());

        assert_eq!(app.score, 1);
        assert_eq!(state.tower.len(), 2);
    }

    #[test]
    fn camera_follows_the_new_top_block() {
        let ctx = ctx();
        let mut app = playing_app();
        let mut state = MatchState::new(&ctx);
        let top_x = state.tower[0].transform.position.x;

        drop_at(&mut state, &mut app, &ctx, top_x);
        let new_top_y = state.tower[1].transform.position.y;
        update(&mut state, &mut app, &ctx, 1.0, &Input::default());

        assert_close(state.camera_y, camera_target(new_top_y, ctx.screen_h));
    }

    #[test]
    fn no_input_is_consumed_after_game_over() {
        let ctx = ctx();
        let mut app = playing_app();
        let mut state = MatchState::new(&ctx);

        drop_at(&mut state, &mut app, &ctx, 0.0);
        let height = state.tower.len();
        let score = app.score;

        update(&mut state, &mut app, &ctx, 0.0, &press(KeyCode::Space));

        assert_eq!(state.tower.len(), height);
        assert_eq!(app.score, score);
    }

    // ------------------------------------------------------------------
    // Audio events
    // ------------------------------------------------------------------

    #[test]
    fn perfect_drop_emits_land_then_perfect() {
        let ctx = ctx();
        let mut app = playing_app();
        let mut state = MatchState::new(&ctx);
        let top_x = state.tower[0].transform.position.x;

        state.sliding.x = top_x;
        let events = update(&mut state, &mut app, &ctx, 0.0, &press(KeyCode::Space));

        assert_eq!(
            events,
            vec![GameEvent::Land, GameEvent::Perfect { combo: 1 }]
        );
    }

    #[test]
    fn perfect_combo_is_reported_for_escalation() {
        let ctx = ctx();
        let mut app = playing_app();
        let mut state = MatchState::new(&ctx);
        let top_x = state.tower[0].transform.position.x;

        state.sliding.x = top_x;
        update(&mut state, &mut app, &ctx, 0.0, &press(KeyCode::Space));
        state.sliding.x = top_x;
        let events = update(&mut state, &mut app, &ctx, 0.0, &press(KeyCode::Space));

        assert_eq!(
            events,
            vec![GameEvent::Land, GameEvent::Perfect { combo: 2 }]
        );
    }

    #[test]
    fn normal_drop_emits_land_and_slice() {
        let ctx = ctx();
        let mut app = playing_app();
        let mut state = MatchState::new(&ctx);
        let top_x = state.tower[0].transform.position.x;

        state.sliding.x = top_x + 50.0;
        let events = update(&mut state, &mut app, &ctx, 0.0, &press(KeyCode::Space));

        assert_eq!(
            events,
            vec![GameEvent::Land, GameEvent::Slice { cut_width: 50.0 }]
        );
    }

    #[test]
    fn miss_emits_miss() {
        let ctx = ctx();
        let mut app = playing_app();
        let mut state = MatchState::new(&ctx);

        state.sliding.x = 0.0;
        let events = update(&mut state, &mut app, &ctx, 0.0, &press(KeyCode::Space));

        assert_eq!(events, vec![GameEvent::Miss]);
        assert_eq!(app.state, AppState::GameOver);
    }

    #[test]
    fn idle_frames_emit_no_events() {
        let ctx = ctx();
        let mut app = playing_app();
        let mut state = MatchState::new(&ctx);

        let events = update(&mut state, &mut app, &ctx, 1.0 / 60.0, &Input::default());

        assert!(events.is_empty());
    }

    #[test]
    fn frames_after_game_over_emit_no_events() {
        let ctx = ctx();
        let mut app = playing_app();
        let mut state = MatchState::new(&ctx);

        drop_at(&mut state, &mut app, &ctx, 0.0);
        let events = update(&mut state, &mut app, &ctx, 0.0, &press(KeyCode::Space));

        assert!(events.is_empty());
    }
}
