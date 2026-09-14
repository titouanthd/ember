//! FreeCell — terminal-themed solitaire.

use freecell::config::load_config;
use freecell::font;
use freecell::Game;

use macroquad::prelude::*;

fn window_conf() -> Conf {
    let ctx = load_config();
    Conf {
        window_title: "FreeCell".to_owned(),
        window_width: ctx.window_w as i32,
        window_height: ctx.window_h as i32,
        window_resizable: false,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let ctx = load_config();

    // Load DejaVu Sans Mono (has the card symbols). Must happen before
    // any text rendering. If it fails, we keep the default font.
    let _font = font::load_default_font().await;

    let _game = Game::new();

    loop {
        clear_background(ctx.color_bg);

        // Temporary test — removed in Session 15.
        draw_text("♠ ♥ ♦ ♣", 100.0, 100.0, 64.0, WHITE);

        if is_key_pressed(KeyCode::Escape) {
            break;
        }

        next_frame().await;
    }
}