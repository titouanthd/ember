//! Rendu complet d'une frame de jeu.

use glam::Vec2;
use macroquad::prelude::*;

use crate::catapult::Catapult;
use crate::components::{Team, Tower, Unit};
use crate::config::GameContext;
use crate::juice;
use crate::systems::{Game, Phase};
use crate::textures;
use crate::units::unit_stats;

pub fn draw_game(ctx: &GameContext, game: &Game, shake: Vec2) {
    draw_world(ctx, game, shake);
    draw_hud(ctx, game);
    juice::draw_flashes(&game.juice);
}

fn draw_world(ctx: &GameContext, game: &Game, shake: Vec2) {
    let palette = game.palette;
    let cam_x = game.camera.x;
    let vh = screen_height();
    let gy = game.ground_y;
    let seed = game.visual_seed;

    textures::draw_sky(palette, gy);
    textures::draw_far_layer(palette, cam_x, gy, seed);
    textures::draw_mid_layer(palette, cam_x, gy, seed);
    textures::draw_mist(palette, gy);
    textures::draw_ground(palette, cam_x, gy, vh);

    textures::draw_tower(true, palette, game.player_tower.x, cam_x, gy, shake);
    textures::draw_tower(false, palette, game.enemy_tower.x, cam_x, gy, shake);

    draw_local_hp_bar(ctx, &game.player_tower, cam_x, gy, shake);
    draw_local_hp_bar(ctx, &game.enemy_tower, cam_x, gy, shake);

    draw_catapult_preview(ctx, game, cam_x, shake);

    draw_catapult_visual(ctx, &game.catapult, cam_x, shake, Team::Player);
    if let Some(ec) = &game.enemy_catapult {
        draw_catapult_visual(ctx, ec, cam_x, shake, Team::Enemy);
    }

    for u in &game.units {
        draw_unit(ctx, u, cam_x, shake);
    }

    for p in &game.projectiles {
        let sx = p.pos.x - cam_x + shake.x;
        let sy = p.pos.y + shake.y;
        draw_circle(sx, sy, p.radius, p.color);
    }

    juice::draw_particles(&game.juice, cam_x, shake);
    juice::draw_floating_texts(&game.juice, cam_x, shake);
}

fn draw_local_hp_bar(ctx: &GameContext, tower: &Tower, cam_x: f32, gy: f32, shake: Vec2) {
    let sx = tower.x - cam_x + shake.x;
    let sy = shake.y;
    let top = gy - textures::TOWER_VISUAL_H + sy;
    let hp_w = 100.0;
    let hp_h = 10.0;
    let hp_x = sx - hp_w * 0.5;
    let hp_y = top - 24.0;
    let hp_col = match tower.team {
        Team::Player => ctx.colors.hp_player,
        Team::Enemy => ctx.colors.hp_enemy,
    };
    draw_rectangle(hp_x, hp_y, hp_w, hp_h, Color::new(0.05, 0.05, 0.08, 1.0));
    draw_rectangle(hp_x, hp_y, hp_w * tower.hp_fraction(), hp_h, hp_col);
    draw_rectangle_lines(hp_x, hp_y, hp_w, hp_h, 1.0, ctx.colors.accent);

    let txt = format!("{:.0} / {:.0}", tower.hp.max(0.0), tower.max_hp);
    let dim = measure_text(&txt, None, 14, 1.0);
    draw_text(&txt, sx - dim.width * 0.5, hp_y - 5.0, 14.0, ctx.colors.text);
}

// ---------- Catapulte ----------

