// games/breakout/src/main.rs
use ember_core::io::load_from_file;
use std::path::PathBuf;
use std::env;

use macroquad::prelude::*;
use breakout::{config, Paddle, Ball, Brick, GameState, systems, Level};
use breakout::systems::UpdateEvent;
use ember_stdlib::graphics::text::draw_centered;

fn load_level(level_index: usize, ctx: &systems::GameContext) -> Vec<Brick> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let level_path = manifest_dir.join(format!("levels/level{}.ron", level_index + 1));

    match load_from_file::<Level>(&level_path) {
        Ok(level) => {
            println!("✅ Niveau {} chargé depuis {:?}", level_index + 1, level_path);
            let mut bricks = Vec::new();
            for data in level.bricks {
                let color = Color::new(data.color_r, data.color_g, data.color_b, data.color_a);
                bricks.push(Brick::new(
                    data.x,
                    data.y,
                    data.width,
                    data.height,
                    color,
                    data.health,
                ));
            }
            bricks
        }
        Err(e) => {
            println!("⚠️  Chargement niveau {} échoué : {}", level_index + 1, e);
            println!("🔨 Génération programmatique de secours...");
            generate_fallback_level(ctx)
        }
    }
}

fn generate_fallback_level(ctx: &systems::GameContext) -> Vec<Brick> {
    let mut bricks = Vec::new();
    let total_width = ctx.brick_cols as f32 * (ctx.brick_width + ctx.brick_padding) - ctx.brick_padding;
    let start_x = (ctx.screen_w - total_width) / 2.0;
    let start_y = 60.0;

    for row in 0..ctx.brick_rows {
        for col in 0..ctx.brick_cols {
            let x = start_x + col as f32 * (ctx.brick_width + ctx.brick_padding);
            let y = start_y + row as f32 * (ctx.brick_height + ctx.brick_padding);
            let color = ctx.brick_colors[row % ctx.brick_colors.len()];
            let health = if row < 2 { 2 } else { 1 };
            bricks.push(Brick::new(x, y, ctx.brick_width, ctx.brick_height, color, health));
        }
    }
    bricks
}

fn reset_paddle(paddle: &mut Paddle, ctx: &systems::GameContext) {
    paddle.transform.position.x = ctx.screen_w / 2.0 - ctx.paddle_width / 2.0;
}

