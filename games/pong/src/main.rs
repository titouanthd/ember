// games/pong/src/main.rs
use ember_stdlib::graphics::text::draw_centered;
use macroquad::prelude::*;
use pong::{GameState, MatchState, Paddle, config, systems};

#[macroquad::main("Pong - Powered by Ember")]
async fn main() {
    let ctx = config::load_config();

    let screen_w = ctx.screen_w;
    let screen_h = ctx.screen_h;
    let paddle_w = ctx.paddle_w;
    let paddle_h = ctx.paddle_h;

    let mut player_paddle = Paddle::new(
        30.0,
        screen_h / 2.0 - paddle_h / 2.0,
        paddle_w,
        paddle_h,
        ctx.player_speed,
        ctx.player_color,
    );

    let mut ai_paddle = Paddle::new(
        screen_w - 30.0 - paddle_w,
        screen_h / 2.0 - paddle_h / 2.0,
        paddle_w,
        paddle_h,
        ctx.ai_speed,
        ctx.ai_color,
    );

    let mut match_state = MatchState::new(&ctx);

    loop {
        let dt = get_frame_time();

        if is_key_pressed(KeyCode::Escape) {
            break;
        }

        match match_state.state {
            GameState::Start => {
                if is_key_pressed(KeyCode::Space) {
                    match_state.state = GameState::Playing;
                    systems::reset_game(&mut player_paddle, &mut ai_paddle, &mut match_state, &ctx);
                }
            }

            GameState::Playing => {
                if is_key_down(KeyCode::W) || is_key_down(KeyCode::Up) {
                    player_paddle.move_up(dt);
                }
                if is_key_down(KeyCode::S) || is_key_down(KeyCode::Down) {
                    player_paddle.move_down(dt, screen_h);
                }

                systems::update(
                    &mut player_paddle,
                    &mut ai_paddle,
                    &mut match_state,
                    &ctx,
                    dt,
                );
            }

            GameState::GameOver | GameState::Win => {
                if is_key_pressed(KeyCode::R) {
                    match_state.state = GameState::Start;
                    systems::reset_game(&mut player_paddle, &mut ai_paddle, &mut match_state, &ctx);
                }
            }

            GameState::LevelCleared => {
                // Pong n'utilise pas LevelCleared.
            }
        }

        clear_background(BLACK);

        // Ligne centrale
        for i in (0..(screen_h as i32)).step_by(30) {
            draw_rectangle(screen_w / 2.0 - 2.0, i as f32, 4.0, 15.0, WHITE);
        }

        if match_state.state != GameState::Start {
            let player_rect = player_paddle.rect();
            let ai_rect = ai_paddle.rect();
            let ball_rect = match_state.ball.rect();

            draw_rectangle(
                player_rect.x,
                player_rect.y,
                player_rect.w,
                player_rect.h,
                player_paddle.sprite.color,
            );
            draw_rectangle(
                ai_rect.x,
                ai_rect.y,
                ai_rect.w,
                ai_rect.h,
                ai_paddle.sprite.color,
            );
            draw_rectangle(
                ball_rect.x,
                ball_rect.y,
                ball_rect.w,
                ball_rect.h,
                match_state.ball.sprite.color,
            );
        }

        let score_text = format!("{}  |  {}", match_state.score_player, match_state.score_ai);
        draw_centered(&score_text, screen_w / 2.0, 60.0, 60, WHITE);

        match match_state.state {
            GameState::Start => {
                draw_centered("PONG", screen_w / 2.0, 180.0, 100, YELLOW);
                draw_centered("Press SPACE to start", screen_w / 2.0, 300.0, 40, WHITE);
                draw_centered("W/S or Arrow keys to move", screen_w / 2.0, 360.0, 30, GRAY);
                draw_centered(
                    &format!("First to {} wins !", ctx.win_score),
                    screen_w / 2.0,
                    420.0,
                    30,
                    GRAY,
                );
            }
            GameState::GameOver => {
                draw_centered("GAME OVER", screen_w / 2.0, 250.0, 80, RED);
                draw_centered("Press R to restart", screen_w / 2.0, 350.0, 40, WHITE);
            }
            GameState::Win => {
                draw_centered("YOU WIN !", screen_w / 2.0, 250.0, 80, GREEN);
                draw_centered("Press R to restart", screen_w / 2.0, 350.0, 40, WHITE);
            }
            _ => {}
        }

        draw_text(
            "Ember Engine v0.2 (Pong + .env)",
            20.0,
            screen_h - 20.0,
            25.0,
            GRAY,
        );

        next_frame().await;
    }
}
