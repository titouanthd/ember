//! Asteroids as a node tree (Phase 7 migration).
//!
//! One root node, `AsteroidsGame`, owns the untouched `GameWorld` and runs
//! the systems in the order the old `main.rs` did. It draws stars,
//! asteroids, bullets, the ship and the HUD through the `RenderingServer`
//! (triangles, lines and circles — no engine extension needed: objects
//! wrap by position, as before, and are not drawn twice at the edges).
//! The game is keyboard-only, so there is no UI subtree.

use crate::components::{Asteroid, Bullet, Ship};
use crate::systems::{self, GameContext, GameWorld, ShipInput};
use crate::{GameState, config};
use ember::prelude::*;

/// macroquad's `GREEN`, used by the win screen.
const GREEN: Color = Color::new(0.0, 0.89, 0.19, 1.0);
/// macroquad's `RED`, used by the game-over screen.
const RED: Color = Color::new(0.9, 0.16, 0.22, 1.0);

/// The ship input for this frame (same keys as before).
pub fn ship_input(input: &Input) -> ShipInput {
    ShipInput {
        rotate_left: input.is_key_down(Key::Left) || input.is_key_down(Key::A),
        rotate_right: input.is_key_down(Key::Right) || input.is_key_down(Key::D),
        thrust: input.is_key_down(Key::Up) || input.is_key_down(Key::W),
        shoot: input.is_key_down(Key::Space),
    }
}

pub struct AsteroidsGame {
    name: String,
    pub ctx: GameContext,
    pub world: GameWorld,
    /// Seconds since start, for the invincibility blink (was `get_time`).
    pub time: f32,
}

impl AsteroidsGame {
    pub fn new(ctx: GameContext, world: GameWorld) -> Self {
        Self {
            name: "Asteroids".into(),
            ctx,
            world,
            time: 0.0,
        }
    }

    /// One frame of the old loop, minus Escape and drawing.
    pub fn step(&mut self, input: &Input, dt: f32) {
        let ctx = &self.ctx;
        let world = &mut self.world;
        let ship = ship_input(input);
        match world.state {
            GameState::Start => {
                if input.is_key_pressed(Key::Space) {
                    world.start_new_game(ctx);
                }
            }
            GameState::Playing => {
                systems::update_ship(&mut world.ship, &ship, ctx, dt);
                world.shoot_cooldown.tick(dt);
                if ship.shoot && world.shoot_cooldown.is_ready() {
                    systems::try_shoot(&world.ship, &mut world.bullets, ctx);
                    world
                        .shoot_cooldown
                        .trigger(ctx.bullet_cooldown_ms as f32 / 1000.0);
                }
                systems::update_bullets(&mut world.bullets, ctx, dt);
                systems::update_asteroids(&mut world.asteroids, ctx, dt);
                let destroyed = systems::resolve_bullet_asteroid_collisions(
                    &mut world.bullets,
                    &mut world.asteroids,
                    ctx,
                );
                world.score += destroyed as i32 * 10;
                world.step_ship_asteroid_collision(ctx, dt);
                if world.asteroids.is_empty() && world.state == GameState::Playing {
                    world.on_wave_cleared(ctx);
                }
            }
            GameState::LevelCleared => world.tick_level_cleared(ctx, dt),
            GameState::GameOver | GameState::Win => {
                if input.is_key_pressed(Key::R) {
                    world.reset_to_start(ctx);
                }
            }
        }
    }

    fn draw_stars(&self, r: &mut dyn RenderingServer) {
        let ctx = &self.ctx;
        let mut seed = 12345u32;
        let mut next = || {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            (seed as f32) / (u32::MAX as f32)
        };
        for _ in 0..60 {
            let x = next() * ctx.screen_w;
            let y = next() * ctx.screen_h;
            r.draw_circle(Vec2::new(x, y), 1.0, ctx.star_color);
        }
    }

