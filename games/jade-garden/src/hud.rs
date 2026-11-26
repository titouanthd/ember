//! In-game HUD: score, objective, moves, cascade.
//!
//! Draws nothing in Won state — main.rs takes over with the poem
//! reveal overlay (which needs the full Progress, not just the Game).

use ember::prelude::*;

use crate::config::GameContext;
use crate::level::Objective;
use crate::systems::{Game, Phase};
use crate::util::fmt_time;

const PAD: f32 = 24.0;

pub fn draw(gfx: &mut dyn RenderingServer, ctx: &GameContext, game: &Game) {
    match game.phase {
        Phase::Won => return, // main.rs draws the win overlay
        Phase::Lost => {
            draw_lost_overlay(gfx, ctx, game);
            return;
        }
        Phase::Playing => {}
    }

    draw_top_bar(gfx, ctx, game);
    draw_objective_panel(gfx, ctx, game);

    if game.cascade_level > 0 {
        draw_cascade_indicator(gfx, ctx, game);
    }
}

fn draw_top_bar(gfx: &mut dyn RenderingServer, ctx: &GameContext, game: &Game) {
    let vw = gfx.viewport().w;

    let title = "jade-garden";
    gfx.draw_text(title, Vec2::new(PAD, 40.0), 24.0, ctx.colors.text);
    let dim_title = gfx.measure_text(title, 24.0);
    let level_txt = format!("— {}", game.level.name);
    gfx.draw_text(
        &level_txt,
        Vec2::new(PAD + dim_title.width + 12.0, 40.0),
        18.0,
        ctx.colors.text_dim,
    );

    // Timer — top right, above the score.
    let time_txt = fmt_time(game.elapsed);
    let dim_time = gfx.measure_text(&time_txt, 16.0);
    gfx.draw_text(
        &time_txt,
        Vec2::new(vw - PAD - dim_time.width, 32.0),
        16.0,
        ctx.colors.text_dim,
    );

    // Score.
    let score_txt = format!("{}", game.score);
    let dim = gfx.measure_text(&score_txt, 28.0);
    gfx.draw_text(
        &score_txt,
        Vec2::new(vw - PAD - dim.width, 68.0),
        28.0,
        ctx.colors.gold,
    );

    // Target star.
    let target_txt = format!("★ {}", game.target_score);
    let dim_t = gfx.measure_text(&target_txt, 14.0);
    gfx.draw_text(
        &target_txt,
        Vec2::new(vw - PAD - dim_t.width, 92.0),
        14.0,
        ctx.colors.text_dim,
    );
}

fn draw_objective_panel(gfx: &mut dyn RenderingServer, ctx: &GameContext, game: &Game) {
    let (label, current, target) = objective_display(game);
    let frac = if target == 0 {
        0.0
    } else {
        (current as f32 / target as f32).clamp(0.0, 1.0)
    };

    let x = PAD;
    let w = 280.0;

    // Anchor from the bottom of the screen with generous spacing.
    let base = gfx.viewport().h - PAD - 118.0;

    // Objective label.
    gfx.draw_text(&label, Vec2::new(x, base), 16.0, ctx.colors.text);

    // Progress (current / target).
    let progress = format!("{} / {}", current, target);
    gfx.draw_text(&progress, Vec2::new(x, base + 28.0), 18.0, ctx.colors.gold);

    // Progress bar.
    let bar_y = base + 42.0;
    let bar_h = 6.0;
    gfx.draw_rect(
        Rect::new(x, bar_y, w, bar_h),
        Color::new(0.15, 0.15, 0.15, 0.5),
    );
    gfx.draw_rect(Rect::new(x, bar_y, w * frac, bar_h), ctx.colors.gold);

    // Moves left.
    let moves_txt = format!("Moves left: {}", game.moves_left);
    gfx.draw_text(&moves_txt, Vec2::new(x, base + 84.0), 15.0, ctx.colors.text);

    // Hint availability.
    let uses_left = game.hint_uses_left;
    let hint_txt = format!("[H] Show a move   ({uses_left} left)");
    let hint_color = if uses_left == 0 {
        Color::new(0.30, 0.28, 0.24, 1.0)
    } else {
        ctx.colors.text_dim
    };
    gfx.draw_text(&hint_txt, Vec2::new(x, base + 112.0), 13.0, hint_color);

    // Help.
    gfx.draw_text(
        "[Tab] Help",
        Vec2::new(x, base + 132.0),
        13.0,
        ctx.colors.text_dim,
    );
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

fn draw_cascade_indicator(gfx: &mut dyn RenderingServer, ctx: &GameContext, game: &Game) {
    let text = format!("×{}", game.cascade_level + 1);
    let dim = gfx.measure_text(&text, 40.0);
    let vx = gfx.viewport().w * 0.5 - dim.width * 0.5;
    let vy = gfx.viewport().h * 0.82;
    let pulse = 1.0 + 0.08 * (game.time * 12.0).sin();
    let font_size = 40.0 * pulse;
    gfx.draw_text(
        &text,
        Vec2::new(vx, vy),
        font_size,
        Color::new(1.0, 0.65, 0.15, 1.0),
    );

    let label = "CASCADE";
    let dim_l = gfx.measure_text(label, 14.0);
    gfx.draw_text(
        label,
        Vec2::new(gfx.viewport().w * 0.5 - dim_l.width * 0.5, vy + 18.0),
        14.0,
        ctx.colors.text_dim,
    );
}

fn draw_lost_overlay(gfx: &mut dyn RenderingServer, ctx: &GameContext, game: &Game) {
    let vw = gfx.viewport().w;
    let vh = gfx.viewport().h;
    gfx.draw_rect(
        Rect::new(0.0, 0.0, vw, vh),
        Color::new(0.02, 0.03, 0.02, 0.72),
    );

    let light = Color::new(0.94, 0.90, 0.80, 1.0);
    let light_dim = Color::new(0.72, 0.68, 0.58, 1.0);

    let title = "The jades fall asleep";
    let dim = gfx.measure_text(title, 48.0);
    gfx.draw_text(
        title,
        Vec2::new(vw * 0.5 - dim.width * 0.5, vh * 0.40),
        48.0,
        ctx.colors.danger,
    );

    let sub = format!("Score {} / ★ {}", game.score, game.target_score);
    let dim2 = gfx.measure_text(&sub, 22.0);
    gfx.draw_text(
        &sub,
        Vec2::new(vw * 0.5 - dim2.width * 0.5, vh * 0.40 + 46.0),
        22.0,
        light,
    );

    let hint = "[Enter] menu    [R] retry    [Esc] menu";
    let dim3 = gfx.measure_text(hint, 18.0);
    gfx.draw_text(
        hint,
        Vec2::new(vw * 0.5 - dim3.width * 0.5, vh * 0.40 + 96.0),
        18.0,
        light_dim,
    );
}
