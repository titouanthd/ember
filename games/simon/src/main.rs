//! Simon — Session A: 4 buttons that play sounds and light up on click.

use std::path::PathBuf;

use simon::components::{button_centers, SimonColor};
use simon::config::{load_config, GameContext};

use ember_stdlib::audio::AudioClip;
use ember_stdlib::input::Input;
use ember_stdlib::ui::circle_button::{CircleButton, CircleButtonEvent};
use ember_stdlib::ui::label::Label;

use macroquad::prelude::*;

fn window_conf() -> Conf {
    let ctx = load_config();
    Conf {
        window_title: "Simon".to_owned(),
        window_width: ctx.window_w as i32,
        window_height: ctx.window_h as i32,
        window_resizable: false,
        ..Default::default()
    }
}

fn sound_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("sounds")
        .join(format!("{name}.wav"))
}

const FLASH_DURATION: f32 = 0.25;

struct Flash {
    color: SimonColor,
    remaining: f32,
}

#[macroquad::main(window_conf)]
async fn main() {
    let ctx = load_config();

    // Load the 4 sounds. Panic with a clear message if missing.
    let sound_names = ["green", "red", "yellow", "blue"];
    let mut sounds: Vec<AudioClip> = Vec::with_capacity(4);
    for name in sound_names {
        let path = sound_path(name);
        match AudioClip::load(&path).await {
            Ok(clip) => sounds.push(clip),
            Err(e) => panic!("Failed to load {path:?}: {e}"),
        }
    }

    let centers = button_centers(&ctx);
    let buttons: [CircleButton; 4] =
        std::array::from_fn(|i| CircleButton::new(centers[i], ctx.button_radius));

    let mut current_flash: Option<Flash> = None;

    loop {
        let dt = get_frame_time().min(1.0 / 30.0);

        let mut input = Input::from_macroquad();
        if is_key_pressed(KeyCode::Escape) {
            input.keys_pressed.push(KeyCode::Escape);
        }

        if input.is_key_pressed(KeyCode::Escape) {
            break;
        }

        // Tick the flash timer.
        if let Some(f) = current_flash.as_mut() {
            f.remaining -= dt;
            if f.remaining <= 0.0 {
                current_flash = None;
            }
        }

        // Handle clicks.
        for (i, btn) in buttons.iter().enumerate() {
            if btn.update(&input) == CircleButtonEvent::Clicked {
                let color = SimonColor::from_index(i).unwrap();
                sounds[i].play();
                current_flash = Some(Flash {
                    color,
                    remaining: FLASH_DURATION,
                });
            }
        }

        // --- Render ---
        clear_background(ctx.color_bg);

        for (i, btn) in buttons.iter().enumerate() {
            let color = SimonColor::from_index(i).unwrap();
            let lit = current_flash.as_ref().map(|f| f.color) == Some(color);
            let (lit_color, dim_color) = colors_for(color, &ctx);
            btn.draw(lit_color, dim_color, lit);
        }

        Label::new("SIMON", ctx.window_w * 0.5, 40.0, 32, ctx.color_text)
            .centered()
            .draw();

        Label::new(
            "Click a button — Session A (no sequence yet)",
            ctx.window_w * 0.5,
            ctx.window_h - 20.0,
            16,
            ctx.color_text,
        )
        .centered()
        .draw();

        next_frame().await;
    }
}

fn colors_for(c: SimonColor, ctx: &GameContext) -> (Color, Color) {
    match c {
        SimonColor::Green => (ctx.color_green_lit, ctx.color_green_dim),
        SimonColor::Red => (ctx.color_red_lit, ctx.color_red_dim),
        SimonColor::Yellow => (ctx.color_yellow_lit, ctx.color_yellow_dim),
        SimonColor::Blue => (ctx.color_blue_lit, ctx.color_blue_dim),
    }
}