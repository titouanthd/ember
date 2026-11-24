//! Snake as a node tree (Phase 7 migration).
//!
//! One root node, `SnakeGame`, owns the untouched `SnakeWorld` plus the
//! session state the macroquad `main.rs` kept in locals (best score,
//! current level, level-cleared timer). It reads input in `process` and
//! draws the grid, walls, food, snake, HUD and overlays through the
//! `RenderingServer`.

use crate::GameState;
use crate::components::{Cell, Direction, Food, Snake, Wall};
use crate::config;
use crate::systems::{self, GameContext, SnakeWorld, TickEvent};
use ember::prelude::*;

pub const MAX_LEVELS: usize = 3;
/// Seconds the "LEVEL CLEARED!" banner stays before the next level.
pub const LEVEL_CLEARED_SECS: f32 = 1.5;

// macroquad's named colours, as the old renderer used them.
const WHITE: Color = Color::new(1.0, 1.0, 1.0, 1.0);
const GRAY: Color = Color::new(0.51, 0.51, 0.51, 1.0);
const GREEN: Color = Color::new(0.0, 0.89, 0.19, 1.0);
const RED: Color = Color::new(0.9, 0.16, 0.22, 1.0);

pub struct SnakeGame {
    name: String,
    pub ctx: GameContext,
    pub world: SnakeWorld,
    pub best: i32,
    pub current_level: usize,
    /// Seconds spent in `LevelCleared`; `None` outside it.
    pub cleared_for: Option<f32>,
}

impl SnakeGame {
    pub fn new(ctx: GameContext) -> Self {
        let level = systems::load_level(0).expect("cannot load levels/level1.ron");
        let world = SnakeWorld::from_level(&level, &ctx);
        Self {
            name: "Snake".into(),
            ctx,
            world,
            best: 0,
            current_level: 0,
            cleared_for: None,
        }
    }

    fn load(&mut self, index: usize) {
        let level = systems::load_level(index).expect("cannot load the level");
        self.world.apply_level(&level, &self.ctx);
    }

    /// One frame of the old main loop, minus drawing.
    fn step(&mut self, dt: f32, input: &Input) {
        if self.world.state == GameState::Playing {
            for (keys, dir) in [
                ([Key::Up, Key::W], Direction::Up),
                ([Key::Down, Key::S], Direction::Down),
                ([Key::Left, Key::A], Direction::Left),
                ([Key::Right, Key::D], Direction::Right),
            ] {
                if keys.iter().any(|&k| input.is_key_pressed(k)) {
                    self.world.snake.request_direction(dir);
                }
            }
        }

        match self.world.state {
            GameState::Start => {
                if input.is_key_pressed(Key::Space) {
                    self.load(self.current_level);
                    self.world.state = GameState::Playing;
                }
            }
            GameState::Playing => {
                if let TickEvent::Died = systems::update(&mut self.world, &self.ctx, dt) {
                    self.best = self.best.max(self.world.score);
                }
            }
            GameState::LevelCleared => {
                let t = self.cleared_for.unwrap_or(0.0) + dt;
                self.cleared_for = Some(t);
                if t > LEVEL_CLEARED_SECS {
                    self.current_level += 1;
                    if self.current_level >= MAX_LEVELS {
                        self.world.state = GameState::Win;
                    } else {
                        self.load(self.current_level);
                        self.cleared_for = None;
                        self.world.state = GameState::Playing;
                    }
                }
            }
            GameState::GameOver => {
                if input.is_key_pressed(Key::R) {
                    self.best = self.best.max(self.world.score);
                    self.world.score = 0;
                    self.current_level = 0;
                    self.world.state = GameState::Start;
                }
            }
            GameState::Win => {
                if input.is_key_pressed(Key::R) {
                    self.world.score = 0;
                    self.current_level = 0;
                    self.world.state = GameState::Start;
                }
            }
        }
    }

    fn cell_rect(&self, cell: Cell, pad: f32) -> Rect {
        let s = self.ctx.cell_size;
        Rect::new(
            cell.x as f32 * s + pad,
            cell.y as f32 * s + pad,
            s - 2.0 * pad,
            s - 2.0 * pad,
        )
    }

    fn draw_grid(&self, r: &mut dyn RenderingServer) {
        let (w, h) = (self.ctx.screen_w(), self.ctx.screen_h());
        for x in 0..=self.ctx.grid_cols {
            let px = x as f32 * self.ctx.cell_size;
            r.draw_line(
                Vec2::new(px, 0.0),
                Vec2::new(px, h),
                1.0,
                self.ctx.grid_color,
            );
        }
        for y in 0..=self.ctx.grid_rows {
            let py = y as f32 * self.ctx.cell_size;
            r.draw_line(
                Vec2::new(0.0, py),
                Vec2::new(w, py),
                1.0,
                self.ctx.grid_color,
            );
        }
    }

