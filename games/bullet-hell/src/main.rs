//! Window, input, render loop. All game logic lives in `systems.rs`.

use bullet_hell::config::{load_config, GameContext};
use bullet_hell::systems::{reset, update, ShipInput, World};
use bullet_hell::GameState;

use glam::Vec2;
use macroquad::prelude::*;

fn window_conf() -> Conf {
    let ctx = load_config();
    Conf {
        window_title: "Bullet Hell".to_owned(),
        window_width: ctx.window_w as i32,
        window_height: ctx.window_h as i32,
        window_resizable: false,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let ctx = load_config();
    let mut world = World::new(&ctx);

    loop {
        let dt = get_frame_time().min(1.0 / 30.0);

        // --- Input ---
        let input = read_input();
        if is_key_pressed(KeyCode::R) {
            reset(&mut world, &ctx);
        }
        match world.state {
            GameState::Start => {
                if is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter) {
                    world.start_run(&ctx);
                }
            }
            GameState::GameOver | GameState::Win => {
                // R handled above.
            }
            _ => {}
        }

        // --- Update ---
        update(&mut world, input, &ctx, dt);

        // --- Render ---
        clear_background(ctx.color_bg);
        render_hud(&world, &ctx);
        render_playfield(&world, &ctx);
        render_overlays(&world, &ctx);

        next_frame().await;
    }
}

fn read_input() -> ShipInput {
    let mut dx = 0.0;
    let mut dy = 0.0;
    if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
        dx -= 1.0;
    }
    if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
        dx += 1.0;
    }
    if is_key_down(KeyCode::Up) || is_key_down(KeyCode::W) {
        dy -= 1.0;
    }
    if is_key_down(KeyCode::Down) || is_key_down(KeyCode::S) {
        dy += 1.0;
    }
    ShipInput {
        dx,
        dy,
        focus: is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift),
        fire: is_key_down(KeyCode::Space),
    }
}

// ---------------------------------------------------------------------------
// HUD
// ---------------------------------------------------------------------------

fn render_hud(world: &World, ctx: &GameContext) {
    // HUD strip background.
    draw_rectangle(
        0.0,
        0.0,
        ctx.window_w,
        ctx.hud_h,
        Color::new(0.0, 0.0, 0.0, 0.6),
    );

    let label = format!(
        "SCORE {:06}   x{:.1}   GRAZE {}   LIVES {}   WAVE {}/{}",
        world.score,
        world.multiplier,
        world.graze_count,
        world.lives.max(0),
        (world.wave + 1).min(world.waves.len() as u32),
        world.waves.len()
    );
    draw_text(&label, 8.0, ctx.hud_h - 6.0, 16.0, ctx.color_hud);
}

// ---------------------------------------------------------------------------
// Playfield
// ---------------------------------------------------------------------------