fn draw_catapult_visual(
    ctx: &GameContext,
    c: &Catapult,
    cam_x: f32,
    shake: Vec2,
    team: Team,
) {
    let sx = c.pos.x - cam_x + shake.x;
    let sy = c.pos.y + shake.y;

    if c.is_rebuilding() {
        draw_catapult_ghost(c, sx, sy, team);
        return;
    }

    let wood = Color::new(0.48, 0.32, 0.20, 1.0);
    let wood_dark = Color::new(0.28, 0.18, 0.11, 1.0);
    let wood_light = Color::new(0.64, 0.46, 0.28, 1.0);
    let metal = Color::new(0.72, 0.74, 0.78, 1.0);
    let metal_dark = Color::new(0.38, 0.38, 0.42, 1.0);
    let rope = Color::new(0.80, 0.70, 0.50, 1.0);
    let accent = match team {
        Team::Player => Color::new(0.35, 0.65, 1.0, 1.0),
        Team::Enemy => Color::new(1.0, 0.35, 0.35, 1.0),
    };

    // Base
    let base_w = 90.0;
    let base_h = 10.0;
    let base_x = sx - base_w * 0.5;
    let base_y = sy - base_h;
    draw_rectangle(base_x, base_y, base_w, base_h, wood);
    draw_rectangle_lines(base_x, base_y, base_w, base_h, 1.5, wood_dark);
    for i in 1..3 {
        let y = base_y + i as f32 * (base_h / 3.0);
        draw_line(base_x + 3.0, y, base_x + base_w - 3.0, y, 0.8, wood_dark);
    }
    draw_rectangle(base_x + 2.0, base_y + base_h - 2.5, base_w - 4.0, 2.0, accent);

    // Roues
    let wheel_r = 4.5;
    let wheel_y = sy - wheel_r;
    draw_circle(base_x + 7.0, wheel_y, wheel_r, wood_dark);
    draw_circle_lines(base_x + 7.0, wheel_y, wheel_r, 1.0, wood_light);
    draw_circle(base_x + base_w - 7.0, wheel_y, wheel_r, wood_dark);
    draw_circle_lines(base_x + base_w - 7.0, wheel_y, wheel_r, 1.0, wood_light);

    // A-frame
    let pivot_h = 45.0;
    let pivot = Vec2::new(sx, sy - base_h - pivot_h);

    draw_line(base_x + 8.0, base_y, pivot.x - 3.0, pivot.y, 5.0, wood);
    draw_line(base_x + base_w - 8.0, base_y, pivot.x + 3.0, pivot.y, 5.0, wood);
    let leg_mid_y = (base_y + pivot.y) * 0.5;
    let leg_a_x = (base_x + 8.0 + pivot.x - 3.0) * 0.5;
    let leg_b_x = (base_x + base_w - 8.0 + pivot.x + 3.0) * 0.5;
    draw_line(leg_a_x, leg_mid_y, leg_b_x, leg_mid_y, 2.0, wood_dark);

    // Plaque pivot
    let plate_w = 12.0;
    let plate_h = 12.0;
    draw_rectangle(
        pivot.x - plate_w * 0.5,
        pivot.y - plate_h * 0.5,
        plate_w,
        plate_h,
        metal,
    );
    draw_rectangle_lines(
        pivot.x - plate_w * 0.5,
        pivot.y - plate_h * 0.5,
        plate_w,
        plate_h,
        1.0,
        metal_dark,
    );
    for (dx, dy) in [(-3.0, -3.0), (3.0, -3.0), (-3.0, 3.0), (3.0, 3.0)] {
        draw_circle(pivot.x + dx, pivot.y + dy, 1.1, metal_dark);
    }

    // Bras
    let aim_dist = (c.aim_target.x - c.pos.x).abs();
    let aim_frac = ((aim_dist - crate::catapult::CATAPULT_MIN_RANGE)
        / (c.max_range - crate::catapult::CATAPULT_MIN_RANGE))
        .clamp(0.0, 1.0);
    let arm_angle = 0.55 + aim_frac * 0.40;

    let arm_len_front = 52.0;
    let arm_len_back = 18.0;
    let dir_front = Vec2::new(arm_angle.cos() * c.facing, -arm_angle.sin());
    let dir_back = -dir_front;
    let tip = pivot + dir_front * arm_len_front;
    let counter_pos = pivot + dir_back * arm_len_back;

    draw_line(counter_pos.x, counter_pos.y, tip.x, tip.y, 6.0, wood);
    draw_line(counter_pos.x, counter_pos.y, tip.x, tip.y, 2.0, wood_light);

    // Contrepoids
    let cw = 13.0;
    draw_rectangle(counter_pos.x - cw * 0.5, counter_pos.y - cw * 0.5, cw, cw, wood_dark);
    draw_rectangle_lines(
        counter_pos.x - cw * 0.5,
        counter_pos.y - cw * 0.5,
        cw,
        cw,
        1.5,
        wood_light,
    );
    draw_line(
        counter_pos.x - cw * 0.5,
        counter_pos.y - cw * 0.5,
        counter_pos.x + cw * 0.5,
        counter_pos.y + cw * 0.5,
        1.0,
        wood_light,
    );
    draw_line(
        counter_pos.x - cw * 0.5,
        counter_pos.y + cw * 0.5,
        counter_pos.x + cw * 0.5,
        counter_pos.y - cw * 0.5,
        1.0,
        wood_light,
    );

    // Fronde
    let sling_len = 14.0;
    let sling_bottom = Vec2::new(tip.x, tip.y + sling_len);
    draw_line(tip.x, tip.y, sling_bottom.x, sling_bottom.y, 1.5, rope);
    draw_circle(sling_bottom.x, sling_bottom.y, 5.0, Color::new(0.55, 0.42, 0.28, 1.0));
    draw_circle_lines(sling_bottom.x, sling_bottom.y, 5.0, 1.0, wood_dark);
    draw_circle(sling_bottom.x, sling_bottom.y, 2.0, Color::new(0.85, 0.85, 0.85, 1.0));

    // Barre HP
    let bar_w = 62.0;
    let bar_h = 5.0;
    let bar_x = sx - bar_w * 0.5;
    let bar_y = pivot.y - 26.0;
    draw_rectangle(bar_x, bar_y, bar_w, bar_h, Color::new(0.05, 0.05, 0.08, 1.0));
    draw_rectangle(
        bar_x,
        bar_y,
        bar_w * c.hp_fraction(),
        bar_h,
        Color::new(0.95, 0.55, 0.30, 1.0),
    );
    draw_rectangle_lines(bar_x, bar_y, bar_w, bar_h, 1.0, ctx.colors.accent);
}

