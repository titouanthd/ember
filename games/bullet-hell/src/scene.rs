//! Bullet Hell as a node tree (Phase 7 migration).
//!
//! One root node, `BulletHellGame`, owns the untouched `World`. It reads
//! the ship input in `process`, calls `systems::update` as the old loop
//! did, and draws HUD, bullets, particles, enemies, the ship and the
//! overlays through the `RenderingServer`. Keyboard only, so there is no
//! UI subtree. As before, there is no quit key (close the window).

use crate::GameState;
use crate::config::{GameContext, load_config};
use crate::systems::{ShipInput, World, reset, update};
use ember::prelude::*;

/// Frame time cap, as in the macroquad version.
pub const MAX_FRAME_DT: f32 = 1.0 / 30.0;
const HUD_BG: Color = Color::new(0.0, 0.0, 0.0, 0.6);
const SUBTLE: Color = Color::new(0.6, 0.65, 0.75, 1.0);
const SUBTLER: Color = Color::new(0.5, 0.55, 0.7, 1.0);
const STATS: Color = Color::new(0.7, 0.75, 0.85, 1.0);
const WAVE_CLEAR: Color = Color::new(0.6, 1.0, 0.7, 1.0);
const GAME_OVER: Color = Color::new(1.0, 0.3, 0.4, 1.0);
const YOU_WIN: Color = Color::new(0.5, 1.0, 0.6, 1.0);

/// The ship input for this frame (same keys as before).
pub fn ship_input(input: &Input) -> ShipInput {
    let down = |a: Key, b: Key| input.is_key_down(a) || input.is_key_down(b);
    let mut dx = 0.0;
    let mut dy = 0.0;
    if down(Key::Left, Key::A) {
        dx -= 1.0;
    }
    if down(Key::Right, Key::D) {
        dx += 1.0;
    }
    if down(Key::Up, Key::W) {
        dy -= 1.0;
    }
    if down(Key::Down, Key::S) {
        dy += 1.0;
    }
    ShipInput {
        dx,
        dy,
        focus: down(Key::LeftShift, Key::RightShift),
        fire: input.is_key_down(Key::Space),
    }
}

pub struct BulletHellGame {
    name: String,
    pub ctx: GameContext,
    pub world: World,
}

impl BulletHellGame {
    pub fn new(ctx: GameContext, world: World) -> Self {
        Self {
            name: "BulletHell".into(),
            ctx,
            world,
        }
    }

    /// One frame of the old loop, minus drawing.
    pub fn step(&mut self, input: &Input, dt: f32) {
        let ship = ship_input(input);
        if input.is_key_pressed(Key::R) {
            reset(&mut self.world, &self.ctx);
        }
        if self.world.state == GameState::Start
            && (input.is_key_pressed(Key::Space) || input.is_key_pressed(Key::Enter))
        {
            self.world.start_run(&self.ctx);
        }
        update(&mut self.world, ship, &self.ctx, dt);
    }

    fn draw_hud(&self, r: &mut dyn RenderingServer) {
        let ctx = &self.ctx;
        let world = &self.world;
        r.draw_rect(Rect::new(0.0, 0.0, ctx.window_w, ctx.hud_h), HUD_BG);
        let label = format!(
            "SCORE {:06}   x{:.1}   GRAZE {}   LIVES {}   WAVE {}/{}",
            world.score,
            world.multiplier,
            world.graze_count,
            world.lives.max(0),
            (world.wave + 1).min(world.waves.len() as u32),
            world.waves.len()
        );
        r.draw_text(&label, Vec2::new(8.0, ctx.hud_h - 6.0), 16.0, ctx.color_hud);
    }

    fn draw_playfield(&self, r: &mut dyn RenderingServer) {
        let ctx = &self.ctx;
        let world = &self.world;
        // The ship is only drawn when actively playing or between waves.
        if !matches!(world.state, GameState::Playing | GameState::LevelCleared) {
            return;
        }
        // Screen shake: offset applied to every playfield draw.
        let shake = if world.shake_magnitude > 0.0 {
            Vec2::new(
                (world.now * 60.0).sin() * world.shake_magnitude,
                (world.now * 71.0).cos() * world.shake_magnitude,
            )
        } else {
            Vec2::ZERO
        };
        let offset = |p: Vec2| ctx.to_window(p) + shake;

        // Bullets (bottom layer), with a faint outer ring.
        for b in world
            .player_bullets
            .iter()
            .chain(world.enemy_bullets.iter())
        {
            let wp = offset(b.pos);
            r.draw_circle(wp, b.radius, b.color);
            let ring = Color::new(b.color.r, b.color.g, b.color.b, 0.35);
            r.draw_circle_outline(wp, b.radius + 1.5, 1.0, ring);
        }

        for p in &world.particles {
            let alpha = (p.ttl / p.max_ttl).clamp(0.0, 1.0);
            let c = Color::new(p.color.r, p.color.g, p.color.b, alpha * p.color.a);
            r.draw_circle(offset(p.pos), p.radius, c);
        }

        // Enemies, with a darker inner core.
        for e in &world.enemies {
            let wp = offset(e.center);
            let color = e.render_color();
            r.draw_circle(wp, e.radius, color);
            let core = Color::new(color.r * 0.4, color.g * 0.4, color.b * 0.4, 1.0);
            r.draw_circle(wp, (e.radius * 0.55).max(1.0), core);
        }

        // Player (top layer).
        if world.player.alive {
            let p = &world.player;
            let wp = offset(p.transform.position);
            let radius = ctx.player_radius;
            r.draw_triangle(
                Vec2::new(wp.x, wp.y - radius),
                Vec2::new(wp.x - radius * 0.8, wp.y + radius * 0.6),
                Vec2::new(wp.x + radius * 0.8, wp.y + radius * 0.6),
                p.color(ctx, world.now),
            );
            // Focus mode: show hitbox + faint graze ring.
            if p.focus {
                let h = ctx.color_hitbox;
                r.draw_circle_outline(wp, ctx.player_hitbox_radius, 1.0, h);
                let graze = Color::new(h.r, h.g, h.b, 0.25);
                r.draw_circle_outline(wp, ctx.player_graze_radius, 1.0, graze);
            }
        }
    }

