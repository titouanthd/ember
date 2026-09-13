// games/breakout/src/main.rs
use macroquad::prelude::*;
use breakout::{config, systems, GameState, BreakoutWorld};
use breakout::systems::UpdateEvent;
use ember_stdlib::graphics::text::draw_centered;

/// Number of levels in the campaign. Kept in `main.rs` (session state, not
/// world state).
const MAX_LEVELS: usize = 3;

#[macroquad::main("Breakout - Powered by Ember")]
async fn main() {
    let mut ctx = config::load_config();

    let screen_w = ctx.screen_w;
    let screen_h = ctx.screen_h;

    // Load level 0 and build the world.
    let bricks = systems::load_level(0, &ctx);
    let mut world = BreakoutWorld::new(bricks, &ctx, MAX_LEVELS);

    loop {
        let dt = get_frame_time();

        if is_key_pressed(KeyCode::Escape) {
            break;
        }

        // --- Input ---
        if world.state == GameState::Playing {
            if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
                world.paddle.move_left(dt);
            }
            if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
                world.paddle.move_right(dt, screen_w);
            }
        }

        // --- State machine ---
        match world.state {
            GameState::Start => {
                if is_key_pressed(KeyCode::Space) {
                    world.state = GameState::Playing;
                    world.reset_ball(&ctx);
                }
            }

            GameState::Playing => {
                let event = systems::update(&mut world, &ctx, dt);

                match event {
                    UpdateEvent::None => {}
                    UpdateEvent::BallLost => {
                        world.on_ball_lost(&ctx);
                    }
                    UpdateEvent::LevelCleared => {
                        let next_level = world.current_level + 1;
                        if next_level >= world.max_levels {
                            world.state = GameState::Win;
                        } else {
                            ctx.ball_speed *= 1.1;
                            let bricks = systems::load_level(next_level, &ctx);
                            world.advance_level(bricks, &ctx);
                        }
                    }
                }
            }

            GameState::GameOver | GameState::Win => {
                if is_key_pressed(KeyCode::R) {
                    ctx = config::load_config();
                    let bricks = systems::load_level(0, &ctx);
                    world.start_new_game(bricks, &ctx);
                    world.state = GameState::Start;
                }
            }

            GameState::LevelCleared => {
                // Breakout n'utilise pas LevelCleared comme état principal.
            }
        }

        // --- RENDU ---
        clear_background(BLACK);

        for brick in &world.bricks {
            if brick.is_alive() {
                let t = &brick.transform;
                draw_rectangle(
                    t.position.x,
                    t.position.y,
                    t.scale.x,
                    t.scale.y,
                    brick.sprite.color,
                );
                draw_rectangle_lines(
                    t.position.x,
                    t.position.y,
                    t.scale.x,
                    t.scale.y,
                    1.0,
                    Color::new(0.2, 0.2, 0.2, 1.0),
                );
            }
        }

        if world.state != GameState::Start {
            let p = &world.paddle.transform;
            draw_rectangle(p.position.x, p.position.y, p.scale.x, p.scale.y, world.paddle.sprite.color);
        }

        if world.state != GameState::Start && world.state != GameState::GameOver {
            let b = &world.ball.transform;
            draw_rectangle(b.position.x, b.position.y, b.scale.x, b.scale.y, world.ball.sprite.color);
        }

        draw_centered(
            &format!(
                "Score: {}  |  Lives: {}  |  Level: {}/{}",
                world.score,
                world.lives,
                world.current_level + 1,
                world.max_levels,
            ),
            screen_w / 2.0,
            30.0,
            28,
            WHITE,
        );

        match world.state {
            GameState::Start => {
                draw_centered("BREAKOUT", screen_w / 2.0, 180.0, 100, YELLOW);
                draw_centered("Press SPACE to start", screen_w / 2.0, 300.0, 40, WHITE);
                draw_centered("Use Left/Right or A/D to move", screen_w / 2.0, 360.0, 30, GRAY);
                draw_centered(
                    &format!(
                        "Level {} - {} bricks to destroy!",
                        world.current_level + 1,
                        world.bricks.iter().filter(|b| b.is_alive()).count()
                    ),
                    screen_w / 2.0,
                    420.0,
                    30,
                    GRAY,
                );
            }
            GameState::GameOver => {
                draw_centered("GAME OVER", screen_w / 2.0, 250.0, 80, RED);
                draw_centered(&format!("Final Score: {}", world.score), screen_w / 2.0, 320.0, 40, WHITE);
                draw_centered("Press R to restart", screen_w / 2.0, 380.0, 40, WHITE);
            }
            GameState::Win => {
                draw_centered("YOU WIN !", screen_w / 2.0, 200.0, 80, GREEN);
                draw_centered("Congratulations !", screen_w / 2.0, 270.0, 50, YELLOW);
                draw_centered(&format!("Final Score: {}", world.score), screen_w / 2.0, 330.0, 40, WHITE);
                draw_centered("Press R to restart", screen_w / 2.0, 390.0, 40, WHITE);
            }
            GameState::Playing => {}
            GameState::LevelCleared => {}
        }

        draw_text(
            format!("Ember Engine v0.2 (Breakout - Level {})", world.current_level + 1),
            20.0,
            screen_h - 20.0,
            25.0,
            GRAY,
        );

        next_frame().await;
    }
}