// games/snake/src/main.rs
use ember_stdlib::graphics::text::draw_centered;
use macroquad::prelude::*;
use snake::{
    GameState,
    components::{Cell, Direction},
    config,
    systems::{self, SnakeWorld, TickEvent},
};
use std::time::Instant;

const MAX_LEVELS: usize = 3;

fn window_conf() -> Conf {
    let ctx = config::load_config();
    Conf {
        window_title: "Snake - Powered by Ember".to_owned(),
        window_width: ctx.screen_w() as i32,
        window_height: ctx.screen_h() as i32,
        window_resizable: false,
        ..Default::default()
    }
}

fn cell_rect(cell: Cell, ctx: &systems::GameContext) -> Rect {
    Rect::new(
        cell.x as f32 * ctx.cell_size,
        cell.y as f32 * ctx.cell_size,
        ctx.cell_size,
        ctx.cell_size,
    )
}

fn draw_grid(ctx: &systems::GameContext) {
    let w = ctx.screen_w();
    let h = ctx.screen_h();
    for x in 0..=ctx.grid_cols {
        let px = x as f32 * ctx.cell_size;
        draw_line(px, 0.0, px, h, 1.0, ctx.grid_color);
    }
    for y in 0..=ctx.grid_rows {
        let py = y as f32 * ctx.cell_size;
        draw_line(0.0, py, w, py, 1.0, ctx.grid_color);
    }
}

fn draw_walls(walls: &[snake::components::Wall], ctx: &systems::GameContext) {
    let pad = 1.0;
    for w in walls {
        let r = cell_rect(w.cell, ctx);
        draw_rectangle(
            r.x + pad,
            r.y + pad,
            r.w - 2.0 * pad,
            r.h - 2.0 * pad,
            w.color,
        );
    }
}

fn draw_snake(snake: &snake::components::Snake, ctx: &systems::GameContext) {
    let pad = 2.0;
    for (i, cell) in snake.body.iter().enumerate() {
        let r = cell_rect(*cell, ctx);
        let color = if i == 0 {
            snake.color_head
        } else {
            snake.color_body
        };
        draw_rectangle(
            r.x + pad,
            r.y + pad,
            r.w - 2.0 * pad,
            r.h - 2.0 * pad,
            color,
        );
    }
}

fn draw_food(food: &snake::components::Food, ctx: &systems::GameContext) {
    let r = cell_rect(food.cell, ctx);
    let pad = 4.0;
    draw_rectangle(
        r.x + pad,
        r.y + pad,
        r.w - 2.0 * pad,
        r.h - 2.0 * pad,
        food.color,
    );
}