    fn draw_overlays(&self, r: &mut dyn RenderingServer) {
        let ctx = &self.ctx;
        let world = &self.world;
        let cx = ctx.window_w * 0.5;
        let cy = ctx.hud_h + ctx.playfield_h() * 0.5;
        match world.state {
            GameState::Start => {
                shadowed(r, "BULLET HELL", cx, cy - 20.0, 48.0, Color::WHITE);
                centered(
                    r,
                    "PRESS SPACE TO START",
                    cx,
                    cy + 30.0,
                    24.0,
                    ctx.color_hud,
                );
                let help = "ARROWS MOVE  ·  SHIFT FOCUS  ·  SPACE FIRE  ·  R RESTART";
                centered(r, help, cx, cy + 70.0, 16.0, SUBTLE);
                let tip = "GRAZE BULLETS TO BUILD YOUR MULTIPLIER";
                centered(r, tip, cx, cy + 100.0, 14.0, SUBTLER);
            }
            GameState::LevelCleared => {
                let title = format!("WAVE {} CLEAR", world.wave + 1);
                shadowed(r, &title, cx, cy, 40.0, WAVE_CLEAR);
            }
            GameState::GameOver => {
                shadowed(r, "GAME OVER", cx, cy - 40.0, 48.0, GAME_OVER);
                let score = format!("SCORE {}", world.score);
                centered(r, &score, cx, cy + 20.0, 24.0, Color::WHITE);
                let stats = format!(
                    "GRAZED {}   ·   MAX x{:.1}",
                    world.graze_count, world.multiplier
                );
                centered(r, &stats, cx, cy + 50.0, 16.0, STATS);
                centered(r, "R TO RESTART", cx, cy + 90.0, 18.0, ctx.color_hud);
            }
            GameState::Win => {
                shadowed(r, "YOU WIN", cx, cy - 40.0, 48.0, YOU_WIN);
                let score = format!("FINAL SCORE {}", world.score);
                centered(r, &score, cx, cy + 20.0, 24.0, Color::WHITE);
                let stats = format!(
                    "GRAZED {}   ·   MULTIPLIER x{:.1}",
                    world.graze_count, world.multiplier
                );
                centered(r, &stats, cx, cy + 50.0, 16.0, STATS);
                centered(r, "R TO PLAY AGAIN", cx, cy + 90.0, 18.0, ctx.color_hud);
            }
            GameState::Playing => {}
        }
    }
}

/// Text centred on `x`; `y` is the baseline.
fn centered(r: &mut dyn RenderingServer, text: &str, x: f32, y: f32, size: f32, color: Color) {
    let w = r.measure_text(text, size).width;
    r.draw_text(text, Vec2::new(x - w * 0.5, y), size, color);
}

/// `centered` over a black shadow offset by 2 px (was
/// `draw_centered_shadowed(.., BLACK, 2.0)`).
fn shadowed(r: &mut dyn RenderingServer, text: &str, x: f32, y: f32, size: f32, color: Color) {
    centered(r, text, x + 2.0, y + 2.0, size, Color::BLACK);
    centered(r, text, x, y, size, color);
}

impl Object for BulletHellGame {
    fn class_name() -> &'static str {
        "BulletHellGame"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl NodeName for BulletHellGame {
    fn name(&self) -> &str {
        &self.name
    }
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

impl Node for BulletHellGame {
    fn ready(&mut self, ctx: &mut NodeContext) {
        ctx.render().set_clear_color(self.ctx.color_bg);
    }

    fn process(&mut self, dt: f32, ctx: &mut NodeContext) {
        let input = ctx.input().clone();
        self.step(&input, dt.min(MAX_FRAME_DT));
    }

    fn draw(&self, ctx: &mut DrawContext) {
        let r = ctx.render();
        self.draw_hud(r);
        self.draw_playfield(r);
        self.draw_overlays(r);
    }
}

/// The game's tree: a single `BulletHellGame` root.
pub fn build_tree(ctx: GameContext, world: World) -> NodeTree {
    NodeTree::with_root(BulletHellGame::new(ctx, world))
}

/// Open the window and play.
pub fn run() {
    let ctx = load_config();
    let (w, h) = (ctx.window_w, ctx.window_h);
    let world = World::new(&ctx);
    let game = SceneLoop::new(
        build_tree(ctx, world),
        MacroquadRenderer::new(),
        MacroquadAudio::new(),
        Rect::new(0.0, 0.0, w, h),
    );
    run_window("Bullet Hell", w as i32, h as i32, game);
}
