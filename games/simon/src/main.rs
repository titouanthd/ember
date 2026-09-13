//! Simon — Session B: playable game with sequence, input validation, scores.

use std::path::PathBuf;

use simon::components::{button_centers, SimonColor};
use simon::config::{load_config, GameContext};
use simon::persistence;
use simon::systems::{
    handle_click, tick_playback, ClickResult, GameWorld, PlayPhase,
};
use simon::GameState;

use ember_stdlib::audio::AudioClip;
use ember_stdlib::input::Input;
use ember_stdlib::ui::circle_button::{CircleButton, CircleButtonEvent};
use ember_stdlib::ui::label::Label;
use ember_stdlib::ui::panel::Panel;

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

struct App {
    state: GameState,
    world: GameWorld,
    best_path: PathBuf,
    sounds: Vec<AudioClip>,
    /// Tracks which button was lit last frame, to trigger a sound only on
    /// transitions (lit → not lit → lit), not every frame.
    last_lit: Option<SimonColor>,
}

impl App {
    fn restart(&mut self) {
        self.world.restart();
        self.state = GameState::Playing;
        self.last_lit = None;
    }

    fn on_round_complete(&mut self) {
        let prev_best = persistence::load_best(&self.best_path);
        let new_best = persistence::max_best(prev_best, self.world.score);
        if new_best > prev_best {
            persistence::save_best(&self.best_path, new_best);
            self.world.best = new_best;
        }
        self.world.next_round();
        self.last_lit = None;
    }