#[macroquad::main("Breakout - Powered by Ember")]
async fn main() {
    let mut ctx = config::load_config();

    let screen_w = ctx.screen_w;
    let screen_h = ctx.screen_h;

    let mut paddle = Paddle::new(
        screen_w / 2.0 - ctx.paddle_width / 2.0,
        screen_h - 50.0,
        ctx.paddle_width,
        ctx.paddle_height,
        400.0,
        ctx.paddle_color,
    );

    let mut ball = Ball::new(
        screen_w / 2.0 - ctx.ball_size / 2.0,
        screen_h / 2.0,
        ctx.ball_size,
        ctx.ball_speed,
        ctx.ball_color,
    );
    ball.vy = -ctx.ball_speed;

    let mut score = 0;
    let mut lives = 3;
    let mut current_level = 0;
    let max_levels = 3;
    let mut state = GameState::Start;

    let mut bricks = load_level(current_level, &ctx);

    loop {
        let dt = get_frame_time();

        if is_key_pressed(KeyCode::Escape) {
            break;
        }

        if state == GameState::Playing {
            if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
                paddle.move_left(dt);
            }
            if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
                paddle.move_right(dt, screen_w);
            }
        }

        match state {
            GameState::Start => {
                if is_key_pressed(KeyCode::Space) {
                    state = GameState::Playing;
                    systems::reset_ball(&mut ball, &ctx);
                }
            }

            GameState::Playing => {
                let event = systems::update(
                    &mut paddle,
                    &mut ball,
                    &mut bricks,
                    &mut score,
                    &mut state,
                    &ctx,
                    dt,
                );

                match event {
                    UpdateEvent::None => {}
                    UpdateEvent::BallLost => {
                        lives -= 1;
                        if lives <= 0 {
                            state = GameState::GameOver;
                        } else {
                            systems::reset_ball(&mut ball, &ctx);
                            reset_paddle(&mut paddle, &ctx);
                        }
                    }
                    UpdateEvent::LevelCleared => {
                        current_level += 1;
                        if current_level >= max_levels {
                            state = GameState::Win;
                        } else {
                            ctx.ball_speed *= 1.1;
                            bricks = load_level(current_level, &ctx);
                            systems::reset_ball(&mut ball, &ctx);
                            reset_paddle(&mut paddle, &ctx);
                        }
                    }
                }
            }

            GameState::GameOver | GameState::Win => {
                if is_key_pressed(KeyCode::R) {
                    state = GameState::Start;
                    score = 0;
                    lives = 3;
                    current_level = 0;
                    ctx = config::load_config();
                    bricks = load_level(current_level, &ctx);
                    systems::reset_ball(&mut ball, &ctx);
                    reset_paddle(&mut paddle, &ctx);
                }
            }

            GameState::LevelCleared => {
                // Breakout n'utilise pas LevelCleared comme état principal.
            }
        }

        // --- RENDU ---
        clear_background(BLACK);

        for brick in &bricks {
            if brick.is_alive() {
                let rect = brick.transform;
                draw_rectangle(
                    rect.position.x,
                    rect.position.y,
                    rect.scale.x,
                    rect.scale.y,
                    brick.sprite.color,
                );
                draw_rectangle_lines(
                    rect.position.x,
                    rect.position.y,
                    rect.scale.x,
                    rect.scale.y,
                    1.0,
                    Color::new(0.2, 0.2, 0.2, 1.0),
                );
            }
        }

        if state != GameState::Start {
            let paddle_rect = paddle.transform;
            draw_rectangle(
                paddle_rect.position.x,
                paddle_rect.position.y,
                paddle_rect.scale.x,
                paddle_rect.scale.y,
                paddle.sprite.color,
            );
        }

        if state != GameState::Start && state != GameState::GameOver {
            let ball_rect = ball.transform;
            draw_rectangle(
                ball_rect.position.x,
                ball_rect.position.y,
                ball_rect.scale.x,
                ball_rect.scale.y,
                ball.sprite.color,
            );
        }

        draw_centered(
            &format!("Score: {}  |  Lives: {}  |  Level: {}/{}", score, lives, current_level + 1, max_levels),
            screen_w / 2.0,
            30.0,
            28,
            WHITE,
        );

        match state {
            GameState::Start => {
                draw_centered("BREAKOUT", screen_w / 2.0, 180.0, 100, YELLOW);
                draw_centered("Press SPACE to start", screen_w / 2.0, 300.0, 40, WHITE);
                draw_centered("Use Left/Right or A/D to move", screen_w / 2.0, 360.0, 30, GRAY);
                draw_centered(
                    &format!("Level {} - {} bricks to destroy!", current_level + 1, bricks.iter().filter(|b| b.is_alive()).count()),
                    screen_w / 2.0, 420.0, 30, GRAY,
                );
            }
            GameState::GameOver => {
                draw_centered("GAME OVER", screen_w / 2.0, 250.0, 80, RED);
                draw_centered(&format!("Final Score: {}", score), screen_w / 2.0, 320.0, 40, WHITE);
                draw_centered("Press R to restart", screen_w / 2.0, 380.0, 40, WHITE);
            }
            GameState::Win => {
                draw_centered("YOU WIN !", screen_w / 2.0, 200.0, 80, GREEN);
                draw_centered("Congratulations !", screen_w / 2.0, 270.0, 50, YELLOW);
                draw_centered(&format!("Final Score: {}", score), screen_w / 2.0, 330.0, 40, WHITE);
                draw_centered("Press R to restart", screen_w / 2.0, 390.0, 40, WHITE);
            }
            GameState::Playing => {}
            GameState::LevelCleared => {}
        }

        draw_text(
            format!("Ember Engine v0.2 (Breakout - Level {})", current_level + 1),
            20.0,
            screen_h - 20.0,
            25.0,
            GRAY,
        );

        next_frame().await;
    }
}