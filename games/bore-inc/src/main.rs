//! bore-inc — entry point.
//!
//! Preload order:
//!   1. Font (async on wasm — fetch).
//!   2. Audio (async on wasm — fetch, must precede the main loop).
//!   3. Progress (sync — RON native / in-memory wasm).
//!   4. GameContext (native: .env; wasm: compiled-in defaults).
//!   5. Sprite atlas.
//!   6. Background textures.
//!   7. AppState.
//!
//! Then the loop. `next_frame().await` is called exactly **once** per
//! iteration.

use macroquad::prelude::*;

use bore_inc::atlas::AtlasBuilder;
use bore_inc::audio::{MusicPlayer, DEFAULT_TRACKS};
use bore_inc::background::BackgroundData;
use bore_inc::config::GameContext;
use bore_inc::fonts;
use bore_inc::progress;
use bore_inc::sprites::archie as archie_sprites;
use bore_inc::sprites::truck::TruckSprites;

use bore_inc::systems::{AppAction, AppState};
#[cfg(target_arch = "wasm32")]
use bore_inc::systems::AppScreen;

/// Path of the sprite sheet, relative to `assets/`.
const SPRITE_SHEET: &str = "archie.png";

/// Seed for the surface zone background. All levels in this zone
/// share the same backdrop.
const ZONE_SEED: u32 = 0x50_11;

/// Seconds between autosaves when progress is dirty.
const AUTOSAVE_INTERVAL: f32 = 2.0;

fn window_conf() -> Conf {
    Conf {
        window_title: "Bore Inc.".to_owned(),
        window_width: 1280,
        window_height: 720,
        window_resizable: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    println!("[bore-inc] boot");

    // 1. Font.
    fonts::load_default_font().await;

    // 2. Audio.
    let mut audio = MusicPlayer::new();
    audio.preload(DEFAULT_TRACKS).await;

    #[cfg(not(target_arch = "wasm32"))]
    audio.start();

    // 3. Progress.
    let prog = progress::load_progress();

    // 4. Context.
    let mut ctx = GameContext::from_env();
    ctx.viewport_w = screen_width();
    ctx.viewport_h = screen_height();

    // 5. Sprite atlas.
    let mut builder = AtlasBuilder::new();
    let image = match archie_sprites::load_sheet(SPRITE_SHEET).await {
        Ok(img) => {
            println!(
                "[bore-inc] sprites: {SPRITE_SHEET} ({}×{})",
                img.width(),
                img.height(),
            );
            img
        }
        Err(e) => panic!("[bore-inc] failed to load {SPRITE_SHEET}: {e}"),
    };
    let archie_rects = archie_sprites::bake_from_image(&mut builder, &image);
    let truck_rects = bore_inc::sprites::truck::bake_into(&mut builder);
    let _ = truck_rects; // wired to AppState below via sprites bundle

    let baked = builder.bake();
    println!(
        "[bore-inc] atlas baked: {}×{} ({} sprites)",
        baked.width,
        baked.height,
        baked.rects.len(),
    );
    let atlas = baked.upload();

    // 6. Background textures.
    let background = BackgroundData::from_seed(ZONE_SEED).upload();
    println!("[bore-inc] background uploaded (zone seed {ZONE_SEED})");

    // 7. State.
    let truck_sprites = TruckSprites {
        r: truck_rects.0,
        l: truck_rects.1,
    };
    let mut state = AppState::new(prog, audio, atlas, archie_rects, truck_sprites, background);

    #[cfg(target_arch = "wasm32")]
    {
        state.screen = AppScreen::WebStart;
    }

    println!("[bore-inc] ready — screen = {:?}", state.screen);

    // ---------- Main loop ----------
    loop {
        let dt = get_frame_time().min(0.05);

        ctx.viewport_w = screen_width();
        ctx.viewport_h = screen_height();

        match state.tick(&ctx, dt) {
            AppAction::Quit => break,
            AppAction::Continue => {}
        }

        // Autosave.
        state.time_since_save += dt;
        if state.progress_dirty && state.time_since_save >= AUTOSAVE_INTERVAL {
            match progress::save_progress(&state.progress) {
                Ok(()) => {
                    state.progress_dirty = false;
                    state.time_since_save = 0.0;
                }
                Err(e) => {
                    eprintln!("[bore-inc] autosave failed: {e}");
                    state.time_since_save = 0.0;
                }
            }
        }

        clear_background(ctx.colors.bg_bottom);
        state.draw(&ctx);

        // Exactly one await per iteration. Do not duplicate.
        next_frame().await;
    }
}