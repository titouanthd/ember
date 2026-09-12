use macroquad::prelude::*;
use asteroids::{
    config,
    components::{Asteroid, Bullet, Ship},
    systems::{self, GameContext, GameWorld, ShipInput},
    GameState,
};
use ember_stdlib::graphics::text::draw_centered;

fn window_conf() -> Conf {
    let ctx = config::load_config();
    Conf {
        window_title: "Asteroids - Powered by Ember".to_owned(),
        window_width: ctx.screen_w as i32,
        window_height: ctx.screen_h as i32,
        window_resizable: false,
        ..Default::default()
    }
}

// ============================================================================
// Rendu
// ============================================================================

fn draw_ship(ship: &Ship, invincible: bool) {
    if invincible {
        let t = get_time() as f32;
        if (t * 12.0).sin() < 0.0 {
            return;
        }
    }

    let r = ship.radius();
    let f = ship.forward();
    let right = Vec2::new(-f.y, f.x);

    let nose = ship.transform.position + f * r;
    let left_wing = ship.transform.position - f * r * 0.6 + right * r * 0.8;
    let right_wing = ship.transform.position - f * r * 0.6 - right * r * 0.8;
    let tail = ship.transform.position - f * r * 0.3;

    draw_triangle(nose, left_wing, tail, ship.sprite.color);
    draw_triangle(nose, tail, right_wing, ship.sprite.color);
}

fn draw_bullet(b: &Bullet) {
    draw_circle(
        b.transform.position.x,
        b.transform.position.y,
        b.radius(),
        b.sprite.color,
    );
}

fn draw_asteroid(a: &Asteroid) {
    let r = a.radius();
    let n = 8;
    let angle_offset = a.transform.rotation;
    let mut points = Vec::with_capacity(n);
    for i in 0..n {
        let theta = angle_offset + (i as f32) * std::f32::consts::TAU / (n as f32);
        let jitter = 1.0 + ((i as f32 * 12.9898).sin() * 0.15);
        points.push(Vec2::new(
            a.transform.position.x + theta.cos() * r * jitter,
            a.transform.position.y + theta.sin() * r * jitter,
        ));
    }
    for i in 0..n {
        let p1 = points[i];
        let p2 = points[(i + 1) % n];
        draw_triangle(
            a.transform.position,
            p1,
            p2,
            Color::new(a.sprite.color.r, a.sprite.color.g, a.sprite.color.b, 0.25),
        );
        draw_line(p1.x, p1.y, p2.x, p2.y, 1.5, a.sprite.color);
    }
}

fn draw_stars(ctx: &GameContext) {
    let mut seed = 12345u32;
    let mut next = || {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        (seed as f32) / (u32::MAX as f32)
    };
    for _ in 0..60 {
        let x = next() * ctx.screen_w;
        let y = next() * ctx.screen_h;
        draw_circle(x, y, 1.0, ctx.star_color);
    }
}

// ============================================================================
// main
// ============================================================================