    fn draw_world(&self, r: &mut dyn RenderingServer) {
        let walls: &[Wall] = &self.world.walls;
        for w in walls {
            r.draw_rect(self.cell_rect(w.cell, 1.0), w.color);
        }
        let food: &Food = &self.world.food;
        r.draw_rect(self.cell_rect(food.cell, 4.0), food.color);
        let snake: &Snake = &self.world.snake;
        for (i, cell) in snake.body.iter().enumerate() {
            let color = if i == 0 {
                snake.color_head
            } else {
                snake.color_body
            };
            r.draw_rect(self.cell_rect(*cell, 2.0), color);
        }
    }

    fn draw_hud(&self, r: &mut dyn RenderingServer) {
        let w = self.ctx.screen_w();
        r.draw_text(
            &format!("Score: {}", self.world.score),
            Vec2::new(12.0, 24.0),
            28.0,
            WHITE,
        );
        r.draw_text(
            &format!("Best: {}", self.best),
            Vec2::new(w - 130.0, 24.0),
            28.0,
            WHITE,
        );
        let speed = self.world.snake_state.food_eaten / self.ctx.food_per_speedup;
        r.draw_text(
            &format!("Speed: {}", speed + 1),
            Vec2::new(w - 130.0, 52.0),
            24.0,
            GRAY,
        );
        r.draw_text(
            &format!(
                "Level {}/{}  -  {}/{}",
                self.current_level + 1,
                MAX_LEVELS,
                self.world.snake_state.food_eaten,
                self.world.snake_state.target_score,
            ),
            Vec2::new(w / 2.0 - 120.0, 24.0),
            24.0,
            WHITE,
        );
    }

    /// `(text, dy from the centre, size, colour)` lines of the overlay.
    pub fn overlay(&self) -> Vec<(String, f32, f32, Color)> {
        let l = |t: &str, dy, size, c| (t.to_string(), dy, size, c);
        match self.world.state {
            GameState::Start => vec![
                l("SNAKE", -40.0, 80.0, GREEN),
                l("Press SPACE to start", 20.0, 30.0, WHITE),
                l("Arrows / WASD to move", 60.0, 24.0, GRAY),
                (
                    format!(
                        "Level {} - {} walls",
                        self.current_level + 1,
                        self.world.walls.len()
                    ),
                    100.0,
                    20.0,
                    GRAY,
                ),
            ],
            GameState::LevelCleared => vec![
                l("LEVEL CLEARED!", 0.0, 60.0, GREEN),
                l("Next level...", 50.0, 30.0, WHITE),
            ],
            GameState::GameOver => vec![
                l("GAME OVER", 0.0, 70.0, RED),
                (
                    format!("Score: {}  |  Best: {}", self.world.score, self.best),
                    50.0,
                    32.0,
                    WHITE,
                ),
                l("Press R to restart", 100.0, 30.0, WHITE),
            ],
            GameState::Win => vec![
                l("YOU WIN!", 0.0, 80.0, GREEN),
                (
                    format!("Final Score: {}", self.world.score),
                    60.0,
                    40.0,
                    WHITE,
                ),
                l("Press R to restart", 120.0, 30.0, WHITE),
            ],
            GameState::Playing => Vec::new(),
        }
    }
}

impl Object for SnakeGame {
    fn class_name() -> &'static str {
        "SnakeGame"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl NodeName for SnakeGame {
    fn name(&self) -> &str {
        &self.name
    }
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

impl Node for SnakeGame {
    fn ready(&mut self, ctx: &mut NodeContext) {
        ctx.render().set_clear_color(self.ctx.bg_color);
    }

    fn process(&mut self, dt: f32, ctx: &mut NodeContext) {
        if ctx.input().is_key_pressed(Key::Escape) {
            ctx.quit();
            return;
        }
        let input = ctx.input().clone();
        self.step(dt, &input);
    }

    fn draw(&self, ctx: &mut DrawContext) {
        let r = ctx.render();
        self.draw_grid(r);
        if self.world.state != GameState::Start {
            self.draw_world(r);
        }
        self.draw_hud(r);
        let (cx, cy) = (self.ctx.screen_w() / 2.0, self.ctx.screen_h() / 2.0);
        for (text, dy, size, color) in self.overlay() {
            // Centred on x, `cy + dy` is the baseline (like draw_centered).
            let m = r.measure_text(&text, size);
            r.draw_text(&text, Vec2::new(cx - m.width / 2.0, cy + dy), size, color);
        }
    }
}

/// The game's tree: a single `SnakeGame` root.
pub fn build_tree(ctx: GameContext) -> NodeTree {
    NodeTree::with_root(SnakeGame::new(ctx))
}

/// Open the window and play.
pub fn run() {
    let ctx = config::load_config();
    let (w, h) = (ctx.screen_w(), ctx.screen_h());
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u32)
        .unwrap_or(1);
    systems::seed_food(seed);
    let game = SceneLoop::new(
        build_tree(ctx),
        MacroquadRenderer::new(),
        MacroquadAudio::new(),
        Rect::new(0.0, 0.0, w, h),
    );
    run_window("Snake - Powered by Ember", w as i32, h as i32, game);
}