    fn draw_ship(&self, r: &mut dyn RenderingServer, ship: &Ship, invincible: bool) {
        if invincible && (self.time * 12.0).sin() < 0.0 {
            return;
        }
        let radius = ship.radius();
        let f = ship.forward();
        let right = Vec2::new(-f.y, f.x);
        let p = ship.transform.position;
        let nose = p + f * radius;
        let left_wing = p - f * radius * 0.6 + right * radius * 0.8;
        let right_wing = p - f * radius * 0.6 - right * radius * 0.8;
        let tail = p - f * radius * 0.3;
        r.draw_triangle(nose, left_wing, tail, ship.sprite.color);
        r.draw_triangle(nose, tail, right_wing, ship.sprite.color);
    }

    fn draw_world(&self, r: &mut dyn RenderingServer) {
        let world = &self.world;
        for a in &world.asteroids {
            draw_asteroid(r, a);
        }
        for b in &world.bullets {
            draw_bullet(r, b);
        }
        if matches!(world.state, GameState::Playing | GameState::LevelCleared) {
            self.draw_ship(r, &world.ship, world.invincibility.is_active());
        }
    }

    fn draw_hud(&self, r: &mut dyn RenderingServer) {
        let ctx = &self.ctx;
        let world = &self.world;
        let w = ctx.screen_w;
        let text = ctx.ui_text_color;
        r.draw_text(
            &format!("Score: {}", world.score),
            Vec2::new(12.0, 24.0),
            24.0,
            text,
        );
        r.draw_text(
            &format!("Wave: {}/{}", world.wave.min(ctx.max_waves), ctx.max_waves),
            Vec2::new(w / 2.0 - 60.0, 24.0),
            24.0,
            text,
        );
        r.draw_text(
            &format!("Lives: {}", world.lives),
            Vec2::new(w - 130.0, 24.0),
            24.0,
            text,
        );
        r.draw_text(
            &format!("Best: {}", world.high_score.max(world.score)),
            Vec2::new(w - 130.0, 52.0),
            20.0,
            ctx.star_color,
        );
    }

    fn draw_screens(&self, r: &mut dyn RenderingServer) {
        let ctx = &self.ctx;
        let world = &self.world;
        let cx = ctx.screen_w / 2.0;
        let cy = ctx.screen_h / 2.0;
        let text = ctx.ui_text_color;
        match world.state {
            GameState::Start => {
                centered(r, "ASTEROIDS", cx, cy - 80.0, 90.0, ctx.ship_color);
                let help = "Left / Right to rotate, Up to thrust";
                centered(r, help, cx, cy - 10.0, 24.0, text);
                centered(r, "SPACE to shoot", cx, cy + 25.0, 24.0, text);
                let goal = format!("Survive {} waves to win!", ctx.max_waves);
                centered(r, &goal, cx, cy + 75.0, 26.0, ctx.bullet_color);
                centered(
                    r,
                    "Press SPACE to start",
                    cx,
                    cy + 130.0,
                    28.0,
                    Color::WHITE,
                );
                let best = format!("Best: {}", world.high_score);
                centered(r, &best, cx, cy + 175.0, 22.0, ctx.star_color);
            }
            GameState::LevelCleared => {
                let title = format!("WAVE {} CLEARED!", world.wave);
                centered(r, &title, cx, cy - 20.0, 60.0, ctx.bullet_color);
                let next = format!("Next wave in {:.1}s", world.level_cleared_timer.max(0.0));
                centered(r, &next, cx, cy + 40.0, 26.0, Color::WHITE);
            }
            GameState::GameOver => {
                centered(r, "GAME OVER", cx, cy - 60.0, 80.0, RED);
                let score = format!("Score: {}", world.score);
                centered(r, &score, cx, cy + 10.0, 36.0, Color::WHITE);
                let best = format!("Best: {}", world.high_score);
                centered(r, &best, cx, cy + 55.0, 28.0, ctx.bullet_color);
                centered(r, "Press R to restart", cx, cy + 110.0, 26.0, Color::WHITE);
            }
            GameState::Win => {
                centered(r, "YOU WIN!", cx, cy - 60.0, 90.0, GREEN);
                let score = format!("Final Score: {}", world.score);
                centered(r, &score, cx, cy + 10.0, 40.0, Color::WHITE);
                let best = format!("Best: {}", world.high_score);
                centered(r, &best, cx, cy + 60.0, 28.0, ctx.bullet_color);
                centered(r, "Press R to restart", cx, cy + 120.0, 26.0, Color::WHITE);
            }
            GameState::Playing => {}
        }
    }
}