#[macroquad::main(window_conf)]
async fn main() {
    let ctx = config::load_config();
    let screen_w = ctx.screen_w;
    let screen_h = ctx.screen_h;

    let mut world = GameWorld::new(&ctx);

    loop {
        let dt = get_frame_time();
        if is_key_pressed(KeyCode::Escape) {
            break;
        }

        // --- Input ---
        let input = ShipInput {
            rotate_left: is_key_down(KeyCode::Left) || is_key_down(KeyCode::A),
            rotate_right: is_key_down(KeyCode::Right) || is_key_down(KeyCode::D),
            thrust: is_key_down(KeyCode::Up) || is_key_down(KeyCode::W),
            shoot: is_key_down(KeyCode::Space),
        };

        // --- Machine à états ---
        match world.state {
            GameState::Start => {
                if is_key_pressed(KeyCode::Space) {
                    world.start_new_game(&ctx);
                }
            }

            GameState::Playing => {
                systems::update_ship(&mut world.ship, &input, &ctx, dt);

                if world.shoot_cooldown > 0.0 {
                    world.shoot_cooldown = (world.shoot_cooldown - dt).max(0.0);
                }
                if input.shoot && world.shoot_cooldown <= 0.0 {
                    systems::try_shoot(&world.ship, &mut world.bullets, &ctx);
                    world.shoot_cooldown = ctx.bullet_cooldown_ms as f32 / 1000.0;
                }

                if world.invincible_until > 0.0 {
                    world.invincible_until = (world.invincible_until - dt).max(0.0);
                }
                let invincible = world.invincible_until > 0.0;

                systems::update_bullets(&mut world.bullets, &ctx, dt);
                systems::update_asteroids(&mut world.asteroids, &ctx, dt);

                let destroyed = systems::resolve_bullet_asteroid_collisions(
                    &mut world.bullets,
                    &mut world.asteroids,
                    &ctx,
                );
                world.score += destroyed as i32 * 10;

                if !invincible
                    && systems::resolve_ship_asteroid_collision(&world.ship, &world.asteroids)
                {
                    world.on_ship_hit(&ctx);
                }

                if world.asteroids.is_empty() && world.state == GameState::Playing {
                    world.on_wave_cleared(&ctx);
                }
            }

            GameState::LevelCleared => {
                world.tick_level_cleared(&ctx, dt);
            }

            GameState::GameOver | GameState::Win => {
                if is_key_pressed(KeyCode::R) {
                    world.reset_to_start(&ctx);
                }
            }
        }

        // --- Rendu ---
        clear_background(ctx.bg_color);
        draw_stars(&ctx);

        for a in &world.asteroids {
            draw_asteroid(a);
        }
        for b in &world.bullets {
            draw_bullet(b);
        }
        if matches!(world.state, GameState::Playing | GameState::LevelCleared) {
            let invincible = world.invincible_until > 0.0;
            draw_ship(&world.ship, invincible);
        }

        // HUD
        draw_text(
            format!("Score: {}", world.score),
            12.0,
            24.0,
            24.0,
            ctx.ui_text_color,
        );
        draw_text(
            format!("Wave: {}/{}", world.wave.min(ctx.max_waves), ctx.max_waves),
            screen_w / 2.0 - 60.0,
            24.0,
            24.0,
            ctx.ui_text_color,
        );
        draw_text(
            format!("Lives: {}", world.lives),
            screen_w - 130.0,
            24.0,
            24.0,
            ctx.ui_text_color,
        );
        draw_text(
            format!("Best: {}", world.high_score.max(world.score)),
            screen_w - 130.0,
            52.0,
            20.0,
            ctx.star_color,
        );

        match world.state {
            GameState::Start => {
                draw_centered("ASTEROIDS", screen_w / 2.0, screen_h / 2.0 - 80.0, 90, ctx.ship_color);
                draw_centered("Left / Right to rotate, Up to thrust", screen_w / 2.0, screen_h / 2.0 - 10.0, 24, ctx.ui_text_color);
                draw_centered("SPACE to shoot", screen_w / 2.0, screen_h / 2.0 + 25.0, 24, ctx.ui_text_color);
                draw_centered(
                    &format!("Survive {} waves to win!", ctx.max_waves),
                    screen_w / 2.0, screen_h / 2.0 + 75.0, 26, ctx.bullet_color,
                );
                draw_centered("Press SPACE to start", screen_w / 2.0, screen_h / 2.0 + 130.0, 28, WHITE);
                draw_centered(
                    &format!("Best: {}", world.high_score),
                    screen_w / 2.0, screen_h / 2.0 + 175.0, 22, ctx.star_color,
                );
            }
            GameState::LevelCleared => {
                draw_centered(
                    &format!("WAVE {} CLEARED!", world.wave),
                    screen_w / 2.0, screen_h / 2.0 - 20.0, 60, ctx.bullet_color,
                );
                draw_centered(
                    &format!("Next wave in {:.1}s", world.level_cleared_timer.max(0.0)),
                    screen_w / 2.0, screen_h / 2.0 + 40.0, 26, WHITE,
                );
            }
            GameState::GameOver => {
                draw_centered("GAME OVER", screen_w / 2.0, screen_h / 2.0 - 60.0, 80, RED);
                draw_centered(
                    &format!("Score: {}", world.score),
                    screen_w / 2.0, screen_h / 2.0 + 10.0, 36, WHITE,
                );
                draw_centered(
                    &format!("Best: {}", world.high_score),
                    screen_w / 2.0, screen_h / 2.0 + 55.0, 28, ctx.bullet_color,
                );
                draw_centered("Press R to restart", screen_w / 2.0, screen_h / 2.0 + 110.0, 26, WHITE);
            }
            GameState::Win => {
                draw_centered("YOU WIN!", screen_w / 2.0, screen_h / 2.0 - 60.0, 90, GREEN);
                draw_centered(
                    &format!("Final Score: {}", world.score),
                    screen_w / 2.0, screen_h / 2.0 + 10.0, 40, WHITE,
                );
                draw_centered(
                    &format!("Best: {}", world.high_score),
                    screen_w / 2.0, screen_h / 2.0 + 60.0, 28, ctx.bullet_color,
                );
                draw_centered("Press R to restart", screen_w / 2.0, screen_h / 2.0 + 120.0, 26, WHITE);
            }
            _ => {}
        }

        next_frame().await;
    }
}