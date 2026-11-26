//! Breakout as a node tree (Phase 7 migration).
//!
//! One root node, `BreakoutGame`, owns the untouched `BreakoutWorld`
//! plus the per-run `GameContext` (the ball speeds up 10 % per level, so
//! the context is session state). It reads input in `process` and draws
//! bricks, paddle, ball and text through the `RenderingServer`. The game
//! is keyboard-only, so there is no UI subtree.

use crate::systems::{self, UpdateEvent};
use crate::{BreakoutWorld, GameContext, GameState, config};
use ember::prelude::*;

/// Number of levels in the campaign (was a `main.rs` constant).
pub const MAX_LEVELS: usize = 3;
const BRICK_OUTLINE: Color = Color::new(0.2, 0.2, 0.2, 1.0);
// macroquad's named colours, used by the old screens (not pure RGB).
const GRAY: Color = Color::new(0.51, 0.51, 0.51, 1.0);
const YELLOW: Color = Color::new(0.99, 0.98, 0.0, 1.0);
const RED: Color = Color::new(0.9, 0.16, 0.22, 1.0);
const GREEN: Color = Color::new(0.0, 0.89, 0.19, 1.0);

pub struct BreakoutGame {
    name: String,
    pub ctx: GameContext,
    pub world: BreakoutWorld,
}

impl BreakoutGame {
    /// Level 0 with the loaded config, as the old `main` did.
    pub fn new(ctx: GameContext) -> Self {
        let bricks = systems::load_level(0, &ctx);
        let world = BreakoutWorld::new(bricks, &ctx, MAX_LEVELS);
        Self {
            name: "Breakout".into(),
            ctx,
            world,
        }
    }

    /// One frame of the old loop, minus Escape and drawing.
    pub fn step(&mut self, input: &Input, dt: f32) {
        let world = &mut self.world;
        if world.state == GameState::Playing {
            if input.is_key_down(Key::Left) || input.is_key_down(Key::A) {
                world.paddle.move_left(dt);
            }
            if input.is_key_down(Key::Right) || input.is_key_down(Key::D) {
                world.paddle.move_right(dt, self.ctx.screen_w);
            }
        }
        match world.state {
            GameState::Start => {
                if input.is_key_pressed(Key::Space) {
                    world.state = GameState::Playing;
                    world.reset_ball(&self.ctx);
                }
            }
            GameState::Playing => match systems::update(world, &self.ctx, dt) {
                UpdateEvent::None => {}
                UpdateEvent::BallLost => world.on_ball_lost(&self.ctx),
                UpdateEvent::LevelCleared => {
                    let next_level = world.current_level + 1;
                    if next_level >= world.max_levels {
                        world.state = GameState::Win;
                    } else {
                        self.ctx.ball_speed *= 1.1;
                        let bricks = systems::load_level(next_level, &self.ctx);
                        world.advance_level(bricks, &self.ctx);
                    }
                }
            },
            GameState::GameOver | GameState::Win => {
                if input.is_key_pressed(Key::R) {
                    self.ctx = config::load_config();
                    let bricks = systems::load_level(0, &self.ctx);
                    world.start_new_game(bricks, &self.ctx);
                    world.state = GameState::Start;
                }
            }
            GameState::LevelCleared => {}
        }
    }

    fn draw_world(&self, r: &mut dyn RenderingServer) {
        let world = &self.world;
        for brick in world.bricks.iter().filter(|b| b.is_alive()) {
            let rect = rect_of(&brick.transform);
            r.draw_rect(rect, brick.sprite.color);
            r.draw_rect_outline(rect, 1.0, BRICK_OUTLINE);
        }
        if world.state != GameState::Start {
            r.draw_rect(rect_of(&world.paddle.transform), world.paddle.sprite.color);
        }
        if world.state != GameState::Start && world.state != GameState::GameOver {
            r.draw_rect(rect_of(&world.ball.transform), world.ball.sprite.color);
        }
    }

