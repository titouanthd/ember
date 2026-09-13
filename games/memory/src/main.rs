//! Memory — terminal-themed pair matching game.

use memory::config::load_config;
use memory::Game;

use macroquad::prelude::*;

fn window_conf() -> Conf {
    let ctx = load_config();
    Conf {
        window_title: "Memory".to_owned(),
        window_width: ctx.window_w as i32,
        window_height: ctx.window_h as i32,
        window_resizable: false,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let ctx = load_config();
    let _game = Game::new();

    loop {
        clear_background(ctx.color_bg);

        let dt = get_frame_time();
        let _ = dt;

        // TODO Session 8-9 : input, update, render

        if is_key_pressed(KeyCode::Escape) {
            break;
        }

        next_frame().await;
    }
}