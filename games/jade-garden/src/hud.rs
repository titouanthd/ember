//! In-game HUD: score, objective, moves, cascade.
//!
//! Draws nothing in Won state — main.rs takes over with the poem
//! reveal overlay (which needs the full Progress, not just the Game).

use macroquad::prelude::*;

use crate::config::GameContext;
use crate::level::Objective;
use crate::systems::{Game, Phase};

const PAD: f32 = 24.0;

pub fn draw(ctx: &GameContext, game: &Game) {
    match game.phase {
        Phase::Won => return, // main.rs draws the win overlay
        Phase::Lost => {
            draw_lost_overlay(ctx, game);
            return;
        }
        Phase::Playing => {}
    }

    draw_top_bar(ctx, game);
    draw_objective_panel(ctx, game);

    if game.cascade_level > 0 {
        draw_cascade_indicator(ctx, game);
    }
}

fn draw_top_bar(ctx: &GameContext, game: &Game) {
    let vw = screen_width();

    let title = "jade-garden";
    draw_text(title, PAD, 40.0, 24.0, ctx.colors.text);
    let dim_title = measure_text(title, None, 24, 1.0);
    let level_txt = format!("— {}", game.level.name);
    draw_text(
        &level_txt,
        PAD + dim_title.width + 12.0,
        40.0,
        18.0,
        ctx.colors.text_dim,
    );

    // Timer — top right, above the score.
    let time_txt = fmt_time(game.elapsed);
    let dim_time = measure_text(&time_txt, None, 16, 1.0);
    draw_text(
        &time_txt,
        vw - PAD - dim_time.width,
        32.0,
        16.0,
        ctx.colors.text_dim,
    );

    // Score.
    let score_txt = format!("{}", game.score);
    let dim = measure_text(&score_txt, None, 28, 1.0);
    draw_text(
        &score_txt,
        vw - PAD - dim.width,
        68.0,
        28.0,
        ctx.colors.gold,
    );

    // Target star.
    let target_txt = format!("★ {}", game.target_score);
    let dim_t = measure_text(&target_txt, None, 14, 1.0);
    draw_text(
        &target_txt,
        vw - PAD - dim_t.width,
        92.0,
        14.0,
        ctx.colors.text_dim,
    );
}

fn draw_objective_panel(ctx: &GameContext, game: &Game) {
    let (label, current, target) = objective_display(game);
    let frac = if target == 0 {
        0.0
    } else {
        (current as f32 / target as f32).clamp(0.0, 1.0)
    };

    let x = PAD;
    let w = 280.0;

    // Anchor from the bottom of the screen with generous spacing.
    let base = screen_height() - PAD - 118.0;

    // Objective label.
    draw_text(&label, x, base, 16.0, ctx.colors.text);

    // Progress (current / target).
    let progress = format!("{} / {}", current, target);
    draw_text(&progress, x, base + 28.0, 18.0, ctx.colors.gold);

    // Progress bar.
    let bar_y = base + 42.0;
    let bar_h = 6.0;
    draw_rectangle(x, bar_y, w, bar_h, Color::new(0.15, 0.15, 0.15, 0.5));
    draw_rectangle(x, bar_y, w * frac, bar_h, ctx.colors.gold);

    // Moves left.
    let moves_txt = format!("Moves left: {}", game.moves_left);
    draw_text(&moves_txt, x, base + 84.0, 15.0, ctx.colors.text);

    // Hint availability.
    let uses_left = game.hint_uses_left;
    let hint_txt = format!("[H] Show a move   ({uses_left} left)");
    let hint_color = if uses_left == 0 {
        Color::new(0.30, 0.28, 0.24, 1.0)
    } else {
        ctx.colors.text_dim
    };
    draw_text(&hint_txt, x, base + 112.0, 13.0, hint_color);

    // Help.
    draw_text("[Tab] Help", x, base + 132.0, 13.0, ctx.colors.text_dim);
}

fn objective_display(game: &Game) -> (String, u32, u32) {
    match game.level.objective {
        Objective::Score(n) => ("Target score".to_string(), game.score, n),
        Objective::ClearJade(j, n) => (
            format!("Clear {} {}", n, j.latin_name()),
            game.objective_progress,
            n,
        ),
        Objective::FillPoem(n) => (
            format!("Trigger {} cascades", n),
            game.objective_progress,
            n as u32,
        ),
    }
}

fn draw_cascade_indicator(ctx: &GameContext, game: &Game) {
    let text = format!("×{}", game.cascade_level + 1);
    let dim = measure_text(&text, None, 40, 1.0);
    let vx = screen_width() * 0.5 - dim.width * 0.5;
    let vy = screen_height() * 0.82;
    let pulse = 1.0 + 0.08 * (game.time * 12.0).sin();
    let font_size = 40.0 * pulse;
    draw_text(&text, vx, vy, font_size, Color::new(1.0, 0.65, 0.15, 1.0));

    let label = "CASCADE";
    let dim_l = measure_text(label, None, 14, 1.0);
    draw_text(
        label,
        screen_width() * 0.5 - dim_l.width * 0.5,
        vy + 18.0,
        14.0,
        ctx.colors.text_dim,
    );
}

fn draw_lost_overlay(ctx: &GameContext, game: &Game) {
    let vw = screen_width();
    let vh = screen_height();
    draw_rectangle(0.0, 0.0, vw, vh, Color::new(0.02, 0.03, 0.02, 0.72));

    let light = Color::new(0.94, 0.90, 0.80, 1.0);
    let light_dim = Color::new(0.72, 0.68, 0.58, 1.0);

    let title = "The jades fall asleep";
    let dim = measure_text(title, None, 48, 1.0);
    draw_text(
        title,
        vw * 0.5 - dim.width * 0.5,
        vh * 0.40,
        48.0,
        ctx.colors.danger,
    );

    let sub = format!("Score {} / ★ {}", game.score, game.target_score);
    let dim2 = measure_text(&sub, None, 22, 1.0);
    draw_text(
        &sub,
        vw * 0.5 - dim2.width * 0.5,
        vh * 0.40 + 46.0,
        22.0,
        light,
    );

    let hint = "[Enter] menu    [R] retry    [Esc] menu";
    let dim3 = measure_text(hint, None, 18, 1.0);
    draw_text(
        hint,
        vw * 0.5 - dim3.width * 0.5,
        vh * 0.40 + 96.0,
        18.0,
        light_dim,
    );
}

/// Formats a duration in seconds as `M:SS`.
fn fmt_time(secs: f32) -> String {
    let total = secs.round() as u32;
    format!("{}:{:02}", total / 60, total % 60)
}