    fn on_game_over(&mut self) {
        let prev_best = persistence::load_best(&self.best_path);
        let new_best = persistence::max_best(prev_best, self.world.score);
        if new_best > prev_best {
            persistence::save_best(&self.best_path, new_best);
            self.world.best = new_best;
        }
        self.state = GameState::GameOver;
        self.last_lit = None;
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let ctx = load_config();

    // Load the 4 sounds.
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

    let best_path = persistence::default_path();
    let best = persistence::load_best(&best_path);

    let mut app = App {
        state: GameState::Start,
        world: GameWorld::new(0xDEAD_BEEF, best),
        best_path,
        sounds,
        last_lit: None,
    };

    loop {
        let dt = get_frame_time().min(1.0 / 30.0);

        let mut input = Input::from_macroquad();
        if is_key_pressed(KeyCode::Space) {
            input.keys_pressed.push(KeyCode::Space);
        }
        if is_key_pressed(KeyCode::R) {
            input.keys_pressed.push(KeyCode::R);
        }
        if is_key_pressed(KeyCode::Escape) {
            input.keys_pressed.push(KeyCode::Escape);
        }

        if input.is_key_pressed(KeyCode::Escape) {
            break;
        }

        match app.state {
            GameState::Start => {
                if input.is_key_pressed(KeyCode::Space) {
                    app.restart();
                }
            }
            GameState::Playing => {
                // Tick playback animation.
                tick_playback(&mut app.world, dt);

                // Play a sound when the lit color changes (lit → not lit → lit).
                sync_playback_sound(&mut app);

                // Accept player clicks only during Waiting phase.
                if app.world.phase.is_accepting_input() {
                    for (i, btn) in buttons.iter().enumerate() {
                        if btn.update(&input) == CircleButtonEvent::Clicked {
                            let color = SimonColor::from_index(i).unwrap();
                            // Immediate feedback sound on click.
                            app.sounds[i].play();
                            match handle_click(&mut app.world, color) {
                                ClickResult::Correct => {}
                                ClickResult::RoundComplete => {
                                    app.on_round_complete();
                                }
                                ClickResult::Wrong => {
                                    app.on_game_over();
                                }
                                ClickResult::Ignored => {}
                            }
                        }
                    }
                }
            }
            GameState::GameOver => {
                if input.is_key_pressed(KeyCode::R) || input.is_key_pressed(KeyCode::Space) {
                    app.state = GameState::Start;
                }
            }
            _ => {}
        }

        // --- Render ---
        clear_background(ctx.color_bg);

        match app.state {
            GameState::Start => render_start(&ctx, &app),
            GameState::Playing | GameState::GameOver => {
                render_playfield(&ctx, &app, &buttons, &input);
                if app.state == GameState::GameOver {
                    render_game_over(&ctx, &app);
                }
            }
            _ => {}
        }

        next_frame().await;
    }
}

/// Compare the current lit color against the last one we played. If they
/// differ and there's a new lit color, play its sound.
fn sync_playback_sound(app: &mut App) {
    let current = app.world.lit_color();
    if current != app.last_lit {
        if let Some(c) = current {
            app.sounds[c.index()].play();
        }
        app.last_lit = current;
    }
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

fn render_start(ctx: &GameContext, app: &App) {
    Label::new(
        "SIMON",
        ctx.window_w * 0.5,
        ctx.window_h * 0.35,
        72,
        ctx.color_text,
    )
    .centered()
    .draw();
    Label::new(
        "PRESS SPACE TO START",
        ctx.window_w * 0.5,
        ctx.window_h * 0.5,
        28,
        ctx.color_text,
    )
    .centered()
    .draw();
    Label::new(
        format!("Best: {}", app.world.best),
        ctx.window_w * 0.5,
        ctx.window_h * 0.6,
        22,
        ctx.color_text,
    )
    .centered()
    .draw();
    Label::new(
        "Watch the sequence, then repeat it by clicking.",
        ctx.window_w * 0.5,
        ctx.window_h * 0.7,
        16,
        ctx.color_text,
    )
    .centered()
    .draw();
}

fn render_playfield(
    ctx: &GameContext,
    app: &App,
    buttons: &[CircleButton; 4],
    input: &Input,
) {
    // Header
    Panel::new(0.0, 0.0, ctx.window_w, ctx.hud_h, ctx.color_header_bg).draw();
    Label::new(
        format!("Score: {}", app.world.score),
        20.0,
        ctx.hud_h * 0.5 + 6.0,
        24,
        ctx.color_text,
    )
    .draw();
    Label::new(
        format!("Best: {}", app.world.best),
        ctx.window_w - 20.0,
        ctx.hud_h * 0.5 + 6.0,
        24,
        ctx.color_text,
    )
    .right()
    .draw();

    // Buttons
    let lit = app.world.lit_color();
    for (i, btn) in buttons.iter().enumerate() {
        let color = SimonColor::from_index(i).unwrap();
        let is_lit = lit == Some(color);
        let (lit_c, dim_c) = colors_for(color, ctx);
        // Hover feedback only when the player can click.
        let hover = app.world.phase.is_accepting_input() && btn.contains(input.mouse_pos);
        btn.draw(lit_c, dim_c, is_lit || hover);
    }

    // Status text under the buttons
    let status = match &app.world.phase {
        PlayPhase::PreRound { .. } => "Get ready...",
        PlayPhase::Showing { .. } => "Watch...",
        PlayPhase::Waiting { .. } => "Your turn",
    };
    Label::new(
        status,
        ctx.window_w * 0.5,
        ctx.window_h - 30.0,
        20,
        ctx.color_text,
    )
    .centered()
    .draw();
}

fn render_game_over(ctx: &GameContext, app: &App) {
    let msg = "GAME OVER";
    let dims = measure_text(msg, None, 56, 1.0);
    let cx = ctx.window_w * 0.5;
    let cy = ctx.window_h * 0.5;

    let sub = format!("Score: {}  ·  Best: {}", app.world.score, app.world.best);
    let sub_dims = measure_text(&sub, None, 22, 1.0);

    let box_w = dims.width.max(sub_dims.width) + 60.0;
    let box_h = 130.0;

    draw_rectangle(
        cx - box_w * 0.5,
        cy - box_h * 0.5,
        box_w,
        box_h,
        Color::new(0.0, 0.0, 0.0, 0.85),
    );
    draw_text(
        msg,
        cx - dims.width * 0.5,
        cy - 10.0,
        56.0,
        Color::new(1.0, 0.35, 0.4, 1.0),
    );
    draw_text(
        &sub,
        cx - sub_dims.width * 0.5,
        cy + 30.0,
        22.0,
        ctx.color_text,
    );
    Label::new(
        "Press R or Space to continue",
        cx,
        cy + 60.0,
        16,
        ctx.color_text,
    )
    .centered()
    .draw();
}

fn colors_for(c: SimonColor, ctx: &GameContext) -> (Color, Color) {
    match c {
        SimonColor::Green => (ctx.color_green_lit, ctx.color_green_dim),
        SimonColor::Red => (ctx.color_red_lit, ctx.color_red_dim),
        SimonColor::Yellow => (ctx.color_yellow_lit, ctx.color_yellow_dim),
        SimonColor::Blue => (ctx.color_blue_lit, ctx.color_blue_dim),
    }
}