    fn draw_text(&self, r: &mut dyn RenderingServer) {
        let world = &self.world;
        let cx = self.ctx.screen_w / 2.0;
        centered(
            r,
            &format!(
                "Score: {}  |  Lives: {}  |  Level: {}/{}",
                world.score,
                world.lives,
                world.current_level + 1,
                world.max_levels,
            ),
            cx,
            30.0,
            28.0,
            Color::WHITE,
        );
        match world.state {
            GameState::Start => {
                centered(r, "BREAKOUT", cx, 180.0, 100.0, YELLOW);
                centered(r, "Press SPACE to start", cx, 300.0, 40.0, Color::WHITE);
                centered(r, "Use Left/Right or A/D to move", cx, 360.0, 30.0, GRAY);
                let alive = world.bricks.iter().filter(|b| b.is_alive()).count();
                centered(
                    r,
                    &format!(
                        "Level {} - {} bricks to destroy!",
                        world.current_level + 1,
                        alive
                    ),
                    cx,
                    420.0,
                    30.0,
                    GRAY,
                );
            }
            GameState::GameOver => {
                centered(r, "GAME OVER", cx, 250.0, 80.0, RED);
                let score = format!("Final Score: {}", world.score);
                centered(r, &score, cx, 320.0, 40.0, Color::WHITE);
                centered(r, "Press R to restart", cx, 380.0, 40.0, Color::WHITE);
            }
            GameState::Win => {
                centered(r, "YOU WIN !", cx, 200.0, 80.0, GREEN);
                centered(r, "Congratulations !", cx, 270.0, 50.0, YELLOW);
                let score = format!("Final Score: {}", world.score);
                centered(r, &score, cx, 330.0, 40.0, Color::WHITE);
                centered(r, "Press R to restart", cx, 390.0, 40.0, Color::WHITE);
            }
            GameState::Playing | GameState::LevelCleared => {}
        }
        r.draw_text(
            &format!(
                "Ember Engine v0.2 (Breakout - Level {})",
                world.current_level + 1
            ),
            Vec2::new(20.0, self.ctx.screen_h - 20.0),
            25.0,
            GRAY,
        );
    }
}

/// Screen rect of a transform (position = top-left, scale = size).
fn rect_of(t: &ember_stdlib::Transform) -> Rect {
    Rect::new(t.position.x, t.position.y, t.scale.x, t.scale.y)
}

/// Text centred on `x`; `y` is the baseline.
fn centered(r: &mut dyn RenderingServer, text: &str, x: f32, y: f32, size: f32, color: Color) {
    let w = r.measure_text(text, size).width;
    r.draw_text(text, Vec2::new(x - w * 0.5, y), size, color);
}

impl Object for BreakoutGame {
    fn class_name() -> &'static str {
        "BreakoutGame"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl NodeName for BreakoutGame {
    fn name(&self) -> &str {
        &self.name
    }
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

impl Node for BreakoutGame {
    fn ready(&mut self, ctx: &mut NodeContext) {
        ctx.render().set_clear_color(Color::BLACK);
    }

    fn process(&mut self, dt: f32, ctx: &mut NodeContext) {
        let input = ctx.input().clone();
        if input.is_key_pressed(Key::Escape) {
            ctx.quit();
            return;
        }
        self.step(&input, dt);
    }

    fn draw(&self, ctx: &mut DrawContext) {
        let r = ctx.render();
        self.draw_world(r);
        self.draw_text(r);
    }
}

/// The game's tree: a single `BreakoutGame` root.
pub fn build_tree(ctx: GameContext) -> NodeTree {
    NodeTree::with_root(BreakoutGame::new(ctx))
}

/// Open the window and play.
pub fn run() {
    let ctx = config::load_config();
    let (w, h) = (ctx.screen_w, ctx.screen_h);
    let game = SceneLoop::new(
        build_tree(ctx),
        MacroquadRenderer::new(),
        MacroquadAudio::new(),
        Rect::new(0.0, 0.0, w, h),
    );
    run_window("Breakout - Powered by Ember", w as i32, h as i32, game);
}