#[macroquad::main(window_conf)]
async fn main() {
    let ctx = config::load_config();
    let screen_w = ctx.screen_w();
    let screen_h = ctx.screen_h();

    // --- État initial ---
    let level = systems::load_level(0).expect("Impossible de charger level1.ron");
    println!("✅ Niveau chargé : {}", level.name);

    let mut world = SnakeWorld::from_level(&level, &ctx);

    // Progression inter-niveaux (reste dans main.rs : ce n'est pas du
    // "match state", c'est du "session state").
    let mut best = 0;
    let mut current_level = 0;
    let mut level_cleared_at: Option<Instant> = None;

    loop {
        let dt = get_frame_time();

        if is_key_pressed(KeyCode::Escape) {
            break;
        }

        // --- Input ---
        if world.state == GameState::Playing {
            if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::W) {
                world.snake.request_direction(Direction::Up);
            }
            if is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::S) {
                world.snake.request_direction(Direction::Down);
            }
            if is_key_pressed(KeyCode::Left) || is_key_pressed(KeyCode::A) {
                world.snake.request_direction(Direction::Left);
            }
            if is_key_pressed(KeyCode::Right) || is_key_pressed(KeyCode::D) {
                world.snake.request_direction(Direction::Right);
            }
        }

        // --- Logique d'état ---
        match world.state {
            GameState::Start => {
                if is_key_pressed(KeyCode::Space) {
                    let lvl = systems::load_level(current_level)
                        .expect("Impossible de charger le niveau");
                    world.apply_level(&lvl, &ctx);
                    world.state = GameState::Playing;
                }
            }

            GameState::Playing => {
                let ev = systems::update(&mut world, &ctx, dt);
                if let TickEvent::Died = ev {
                    best = best.max(world.score);
                }
            }

            GameState::LevelCleared => {
                if level_cleared_at.is_none() {
                    level_cleared_at = Some(Instant::now());
                }
                if level_cleared_at.unwrap().elapsed().as_secs_f32() > 1.5 {
                    current_level += 1;
                    if current_level >= MAX_LEVELS {
                        world.state = GameState::Win;
                    } else {
                        let lvl = systems::load_level(current_level)
                            .expect("Impossible de charger le niveau suivant");
                        world.apply_level(&lvl, &ctx);
                        level_cleared_at = None;
                        world.state = GameState::Playing;
                    }
                }
            }

            GameState::GameOver => {
                if is_key_pressed(KeyCode::R) {
                    best = best.max(world.score);
                    world.score = 0;
                    current_level = 0;
                    world.state = GameState::Start;
                }
            }

            GameState::Win => {
                if is_key_pressed(KeyCode::R) {
                    world.score = 0;
                    current_level = 0;
                    world.state = GameState::Start;
                }
            }
        }

        // --- Rendu ---
        clear_background(ctx.bg_color);
        draw_grid(&ctx);

        if world.state != GameState::Start {
            draw_walls(&world.walls, &ctx);
            draw_food(&world.food, &ctx);
            draw_snake(&world.snake, &ctx);
        }

        // --- HUD permanent ---
        draw_text(format!("Score: {}", world.score), 12.0, 24.0, 28.0, WHITE);
        draw_text(
            format!("Best: {}", best),
            screen_w - 130.0,
            24.0,
            28.0,
            WHITE,
        );
        let speed_level = world.snake_state.food_eaten / ctx.food_per_speedup;
        draw_text(
            format!("Speed: {}", speed_level + 1),
            screen_w - 130.0,
            52.0,
            24.0,
            GRAY,
        );
        draw_text(
            format!(
                "Level {}/{}  -  {}/{}",
                current_level + 1,
                MAX_LEVELS,
                world.snake_state.food_eaten,
                world.snake_state.target_score,
            ),
            screen_w / 2.0 - 120.0,
            24.0,
            24.0,
            WHITE,
        );

        // --- Overlays d'état ---
        match world.state {
            GameState::Start => {
                draw_centered("SNAKE", screen_w / 2.0, screen_h / 2.0 - 40.0, 80, GREEN);
                draw_centered(
                    "Press SPACE to start",
                    screen_w / 2.0,
                    screen_h / 2.0 + 20.0,
                    30,
                    WHITE,
                );
                draw_centered(
                    "Arrows / WASD to move",
                    screen_w / 2.0,
                    screen_h / 2.0 + 60.0,
                    24,
                    GRAY,
                );
                draw_centered(
                    &format!("Level {} - {} walls", current_level + 1, world.walls.len()),
                    screen_w / 2.0,
                    screen_h / 2.0 + 100.0,
                    20,
                    GRAY,
                );
            }

            GameState::LevelCleared => {
                draw_centered("LEVEL CLEARED!", screen_w / 2.0, screen_h / 2.0, 60, GREEN);
                draw_centered(
                    "Next level...",
                    screen_w / 2.0,
                    screen_h / 2.0 + 50.0,
                    30,
                    WHITE,
                );
            }

            GameState::GameOver => {
                draw_centered("GAME OVER", screen_w / 2.0, screen_h / 2.0, 70, RED);
                draw_centered(
                    &format!("Score: {}  |  Best: {}", world.score, best),
                    screen_w / 2.0,
                    screen_h / 2.0 + 50.0,
                    32,
                    WHITE,
                );
                draw_centered(
                    "Press R to restart",
                    screen_w / 2.0,
                    screen_h / 2.0 + 100.0,
                    30,
                    WHITE,
                );
            }

            GameState::Win => {
                draw_centered("YOU WIN!", screen_w / 2.0, screen_h / 2.0, 80, GREEN);
                draw_centered(
                    &format!("Final Score: {}", world.score),
                    screen_w / 2.0,
                    screen_h / 2.0 + 60.0,
                    40,
                    WHITE,
                );
                draw_centered(
                    "Press R to restart",
                    screen_w / 2.0,
                    screen_h / 2.0 + 120.0,
                    30,
                    WHITE,
                );
            }

            GameState::Playing => {}
        }

        next_frame().await;
    }
}
