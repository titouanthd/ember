// games/taiji/src/main.rs
use ember_stdlib::input::Input;
use macroquad::prelude::*;
use taiji::systems::{self, MatchState, ScoreEvent};
use taiji::{config::GameContext, render, App, AppState};

fn window_conf() -> Conf {
    Conf {
        window_title: "TÀIJÍ".to_owned(),
        window_width: 800,
        window_height: 600,
        window_resizable: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let ctx = GameContext::default();
    let mut app = App::new();
    let mut match_state = MatchState::new(&ctx);

    let virtual_w = ctx.screen_w;
    let virtual_h = ctx.screen_h;
    let render_target = render_target(virtual_w as u32, virtual_h as u32);
    render_target.texture.set_filter(FilterMode::Linear);
    let mut game_camera =
        Camera2D::from_display_rect(Rect::new(0.0, 0.0, virtual_w, virtual_h));
    game_camera.render_target = Some(render_target.clone());

    loop {
        let dt = get_frame_time().min(1.0 / 60.0);
        let input = Input::from_macroquad_with_keys(&[
            KeyCode::W,
            KeyCode::S,
            KeyCode::Up,
            KeyCode::Down,
            KeyCode::Space,
            KeyCode::Escape,
        ]);

        let prev_state = app.state;
        app.update(&input);

        // Reset du match si on démarre une nouvelle partie
        if matches!(prev_state, AppState::MainMenu | AppState::GameOver)
            && app.state == AppState::Playing
        {
            match_state = MatchState::new(&ctx);
        }

        // Logique de jeu
        if app.state == AppState::Playing {
            if let Some(event) = systems::update(&mut match_state, &ctx, dt, &input) {
                match event {
                    ScoreEvent::PlayerScored => {
                        app.score_player += 1;
                        if app.score_player >= ctx.win_score {
                            app.state = AppState::GameOver;
                        } else {
                            match_state.reset_round(&ctx, -1.0);
                        }
                    }
                    ScoreEvent::AiScored => {
                        app.score_ai += 1;
                        if app.score_ai >= ctx.win_score {
                            app.state = AppState::GameOver;
                        } else {
                            match_state.reset_round(&ctx, 1.0);
                        }
                    }
                }
            }
        }

        // --- Rendu dans le canvas virtuel ---
        set_camera(&game_camera);
        clear_background(WHITE);

        // Effet visuel du Souffle du Temps (Teinte bleue)
        if match_state.time_scale < 0.95 {
            let intensity = (1.0 - match_state.time_scale) / 0.75;
            draw_rectangle(0.0, 0.0, virtual_w, virtual_h, Color::new(0.4, 0.6, 0.8, intensity * 0.2));
            
            // Texte "YIN" subtil
            let text = "YIN";
            let dim = measure_text(text, None, 20, 1.0);
            draw_text(
                text,
                virtual_w / 2.0 - dim.width / 2.0,
                50.0,
                20.0,
                Color::new(0.0, 0.0, 0.0, intensity * 0.5),
            );
        }

        if app.state == AppState::Playing || app.state == AppState::Paused {
            draw_paddle(&match_state.player, &ctx);
            draw_paddle(&match_state.ai, &ctx);
            draw_ball(&match_state, &ctx);
        }

        app.draw(&ctx);

        // --- Blit avec letterbox ---
        set_default_camera();
        clear_background(BLACK);
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
                flip_y: true,
                ..Default::default()
            },
        );

        next_frame().await;
    }
}

fn draw_paddle(paddle: &taiji::Paddle, ctx: &GameContext) {
    draw_rectangle(
        paddle.transform.position.x,
        paddle.transform.position.y,
        ctx.paddle_w,
        ctx.paddle_h,
        paddle.color,
    );
}

fn draw_ball(match_state: &MatchState, ctx: &GameContext) {
    let ball = &match_state.ball;
    
    // La balle devient bleue quand le temps est ralenti
    let color = if match_state.time_scale < 0.95 {
        Color::new(0.0, 0.5, 1.0, 1.0) // Bleu Yin
    } else {
        ctx.color_ball
    };

    draw_circle(
        ball.transform.position.x,
        ball.transform.position.y,
        ctx.ball_radius,
        color,
    );
}