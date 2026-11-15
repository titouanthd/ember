// games/pagoda/src/main.rs
use ember_stdlib::input::Input;
use macroquad::prelude::*;
use pagoda::{
    App, AppState, art, audio,
    config::{GameContext, background_color},
    fonts, persistence, render, scene,
    systems::{self, MatchState},
    ui,
};

/// Extra camera zoom at full punch.
const CAMERA_PUNCH_ZOOM: f32 = 0.03;

fn window_conf() -> Conf {
    Conf {
        window_title: "PAGODA".to_owned(),
        window_width: 800,
        window_height: 600,
        window_resizable: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    // Must run before any text is drawn.
    let _font = fonts::load_default_font().await;
    let cjk = fonts::CjkFont::load().await;
    // Load the SFX bank before the loop: on the web each clip is fetched.
    let mut sounds = audio::SoundBank::load().await;

    let ctx = GameContext::default();
    let best = persistence::default();
    let mut app = App::with_high_score(best.load_or(0));
    let mut match_state = MatchState::new(&ctx);

    let virtual_w = ctx.screen_w;
    let virtual_h = ctx.screen_h;
    let render_target = render_target(virtual_w as u32, virtual_h as u32);
    render_target.texture.set_filter(FilterMode::Linear);
    let mut game_camera = Camera2D::from_display_rect(Rect::new(0.0, 0.0, virtual_w, virtual_h));
    game_camera.render_target = Some(render_target.clone());
    let base_zoom = game_camera.zoom;

    loop {
        let dt = get_frame_time().min(1.0 / 60.0);
        let mut input =
            Input::from_macroquad_with_keys(&[KeyCode::Space, KeyCode::Enter, KeyCode::Escape]);

        // The letterbox rect is fixed for the frame: reuse it for mouse
        // mapping now and for the final blit below.
        let (dst_x, dst_y, dst_w, dst_h) =
            render::letterbox_rect(screen_width(), screen_height(), virtual_w, virtual_h);

        let mute_button = ui::MuteButton::default();
        let pointer = render::window_to_virtual(
            input.mouse_pos,
            screen_width(),
            screen_height(),
            virtual_w,
            virtual_h,
        );
        let hovered = pointer.is_some_and(|p| mute_button.contains(p));
        let mut mute_clicked = hovered && input.mouse_left_pressed;

        // Macroquad reports touch separately from the mouse. On a touch
        // screen, a fresh tap is the game's one-button input, but a tap
        // on the speaker must toggle sound instead of dropping a block.
        for touch in touches() {
            if matches!(touch.phase, TouchPhase::Started) {
                let tap = render::window_to_virtual(
                    vec2(touch.position.x, touch.position.y),
                    screen_width(),
                    screen_height(),
                    virtual_w,
                    virtual_h,
                );
                if tap.is_some_and(|p| mute_button.contains(p)) {
                    mute_clicked = true;
                } else {
                    input.mouse_left_pressed = true;
                }
            }
        }

        if mute_clicked {
            sounds.toggle_mute();
            // Consume the click so it cannot start, drop or restart too.
            input.mouse_left_pressed = false;
            input.mouse_left_down = false;
        }

        let prev_state = app.state;
        app.update(&input);

        // Si on vient de lancer une partie, on initialise le match ET on saute
        // la logique de cette frame pour ne pas consommer le Space de démarrage.
        if matches!(prev_state, AppState::MainMenu | AppState::GameOver)
            && app.state == AppState::Playing
        {
            match_state = MatchState::new(&ctx);
            sounds.start();
        } else {
            for event in systems::update(&mut match_state, &mut app, &ctx, dt, &input) {
                sounds.on_event(event);
            }
        }

        // Persist a new best the moment a run ends. On WASM `save` fails
        // silently, leaving the record in memory for the session.
        if prev_state == AppState::Playing && app.state == AppState::GameOver {
            if app.record_high_score() {
                best.save(&app.high_score);
            }
            if app.new_best {
                sounds.new_best();
            }
        }

        // --- Render ---
        let shake = match_state.juice.shake_offset();
        game_camera.zoom = base_zoom * (1.0 + match_state.camera_punch * CAMERA_PUNCH_ZOOM);

        set_camera(&game_camera);
        let tower_height = match_state.tower.len() as u32;
        clear_background(background_color(tower_height));
        scene::draw(match_state.camera_y, tower_height, virtual_w, virtual_h);

        if app.state == AppState::Playing || app.state == AppState::GameOver {
            // Draw the tower. The newest block carries a short gold
            // landing outline.
            let last_index = match_state.tower.len().saturating_sub(1);
            for (index, block) in match_state.tower.iter().enumerate() {
                let glow = if index == last_index {
                    match_state.land_pulse
                } else {
                    0.0
                };
                art::draw_block(block, match_state.camera_y, shake, glow);
            }

            // Draw the sliding block (if still playing)
            if app.state == AppState::Playing {
                let top_block = match_state.tower.last().unwrap();
                let slide_y = top_block.transform.position.y - ctx.block_height
                    + match_state.camera_y
                    + shake.y;

                let slide_center = match_state.sliding.x + match_state.sliding.width / 2.0;
                let top_center = top_block.transform.position.x + top_block.transform.scale.x / 2.0;
                let window =
                    systems::perfect_window(match_state.sliding.speed, ctx.perfect_threshold);
                let aligned = (slide_center - top_center).abs() <= window;

                art::draw_sliding_block(
                    &match_state.sliding,
                    slide_y,
                    ctx.block_height,
                    shake,
                    aligned,
                );
                art::draw_drop_guide(
                    slide_center + shake.x,
                    top_center + shake.x,
                    top_block.transform.position.y + match_state.camera_y + shake.y,
                    ctx.block_height,
                    aligned,
                    get_time() as f32,
                );
            }

            match_state.juice.draw_world(match_state.camera_y, shake);
        }

        match_state.juice.draw_flash(virtual_w, virtual_h);
        app.draw(&ctx, match_state.combo, &cjk);
        mute_button.draw(sounds.is_muted(), hovered);

        // --- Blit ---
        set_default_camera();
        clear_background(BLACK);
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