fn render_playfield(world: &World, ctx: &GameContext) {
    // The ship is only drawn when actively playing or between waves.
    let show_gameplay = matches!(
        world.state,
        GameState::Playing | GameState::LevelCleared
    );
    if !show_gameplay {
        return;
    }

    // Screen shake: offset applied to every playfield draw.
    let shake = if world.shake_magnitude > 0.0 {
        let a = (world.now * 60.0).sin() * world.shake_magnitude;
        let b = (world.now * 71.0).cos() * world.shake_magnitude;
        Vec2::new(a, b)
    } else {
        Vec2::ZERO
    };
    let offset = |p: Vec2| -> Vec2 {
        let w = ctx.to_window(p);
        Vec2::new(w.x + shake.x, w.y + shake.y)
    };

    // --- Bullets (bottom layer) ---
    for b in world.player_bullets.iter().chain(world.enemy_bullets.iter()) {
        let wp = offset(b.pos);
        draw_circle(wp.x, wp.y, b.radius, b.color);
        // Faint outer ring for readability on dark backgrounds.
        draw_circle_lines(
            wp.x,
            wp.y,
            b.radius + 1.5,
            1.0,
            Color::new(b.color.r, b.color.g, b.color.b, 0.35),
        );
    }

    // --- Particles ---
    for p in &world.particles {
        let wp = offset(p.pos);
        let alpha = (p.ttl / p.max_ttl).clamp(0.0, 1.0);
        draw_circle(
            wp.x,
            wp.y,
            p.radius,
            Color::new(p.color.r, p.color.g, p.color.b, alpha * p.color.a),
        );
    }

    // --- Enemies ---
    for e in &world.enemies {
        let wp = offset(e.center);
        let color = e.render_color();
        draw_circle(wp.x, wp.y, e.radius, color);
        // Inner darker core for depth.
        draw_circle(
            wp.x,
            wp.y,
            (e.radius * 0.55).max(1.0),
            Color::new(color.r * 0.4, color.g * 0.4, color.b * 0.4, 1.0),
        );
    }

    // --- Player (top layer) ---
    if world.player.alive {
        let p = &world.player;
        let wp = offset(p.transform.position);
        let color = p.color(ctx, world.now);

        let r = ctx.player_radius;
        let a = Vec2::new(wp.x, wp.y - r);
        let b = Vec2::new(wp.x - r * 0.8, wp.y + r * 0.6);
        let c = Vec2::new(wp.x + r * 0.8, wp.y + r * 0.6);
        draw_triangle(a, b, c, color);

        // Focus mode: show hitbox + faint graze ring.
        if p.focus {
            draw_circle_lines(wp.x, wp.y, ctx.player_hitbox_radius, 1.0, ctx.color_hitbox);
            draw_circle_lines(
                wp.x,
                wp.y,
                ctx.player_graze_radius,
                1.0,
                Color::new(
                    ctx.color_hitbox.r,
                    ctx.color_hitbox.g,
                    ctx.color_hitbox.b,
                    0.25,
                ),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Overlays (title, wave clear banner, end screens)
// ---------------------------------------------------------------------------

fn render_overlays(world: &World, ctx: &GameContext) {
    let cx = ctx.window_w * 0.5;
    let cy = ctx.hud_h + ctx.playfield_h() * 0.5;

    match world.state {
        GameState::Start => {
            ember_stdlib::graphics::text::draw_centered_shadowed(
                "BULLET HELL",
                cx,
                cy - 20.0,
                48,
                WHITE,
                BLACK,
                2.0,
            );
            ember_stdlib::graphics::text::draw_centered(
                "PRESS SPACE TO START",
                cx,
                cy + 30.0,
                24,
                ctx.color_hud,
            );
            ember_stdlib::graphics::text::draw_centered(
                "ARROWS MOVE  ·  SHIFT FOCUS  ·  SPACE FIRE  ·  R RESTART",
                cx,
                cy + 70.0,
                16,
                Color::new(0.6, 0.65, 0.75, 1.0),
            );
            ember_stdlib::graphics::text::draw_centered(
                "GRAZE BULLETS TO BUILD YOUR MULTIPLIER",
                cx,
                cy + 100.0,
                14,
                Color::new(0.5, 0.55, 0.7, 1.0),
            );
        }
        GameState::LevelCleared => {
            ember_stdlib::graphics::text::draw_centered_shadowed(
                &format!("WAVE {} CLEAR", world.wave + 1),
                cx,
                cy,
                40,
                Color::new(0.6, 1.0, 0.7, 1.0),
                BLACK,
                2.0,
            );
        }
        GameState::GameOver => {
            ember_stdlib::graphics::text::draw_centered_shadowed(
                "GAME OVER",
                cx,
                cy - 40.0,
                48,
                Color::new(1.0, 0.3, 0.4, 1.0),
                BLACK,
                2.0,
            );
            ember_stdlib::graphics::text::draw_centered(
                &format!("SCORE {}", world.score),
                cx,
                cy + 20.0,
                24,
                WHITE,
            );
            ember_stdlib::graphics::text::draw_centered(
                &format!("GRAZED {}   ·   MAX x{:.1}", world.graze_count, world.multiplier),
                cx,
                cy + 50.0,
                16,
                Color::new(0.7, 0.75, 0.85, 1.0),
            );
            ember_stdlib::graphics::text::draw_centered(
                "R TO RESTART",
                cx,
                cy + 90.0,
                18,
                ctx.color_hud,
            );
        }
        GameState::Win => {
            ember_stdlib::graphics::text::draw_centered_shadowed(
                "YOU WIN",
                cx,
                cy - 40.0,
                48,
                Color::new(0.5, 1.0, 0.6, 1.0),
                BLACK,
                2.0,
            );
            ember_stdlib::graphics::text::draw_centered(
                &format!("FINAL SCORE {}", world.score),
                cx,
                cy + 20.0,
                24,
                WHITE,
            );
            ember_stdlib::graphics::text::draw_centered(
                &format!("GRAZED {}   ·   MULTIPLIER x{:.1}", world.graze_count, world.multiplier),
                cx,
                cy + 50.0,
                16,
                Color::new(0.7, 0.75, 0.85, 1.0),
            );
            ember_stdlib::graphics::text::draw_centered(
                "R TO PLAY AGAIN",
                cx,
                cy + 90.0,
                18,
                ctx.color_hud,
            );
        }
        _ => {}
    }
}