fn draw_catapult_ghost(c: &Catapult, sx: f32, sy: f32, team: Team) {
    let ghost = Color::new(0.35, 0.35, 0.40, 0.4);
    let accent = match team {
        Team::Player => Color::new(0.35, 0.65, 1.0, 0.35),
        Team::Enemy => Color::new(1.0, 0.35, 0.35, 0.35),
    };

    let base_w = 90.0;
    let base_h = 10.0;
    let base_x = sx - base_w * 0.5;
    let base_y = sy - base_h;
    draw_rectangle(base_x, base_y, base_w, base_h, ghost);

    let pivot_h = 45.0;
    let pivot = Vec2::new(sx, sy - base_h - pivot_h);
    draw_line(base_x + 8.0, base_y, pivot.x - 3.0, pivot.y, 5.0, ghost);
    draw_line(base_x + base_w - 8.0, base_y, pivot.x + 3.0, pivot.y, 5.0, ghost);

    draw_rectangle(base_x + 2.0, base_y + base_h - 2.5, base_w - 4.0, 2.0, accent);

    let txt = format!("REBUILD {:.1}s", c.rebuild_remaining());
    let dim = measure_text(&txt, None, 14, 1.0);
    draw_text(&txt, sx - dim.width * 0.5, pivot.y - 12.0, 14.0, ghost);
}

fn draw_catapult_preview(ctx: &GameContext, game: &Game, cam_x: f32, shake: Vec2) {
    let c = &game.catapult;
    if !c.is_alive() {
        return;
    }
    let points = c.preview_points(24);
    if points.len() < 2 {
        return;
    }
    let accent = ctx.colors.accent;
    let n = points.len() as f32;
    for (i, p) in points.iter().enumerate() {
        let alpha = 1.0 - (i as f32 / n);
        let sx = p.x - cam_x + shake.x;
        let sy = p.y + shake.y;
        let r = 2.5 * alpha + 0.8;
        let col = Color::new(accent.r, accent.g, accent.b, 0.65 * alpha);
        draw_circle(sx, sy, r, col);
    }
}

// ---------- Unités ----------

fn draw_unit(ctx: &GameContext, u: &Unit, cam_x: f32, shake: Vec2) {
    let stats = match unit_stats(&u.kind) {
        Some(s) => s,
        None => return,
    };
    let size = stats.size_vec();
    let sx = u.pos.x - cam_x + shake.x;
    let sy = u.pos.y + shake.y;

    // L'anim d'attaque se joue sur la durée écoulée depuis le trigger.
    let attack_elapsed = if u.attack_cd.is_active() {
        u.attack_cd.duration() - u.attack_cd.remaining()
    } else {
        f32::MAX
    };
    let attacking = u.attack_cd.is_active()
        && (0.0..crate::stickman::ATTACK_DURATION).contains(&attack_elapsed);
    let pose = crate::stickman::auto_pose(u, attacking);

    let anim_phase = if attacking {
        attack_elapsed
    } else {
        u.pose_phase
    };

    let color = if u.hit_flash.is_active() {
        WHITE
    } else {
        match u.team {
            Team::Player => tint(stats.color(), ctx.colors.player, 0.3),
            Team::Enemy => tint(stats.color(), ctx.colors.enemy, 0.3),
        }
    };

    let facing = match u.team {
        Team::Player => 1.0,
        Team::Enemy => -1.0,
    };

    crate::stickman::draw(
        u,
        Vec2::new(sx, sy),
        size,
        facing,
        pose,
        anim_phase,
        color,
    );

    if u.hp < u.max_hp {
        let bar_w = size.x + 6.0;
        let bar_h = 4.0;
        let bx = sx - bar_w * 0.5;
        let by = sy - size.y - 8.0;
        draw_rectangle(bx, by, bar_w, bar_h, Color::new(0.1, 0.1, 0.12, 1.0));
        let hp_col = if u.team == Team::Player {
            ctx.colors.hp_player
        } else {
            ctx.colors.hp_enemy
        };
        draw_rectangle(bx, by, bar_w * u.hp_fraction(), bar_h, hp_col);
    }
}