fn draw_bullet(r: &mut dyn RenderingServer, b: &Bullet) {
    r.draw_circle(b.transform.position, b.radius(), b.sprite.color);
}

/// An 8-sided jittered outline with a translucent fill (unchanged maths).
fn draw_asteroid(r: &mut dyn RenderingServer, a: &Asteroid) {
    let radius = a.radius();
    let n = 8;
    let centre = a.transform.position;
    let points: Vec<Vec2> = (0..n)
        .map(|i| {
            let theta = a.transform.rotation + (i as f32) * std::f32::consts::TAU / (n as f32);
            let jitter = 1.0 + ((i as f32 * 12.9898).sin() * 0.15);
            Vec2::new(
                centre.x + theta.cos() * radius * jitter,
                centre.y + theta.sin() * radius * jitter,
            )
        })
        .collect();
    let c = a.sprite.color;
    let fill = Color::new(c.r, c.g, c.b, 0.25);
    for i in 0..n {
        let (p1, p2) = (points[i], points[(i + 1) % n]);
        r.draw_triangle(centre, p1, p2, fill);
        r.draw_line(p1, p2, 1.5, c);
    }
}

/// Text centred on `x`; `y` is the baseline.
fn centered(r: &mut dyn RenderingServer, text: &str, x: f32, y: f32, size: f32, color: Color) {
    let w = r.measure_text(text, size).width;
    r.draw_text(text, Vec2::new(x - w * 0.5, y), size, color);
}

impl Object for AsteroidsGame {
    fn class_name() -> &'static str {
        "AsteroidsGame"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl NodeName for AsteroidsGame {
    fn name(&self) -> &str {
        &self.name
    }
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

impl Node for AsteroidsGame {
    fn ready(&mut self, ctx: &mut NodeContext) {
        ctx.render().set_clear_color(self.ctx.bg_color);
    }

    fn process(&mut self, dt: f32, ctx: &mut NodeContext) {
        let input = ctx.input().clone();
        if input.is_key_pressed(Key::Escape) {
            ctx.quit();
            return;
        }
        self.time += dt;
        self.step(&input, dt);
    }

    fn draw(&self, ctx: &mut DrawContext) {
        let r = ctx.render();
        self.draw_stars(r);
        self.draw_world(r);
        self.draw_hud(r);
        self.draw_screens(r);
    }
}

/// The game's tree: a single `AsteroidsGame` root.
pub fn build_tree(ctx: GameContext, world: GameWorld) -> NodeTree {
    NodeTree::with_root(AsteroidsGame::new(ctx, world))
}

/// A seed from the clock, so each run spawns differently.
fn clock_seed() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u32)
        .unwrap_or(1)
}

/// Open the window and play.
pub fn run() {
    systems::seed_rng(clock_seed());
    let ctx = config::load_config();
    let (w, h) = (ctx.screen_w, ctx.screen_h);
    let world = GameWorld::new(&ctx);
    let game = SceneLoop::new(
        build_tree(ctx, world),
        MacroquadRenderer::new(),
        MacroquadAudio::new(),
        Rect::new(0.0, 0.0, w, h),
    );
    run_window("Asteroids - Powered by Ember", w as i32, h as i32, game);
}
