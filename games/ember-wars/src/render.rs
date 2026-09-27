//! Rendu complet d'une frame de jeu.

use glam::Vec2;
use macroquad::prelude::*;

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

    draw_turret(ctx, game, cam_x, shake);

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

fn draw_turret(ctx: &GameContext, game: &Game, cam_x: f32, shake: Vec2) {
    let t = &game.turret;
    let sx = t.pos.x - cam_x + shake.x;
    let sy = t.pos.y + shake.y;
    let base_r = 18.0;
    draw_circle(sx, sy, base_r, ctx.colors.turret);
    draw_circle_lines(sx, sy, base_r, 2.0, ctx.colors.accent);

    let dir = Vec2::new(t.angle.cos(), t.angle.sin());
    let tip = Vec2::new(sx, sy) + dir * 30.0;
    draw_line(sx, sy, tip.x, tip.y, 5.0, ctx.colors.turret);

    if t.multi_shot >= 2 {
        let perp = Vec2::new(-dir.y, dir.x) * 5.0;
        let tip2 = Vec2::new(sx, sy) + perp + dir * 30.0;
        let tip3 = Vec2::new(sx, sy) - perp + dir * 30.0;
        draw_line(sx + perp.x, sy + perp.y, tip2.x, tip2.y, 3.0, ctx.colors.turret);
        draw_line(sx - perp.x, sy - perp.y, tip3.x, tip3.y, 3.0, ctx.colors.turret);
    }

    let label = t.shot_kind.label();
    let dim = measure_text(label, None, 14, 1.0);
    draw_text(label, sx - dim.width * 0.5, sy - 30.0, 14.0, ctx.colors.text);
}

fn draw_unit(ctx: &GameContext, u: &Unit, cam_x: f32, shake: Vec2) {
    let stats = match unit_stats(&u.kind) {
        Some(s) => s,
        None => return,
    };
    let size = stats.size_vec();
    let sx = u.pos.x - cam_x + shake.x;
    let sy = u.pos.y + shake.y;

    // Détection de l'attaque : l'attack_cd vient d'être armé, c'est-à-dire
    // que le temps écoulé (attack_cd_max - attack_cd) est inférieur à la
    // durée d'animation d'attaque.
    let attack_elapsed = if u.attack_cd_max > 0.0 {
        u.attack_cd_max - u.attack_cd
    } else {
        f32::MAX
    };
    let attacking = u.attack_cd > 0.0
        && (0.0..crate::stickman::ATTACK_DURATION).contains(&attack_elapsed);
    let pose = crate::stickman::auto_pose(u, attacking);

    // Phase d'animation : attack_elapsed quand attaque, pose_phase sinon.
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