fn tint(base: Color, team: Color, amount: f32) -> Color {
    Color::new(
        base.r * (1.0 - amount) + team.r * amount,
        base.g * (1.0 - amount) + team.g * amount,
        base.b * (1.0 - amount) + team.b * amount,
        base.a,
    )
}

fn draw_hud(ctx: &GameContext, game: &Game) {
    let x = 16.0;
    let y = 16.0;
    let w = 220.0;
    let h = 20.0;
    draw_rectangle(x, y, w, h, Color::new(0.1, 0.1, 0.12, 1.0));
    draw_rectangle(x, y, w * game.player_mana.fraction(), h, ctx.colors.mana);
    draw_rectangle_lines(x, y, w, h, 1.5, ctx.colors.accent);
    let mana_txt = format!(
        "MANA {:.0}/{:.0}",
        game.player_mana.current, game.player_mana.max
    );
    draw_text(&mana_txt, x + 6.0, y + 15.0, 13.0, ctx.colors.text);

    let gold_txt = format!(
        "GOLD {:.0}   KILLS {}   {:.1}s",
        game.player_upgrades.gold, game.stats.kills, game.stats.elapsed
    );
    draw_text(&gold_txt, x, y + 40.0, 14.0, ctx.colors.gold);

    let cat_txt = format!(
        "CATAPULT  {:.0}/{:.0}",
        game.catapult.hp.max(0.0),
        game.catapult.max_hp
    );
    let cat_col = if game.catapult.is_alive() {
        Color::new(0.95, 0.55, 0.30, 1.0)
    } else {
        Color::new(0.6, 0.35, 0.30, 1.0)
    };
    draw_text(&cat_txt, x, y + 60.0, 13.0, cat_col);

    if let Some(ec) = &game.enemy_catapult {
        let ec_txt = format!("ENEMY CAT  {:.0}/{:.0}", ec.hp.max(0.0), ec.max_hp);
        let ec_col = if ec.is_alive() {
            Color::new(1.0, 0.40, 0.40, 1.0)
        } else {
            Color::new(0.6, 0.35, 0.35, 1.0)
        };
        draw_text(&ec_txt, x, y + 78.0, 13.0, ec_col);
    }

    let title = "EMBER WARS";
    let dim = measure_text(title, None, 22, 1.0);
    let tx = screen_width() - dim.width - 20.0;
    draw_text(title, tx, 30.0, 22.0, ctx.colors.accent);

    let level_hint = format!("{} — {}", game.level.id.to_uppercase(), game.level.name);
    let dim2 = measure_text(&level_hint, None, 15, 1.0);
    draw_text(
        &level_hint,
        screen_width() - dim2.width - 20.0,
        54.0,
        15.0,
        ctx.colors.text,
    );

    if let Some(remaining) = game.survive_remaining() {
        let txt = format!("SURVIVE  {:.0}s", remaining.ceil());
        let dim = measure_text(&txt, None, 20, 1.0);
        draw_text(
            &txt,
            screen_width() * 0.5 - dim.width * 0.5,
            66.0,
            20.0,
            ctx.colors.accent,
        );
    }

    let kind_label = game.catapult.shot_kind.label();
    let kind_txt = format!("[1/2/3] {}", kind_label);
    let dim3 = measure_text(&kind_txt, None, 14, 1.0);
    draw_text(
        &kind_txt,
        screen_width() - dim3.width - 20.0,
        80.0,
        14.0,
        ctx.colors.text,
    );
}

pub fn draw_end_overlay(ctx: &GameContext, game: &Game) {
    if game.phase == Phase::Playing {
        return;
    }
    let vw = screen_width();
    let vh = screen_height();
    draw_rectangle(0.0, 0.0, vw, vh, Color::new(0.0, 0.0, 0.0, 0.65));

    let (title, color) = match game.phase {
        Phase::Won => ("VICTORY", ctx.colors.hp_player),
        Phase::Lost => ("DEFEAT", ctx.colors.hp_enemy),
        Phase::Playing => unreachable!(),
    };
    let dim = measure_text(title, None, 72, 1.0);
    draw_text(title, vw * 0.5 - dim.width * 0.5, vh * 0.4, 72.0, color);

    let sub = format!(
        "Kills {}  |  Gold {:.0}  |  {:.1}s",
        game.stats.kills, game.stats.gold_earned, game.stats.elapsed
    );
    let dim2 = measure_text(&sub, None, 24, 1.0);
    draw_text(
        &sub,
        vw * 0.5 - dim2.width * 0.5,
        vh * 0.4 + 44.0,
        24.0,
        ctx.colors.text,
    );

    let hint = "Press R to retry, Enter for menu";
    let dim3 = measure_text(hint, None, 20, 1.0);
    draw_text(
        hint,
        vw * 0.5 - dim3.width * 0.5,
        vh * 0.4 + 90.0,
        20.0,
        ctx.colors.text,
    );
}