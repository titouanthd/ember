// games/pong/src/main.rs
use ember_stdlib::graphics::text::draw_centered;
use ember_stdlib::{Sprite, Transform};
use macroquad::prelude::*;
use pong::{GameContext, GameState, MatchState, Paddle, config, render, systems};

/// Frame time cap. Empêche le tunneling quand un hitch arrive (alt-tab,
/// breakpoint, shader compile, pause GC).
const MAX_FRAME_DT: f32 = 1.0 / 60.0;

/// Default window size at startup. The actual virtual canvas size is
/// `ctx.screen_w × ctx.screen_h` (from `.env`). The window can be any size;
/// the canvas is letterboxed to fit.
const DEFAULT_WINDOW_W: i32 = 800;
const DEFAULT_WINDOW_H: i32 = 600;

fn window_conf() -> Conf {
    Conf {
        window_title: "Pong - Powered by Ember".to_owned(),
        window_width: DEFAULT_WINDOW_W,
        window_height: DEFAULT_WINDOW_H,
        window_resizable: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let ctx = config::load_config();

    // Virtual canvas: the game always renders here at `ctx.screen_w ×
    // ctx.screen_h`. It's scaled up (or down) to fit the window with
    // letterbox bars. Gameplay stays in virtual coords — no rescaling
    // of entities or physics on resize.
    let virtual_w = ctx.screen_w;
    let virtual_h = ctx.screen_h;

    let render_target = render_target(virtual_w as u32, virtual_h as u32);
    render_target.texture.set_filter(FilterMode::Linear);

    let mut game_camera =
        Camera2D::from_display_rect(Rect::new(0.0, 0.0, virtual_w, virtual_h));
    game_camera.render_target = Some(render_target.clone());

    let paddle_y = ctx.screen_h / 2.0 - ctx.paddle_h / 2.0;

    let mut player_paddle = Paddle::new(
        30.0,
        paddle_y,
        ctx.paddle_w,
        ctx.paddle_h,
        ctx.player_speed,
        ctx.player_color,
    );

    let mut ai_paddle = Paddle::new(
        ctx.screen_w - 30.0 - ctx.paddle_w,
        paddle_y,
        ctx.paddle_w,
        ctx.paddle_h,
        ctx.ai_speed,
        ctx.ai_color,
    );

    let mut match_state = MatchState::new(&ctx);

    loop {
        let dt = get_frame_time().min(MAX_FRAME_DT);

        if is_key_pressed(KeyCode::Escape) {
            break;
        }

        // ---- Game logic (unchanged, virtual coords) ----
        match match_state.state {
            GameState::Start => {
                if is_key_pressed(KeyCode::Space) {
                    systems::reset_match(
                        &mut player_paddle,
                        &mut ai_paddle,
                        &mut match_state,
                        &ctx,
                    );
                    match_state.state = GameState::Playing;
                }
            }

            GameState::Playing => {
                player_paddle.reset_velocity();
                if is_key_down(KeyCode::W) || is_key_down(KeyCode::Up) {
                    player_paddle.move_up(dt);
                }
                if is_key_down(KeyCode::S) || is_key_down(KeyCode::Down) {
                    player_paddle.move_down(dt, ctx.screen_h);
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
                    systems::reset_match(
                        &mut player_paddle,
                        &mut ai_paddle,
                        &mut match_state,
                        &ctx,
                    );
                }
            }

            GameState::LevelCleared => {
                // Pong n'utilise pas LevelCleared.
            }
        }

        // ---- Render to virtual canvas ----
        set_camera(&game_camera);
        render_game(&player_paddle, &ai_paddle, &match_state, &ctx);

        // ---- Blit canvas to window (letterbox) ----
        set_default_camera();
        clear_background(BLACK); // fills letterbox bars

        let (dst_x, dst_y, dst_w, dst_h) = render::letterbox_rect(
            screen_width(),
            screen_height(),
            virtual_w,
            virtual_h,
        );
        draw_texture_ex(
            &render_target.texture,
            dst_x,
            dst_y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(dst_w, dst_h)),
                flip_y: true, // <--- LA LIGNE À AJOUTER
                ..Default::default()
            },
        );

        next_frame().await;
    }
}

fn render_game(player: &Paddle, ai: &Paddle, match_state: &MatchState, ctx: &GameContext) {
    clear_background(BLACK);

    // Ligne centrale
    for i in (0..(ctx.screen_h as i32)).step_by(30) {
        draw_rectangle(ctx.screen_w / 2.0 - 2.0, i as f32, 4.0, 15.0, WHITE);
    }

    // Entités (masquées pendant Start)
    if match_state.state != GameState::Start {
        draw_entity(&player.transform, &player.sprite);
        draw_entity(&ai.transform, &ai.sprite);
        draw_entity(&match_state.ball.transform, &match_state.ball.sprite);
    }

    // Score
    let score_text = format!("{}  |  {}", match_state.score_player, match_state.score_ai);
    draw_centered(&score_text, ctx.screen_w / 2.0, 60.0, 60, WHITE);

    // Overlays
    match match_state.state {
        GameState::Start => {
            draw_centered("PONG", ctx.screen_w / 2.0, 180.0, 100, YELLOW);
            draw_centered("Press SPACE to start", ctx.screen_w / 2.0, 300.0, 40, WHITE);
            draw_centered("W/S or Arrow keys to move", ctx.screen_w / 2.0, 360.0, 30, GRAY);
            draw_centered(
                &format!("First to {} wins !", ctx.win_score),
                ctx.screen_w / 2.0,
                420.0,
                30,
                GRAY,
            );
        }
        GameState::GameOver => {
            draw_centered("GAME OVER", ctx.screen_w / 2.0, 250.0, 80, RED);
            draw_centered("Press R to restart", ctx.screen_w / 2.0, 350.0, 40, WHITE);
        }
        GameState::Win => {
            draw_centered("YOU WIN !", ctx.screen_w / 2.0, 250.0, 80, GREEN);
            draw_centered("Press R to restart", ctx.screen_w / 2.0, 350.0, 40, WHITE);
        }
        _ => {}
    }

    draw_text(
        "Ember Engine v0.2 (Pong + .env)",
        20.0,
        ctx.screen_h - 20.0,
        25.0,
        GRAY,
    );
}

fn draw_entity(transform: &Transform, sprite: &Sprite) {
    if !sprite.visible {
        return;
    }
    draw_rectangle(
        transform.position.x,
        transform.position.y,
        transform.scale.x,
        transform.scale.y,
        sprite.color,
    );
}