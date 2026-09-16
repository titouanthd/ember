//! Air Hockey — entry point.
//!
//! Owns the macroquad window, builds one `Input` snapshot per frame, drains
//! `GameEvent`s into the `SoundBank`, and draws everything. All game logic
//! lives in `systems.rs`; all physics in `physics.rs`; all shapes here.

use std::path::PathBuf;

use air_hockey::audio::SoundBank;
use air_hockey::components::Side;
use air_hockey::{Game, GameContext, Phase};
use ember_stdlib::input::Input;
use glam::Vec2;
use macroquad::prelude::*;

/// Duration of the visual goal flash (must match `systems::GOAL_FLASH_TIME`).
const GOAL_FLASH_TIME: f32 = 0.2;

fn window_conf() -> Conf {
    Conf {
        window_title: "Air Hockey".to_owned(),
        window_width: 1280,
        window_height: 720,
        window_resizable: false,
        high_dpi: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let _font = air_hockey::font::load_default_font().await;
    let ctx = GameContext::load();
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sounds = SoundBank::load(&manifest_dir).await;
    let mut game = Game::new(&ctx);

    // Every key the game reads. Must stay in sync with the key constants
    // in `systems.rs` (P1_UP / P1_DOWN / ... and the meta keys).
    let tracked_keys: &[KeyCode] = &[
        // Player 1 (with AZERTY physical-key aliases).
        KeyCode::W,
        KeyCode::Z,
        KeyCode::S,
        KeyCode::A,
        KeyCode::Q,
        KeyCode::D,
        // Player 2.
        KeyCode::Up,
        KeyCode::Down,
        KeyCode::Left,
        KeyCode::Right,
        // Meta.
        KeyCode::Space,
        KeyCode::P,
        KeyCode::Escape,
    ];

    loop {
        // `Input::from_macroquad_with_keys` only populates `keys_pressed`
        // (see the stdlib doc comment). Continuous movement reads
        // `keys_down`, so we fill the two remaining vectors here from the
        // same list.
        let mut input = Input::from_macroquad_with_keys(tracked_keys);
        input.keys_down = tracked_keys
            .iter()
            .copied()
            .filter(|k| is_key_down(*k))
            .collect();
        input.keys_released = tracked_keys
            .iter()
            .copied()
            .filter(|k| is_key_released(*k))
            .collect();

        if input.is_key_pressed(KeyCode::Escape) {
            break;
        }

        let raw_dt = get_frame_time();
        let dt = raw_dt.min(ctx.tuning.dt_max);

        let events = game.update(&input, &ctx, dt);
        for ev in events {
            sounds.on_event(ev);
        }

        clear_background(ctx.colors.table_bg);
        draw_frame(&game, &ctx);
        next_frame().await;
    }
}

// ─── Rendering ─────────────────────────────────────────────────────────────

fn draw_frame(game: &Game, ctx: &GameContext) {
    let shake = game.shake_offset;
    draw_table(ctx, shake);
    draw_goals(game, ctx, shake);
    draw_particles(game, ctx, shake);
    draw_puck(game, ctx, shake);
    draw_paddles(game, ctx, shake);
    draw_hud(game, ctx, shake);
    draw_overlays(game, ctx, shake);
}

fn draw_table(ctx: &GameContext, shake: Vec2) {
    let a = &ctx.arena;
    let ox = shake.x;
    let oy = shake.y;

    draw_rectangle(
        a.wall + ox,
        a.wall + oy,
        a.w - 2.0 * a.wall,
        a.h - 2.0 * a.wall,
        ctx.colors.table_bg,
    );

    let step = 80.0;
    let mut x = a.wall + step;
    while x < a.w - a.wall - 1.0 {
        draw_line(x + ox, a.wall + oy, x + ox, a.h - a.wall + oy, 1.0, ctx.colors.grid);
        x += step;
    }
    let mut y = a.wall + step;
    while y < a.h - a.wall - 1.0 {
        draw_line(a.wall + ox, y + oy, a.w - a.wall + ox, y + oy, 1.0, ctx.colors.grid);
        y += step;
    }

    draw_line(
        a.mid_x() + ox,
        a.wall + oy,
        a.mid_x() + ox,
        a.h - a.wall + oy,
        2.0,
        ctx.colors.midline,
    );
    draw_circle_lines(a.mid_x() + ox, a.mid_y() + oy, 60.0, 2.0, ctx.colors.midline);

    let b = ctx.colors.table_border;
    draw_rectangle(ox, oy, a.w, a.wall, b);
    draw_rectangle(ox, a.h - a.wall + oy, a.w, a.wall, b);
    draw_rectangle(ox, oy, a.wall, a.goal_top(), b);
    draw_rectangle(ox, a.goal_bottom() + oy, a.wall, a.h - a.goal_bottom(), b);
    draw_rectangle(a.w - a.wall + ox, oy, a.wall, a.goal_top(), b);
    draw_rectangle(
        a.w - a.wall + ox,
        a.goal_bottom() + oy,
        a.wall,
        a.h - a.goal_bottom(),
        b,
    );
}

fn draw_goals(game: &Game, ctx: &GameContext, shake: Vec2) {
    let a = &ctx.arena;
    let ox = shake.x;
    let oy = shake.y;

    let left = (ox, a.goal_top() + oy, a.wall, a.goal_h);
    let right = (a.w - a.wall + ox, a.goal_top() + oy, a.wall, a.goal_h);

    draw_rectangle(left.0, left.1, left.2, left.3, ctx.colors.goal_p1);
    draw_rectangle(right.0, right.1, right.2, right.3, ctx.colors.goal_p2);

    if game.goal_flash[0] > 0.0 {
        let mut c = ctx.colors.goal_flash;
        c.a *= (game.goal_flash[0] / GOAL_FLASH_TIME).clamp(0.0, 1.0);
        draw_rectangle(left.0, left.1, left.2, left.3, c);
    }
    if game.goal_flash[1] > 0.0 {
        let mut c = ctx.colors.goal_flash;
        c.a *= (game.goal_flash[1] / GOAL_FLASH_TIME).clamp(0.0, 1.0);
        draw_rectangle(right.0, right.1, right.2, right.3, c);
    }
}

fn draw_particles(game: &Game, ctx: &GameContext, shake: Vec2) {
    for p in game.particles.iter() {
        let alpha = p.alive_fraction();
        let mut c = ctx.colors.puck;
        c.a = alpha * 0.9;
        let size = 2.0 + alpha * 3.0;
        draw_circle(p.pos.x + shake.x, p.pos.y + shake.y, size, c);
    }
}

fn draw_puck(game: &Game, ctx: &GameContext, shake: Vec2) {
    let p = &game.puck;
    let ox = shake.x;
    let oy = shake.y;

    for (pos, t) in game.trail.iter() {
        let mut c = ctx.colors.puck_trail;
        c.a *= 1.0 - t;
        let r = p.radius * (1.0 - t * 0.7);
        draw_circle(pos.x + ox, pos.y + oy, r, c);
    }

    let shadow = Color::new(0.0, 0.0, 0.0, 0.15);
    draw_circle(p.pos.x + ox + 2.0, p.pos.y + oy + 3.0, p.radius, shadow);
    draw_circle(p.pos.x + ox, p.pos.y + oy, p.radius, ctx.colors.puck);

    let hx = p.pos.x + ox - p.radius * 0.35;
    let hy = p.pos.y + oy - p.radius * 0.35;
    draw_circle(hx, hy, p.radius * 0.35, Color::new(1.0, 1.0, 1.0, 0.7));
}

fn draw_paddles(game: &Game, ctx: &GameContext, shake: Vec2) {
    for pad in &game.paddles {
        let base = ctx.colors.for_side(pad.side);
        let ox = pad.pos.x + shake.x;
        let oy = pad.pos.y + shake.y;
        let r = pad.radius;

        let mut halo = base;
        halo.a = 0.18;
        draw_circle(ox, oy, r * 1.35, halo);

        draw_circle(ox, oy, r, base);

        let core = Color::new(
            (base.r + 1.0) * 0.5,
            (base.g + 1.0) * 0.5,
            (base.b + 1.0) * 0.5,
            0.85,
        );
        draw_circle(ox, oy, r * 0.45, core);
    }
}

fn draw_hud(game: &Game, ctx: &GameContext, shake: Vec2) {
    let a = &ctx.arena;
    let cx = a.mid_x() + shake.x;
    let top = 60.0 + shake.y;

    let score_text = format!("J1   {}  —  {}   J2", game.score.p1, game.score.p2);
    let played = game.rounds.p1 + game.rounds.p2;
    let best_of = ctx.tuning.rounds_to_win_match * 2 - 1;
    let rounds_text = format!("Manche {} / {}", played + 1, best_of);

    draw_centered(&score_text, cx + 2.0, top + 2.0, 48, ctx.colors.text_shadow);
    draw_centered(&score_text, cx, top, 48, ctx.colors.text);

    draw_centered(&rounds_text, cx + 1.0, top + 39.0, 22, ctx.colors.text_shadow);
    draw_centered(&rounds_text, cx, top + 38.0, 22, ctx.colors.text);
}

fn draw_overlays(game: &Game, ctx: &GameContext, shake: Vec2) {
    let a = &ctx.arena;
    let cx = a.mid_x() + shake.x;
    let cy = a.mid_y() + shake.y;

    match game.phase {
        Phase::Menu => {
            draw_centered("AIR HOCKEY", cx + 2.0, cy - 118.0, 72, ctx.colors.text_shadow);
            draw_centered("AIR HOCKEY", cx, cy - 120.0, 72, ctx.colors.text);
            draw_centered(
                "Appuyez sur ESPACE pour jouer",
                cx,
                cy + 20.0,
                32,
                ctx.colors.text,
            );
            draw_centered(
                "J1 : W A S D     J2 : Flèches",
                cx,
                cy + 80.0,
                24,
                ctx.colors.text,
            );
            draw_centered(
                "P : pause     Échap : quitter",
                cx,
                cy + 120.0,
                20,
                ctx.colors.text,
            );
        }
        Phase::Countdown { t } => {
            let n = t.ceil().max(1.0) as i32;
            let label = if n > 1 { format!("{}", n) } else { "GO!".to_string() };
            let frac = t - t.floor();
            let scale = 1.4 - 0.4 * frac;
            let size = (120.0 * scale).max(40.0) as u16;
            draw_centered(&label, cx, cy - 40.0, size, ctx.colors.text);
        }
        Phase::GoalPause { scorer, .. } => {
            let who = if scorer == Side::Left { 1 } else { 2 };
            let msg = format!("BUT  J{} !", who);
            draw_centered(&msg, cx + 2.0, cy - 38.0, 72, ctx.colors.text_shadow);
            draw_centered(&msg, cx, cy - 40.0, 72, ctx.colors.text);
        }
        Phase::RoundOver { winner, .. } => {
            let who = if winner == Side::Left { 1 } else { 2 };
            let msg = format!("Manche à J{}", who);
            draw_centered(&msg, cx, cy - 40.0, 64, ctx.colors.text);
        }
        Phase::MatchOver { winner } => {
            let who = if winner == Side::Left { 1 } else { 2 };
            let msg = format!("J{} GAGNE !", who);
            draw_centered(&msg, cx, cy - 80.0, 72, ctx.colors.text);
            draw_centered("ESPACE pour rejouer", cx, cy + 40.0, 32, ctx.colors.text);
        }
        Phase::Playing => {
            if game.paused {
                draw_centered("PAUSE", cx, cy - 40.0, 72, ctx.colors.text);
                draw_centered("P pour reprendre", cx, cy + 40.0, 24, ctx.colors.text);
            }
        }
    }
}

// ─── Centred text helper (local, to avoid stdlib path guessing) ────────────

fn draw_centered(text: &str, center_x: f32, y: f32, size: u16, color: Color) {
    let dim = measure_text(text, None, size, 1.0);
    draw_text(
        text,
        center_x - dim.width / 2.0,
        y,
        size as f32,
        color,
    );
}