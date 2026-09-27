//! Rendu complet d'une frame de jeu.

use glam::Vec2;
use macroquad::prelude::*;

use crate::catapult::{CATAPULT_HITBOX_H, CATAPULT_HITBOX_W};
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

    // Preview de la trajectoire (avant le projectile).
    draw_catapult_preview(ctx, game, cam_x, shake);

    draw_catapult(ctx, game, cam_x, shake);

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
    let top = gy - 140.0 + sy;
    let hp_w = 100.0;
    let hp_h = 10.0;
    let hp_x = sx - hp_w * 0.5;
    let hp_y = top - 34.0;
    let hp_col = match tower.team {
        Team::Player => ctx.colors.hp_player,
        Team::Enemy => ctx.colors.hp_enemy,
    };
    draw_rectangle(hp_x, hp_y, hp_w, hp_h, Color::new(0.05, 0.05, 0.08, 1.0));
    draw_rectangle(hp_x, hp_y, hp_w * tower.hp_fraction(), hp_h, hp_col);
    draw_rectangle_lines(hp_x, hp_y, hp_w, hp_h, 1.0, ctx.colors.accent);

    let txt = format!("{:.0} / {:.0}", tower.hp.max(0.0), tower.max_hp);
    let dim = measure_text(&txt, None, 14, 1.0);
    draw_text(
        &txt,
        sx - dim.width * 0.5,
        hp_y - 5.0,
        14.0,
        ctx.colors.text,
    );
}

fn draw_catapult(ctx: &GameContext, game: &Game, cam_x: f32, shake: Vec2) {
    let c = &game.catapult;
    let sx = c.pos.x - cam_x + shake.x;
    let sy = c.pos.y + shake.y;

    if c.is_rebuilding() {
        // Silhouette fantôme + compte à rebours.
        let ghost = Color::new(0.35, 0.35, 0.40, 0.35);
        draw_rectangle(
            sx - CATAPULT_HITBOX_W * 0.5,
            sy - CATAPULT_HITBOX_H,
            CATAPULT_HITBOX_W,
            CATAPULT_HITBOX_H,
            ghost,
        );
        let txt = format!("REBUILD {:.1}s", c.rebuild_timer);
        let dim = measure_text(&txt, None, 14, 1.0);
        draw_text(&txt, sx - dim.width * 0.5, sy - CATAPULT_HITBOX_H - 12.0, 14.0, ghost);
        return;
    }

    // Base (bois sombre).
    let base_col = Color::new(0.42, 0.30, 0.20, 1.0);
    draw_rectangle(
        sx - CATAPULT_HITBOX_W * 0.5,
        sy - 20.0,
        CATAPULT_HITBOX_W,
        20.0,
        base_col,
    );
    draw_rectangle_lines(
        sx - CATAPULT_HITBOX_W * 0.5,
        sy - 20.0,
        CATAPULT_HITBOX_W,
        20.0,
        1.5,
        ctx.colors.accent,
    );

    // Fulcrum.
    let fulcrum = Vec2::new(sx, sy - 20.0);

    // Bras : angle dérivé de la vélocité de tir courante.
    let angle = match c.launch_velocity() {
        Some(v) => v.y.atan2(v.x),  // en repère écran (y vers le bas)
        None => -0.4,
    };
    let arm_len = 38.0;
    let tip = fulcrum + Vec2::new(angle.cos() * arm_len, angle.sin() * arm_len);
    draw_line(fulcrum.x, fulcrum.y, tip.x, tip.y, 4.0, base_col);
    draw_circle(tip.x, tip.y, 5.0, Color::new(0.75, 0.55, 0.35, 1.0));

    // Contrepoids.
    let counter = fulcrum - Vec2::new(angle.cos() * 12.0, angle.sin() * 12.0);
    draw_circle(counter.x, counter.y, 5.0, Color::new(0.30, 0.22, 0.15, 1.0));

    // Barre de HP.
    let bar_w = CATAPULT_HITBOX_W + 10.0;
    let bar_h = 5.0;
    let bar_x = sx - bar_w * 0.5;
    let bar_y = sy - CATAPULT_HITBOX_H - 12.0;
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

fn draw_unit(ctx: &GameContext, u: &Unit, cam_x: f32, shake: Vec2) {
    let stats = match unit_stats(&u.kind) {
        Some(s) => s,
        None => return,
    };
    let size = stats.size_vec();
    let sx = u.pos.x - cam_x + shake.x;
    let sy = u.pos.y + shake.y;

    let attack_elapsed = if u.attack_cd_max > 0.0 {
        u.attack_cd_max - u.attack_cd
    } else {
        f32::MAX
    };
    let attacking = u.attack_cd > 0.0
        && (0.0..crate::stickman::ATTACK_DURATION).contains(&attack_elapsed);
    let pose = crate::stickman::auto_pose(u, attacking);

    let anim_phase = if attacking {
        attack_elapsed
    } else {
        u.pose_phase
    };

    let color = if u.hit_flash > 0.0 {
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

    // HP catapulte (petite barre séparée).
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

    // Indicateur ShotKind.
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