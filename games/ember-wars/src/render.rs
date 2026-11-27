//! Rendu complet d'une frame de jeu.

use crate::frame::{screen_height, screen_width};
use ember::prelude::*;
use glam::Vec2;

use std::collections::HashSet;

use crate::catapult::{Catapult, ShotKind};
use crate::components::{Projectile, Team, Tower, Unit};
use crate::config::{Colors, GameContext};
use crate::systems::{Game, Phase};
use crate::textures;
use crate::units::{all_units, can_spawn, unit_stats};
use ember_stdlib::fx;

/// The world under its nodes: sky, ground, towers and their HP bars
/// (`World::draw`). The catapults, units and projectiles draw themselves
/// next, as nodes.
pub fn draw_backdrop(gfx: &mut dyn RenderingServer, ctx: &GameContext, game: &Game, shake: Vec2) {
    let palette = game.palette;
    let cam_x = game.camera.x;
    let vh = screen_height();
    let gy = game.ground_y;
    let seed = game.visual_seed;

    textures::draw_sky(gfx, palette, gy);
    textures::draw_far_layer(gfx, palette, cam_x, gy, seed);
    textures::draw_mid_layer(gfx, palette, cam_x, gy, seed);
    textures::draw_mist(gfx, palette, gy);
    textures::draw_ground(gfx, palette, cam_x, gy, vh);

    textures::draw_tower(gfx, true, palette, game.player_tower.x, cam_x, gy, shake);
    textures::draw_tower(gfx, false, palette, game.enemy_tower.x, cam_x, gy, shake);

    draw_local_hp_bar(gfx, ctx, &game.player_tower, cam_x, gy, shake);
    draw_local_hp_bar(gfx, ctx, &game.enemy_tower, cam_x, gy, shake);
}

/// What draws over the nodes (`World::draw_after_children`): particles,
/// floating texts, the HUD and the screen flashes.
pub fn draw_overlay(
    gfx: &mut dyn RenderingServer,
    ctx: &GameContext,
    game: &Game,
    hud: &Hud,
    shake: Vec2,
) {
    let offset = shake - Vec2::new(game.camera.x, 0.0);
    fx::draw_particles(gfx, &game.juice, offset);
    fx::draw_floating_texts(gfx, &game.juice, offset);
    draw_hud(gfx, ctx, game, hud);
    let size = gfx.viewport().size();
    fx::draw_flashes(gfx, &game.juice, size);
}

/// A catapult node: the player's aim preview, then the catapult.
pub fn draw_catapult(
    gfx: &mut dyn RenderingServer,
    colors: &Colors,
    c: &Catapult,
    team: Team,
    cam_x: f32,
    shake: Vec2,
) {
    if team == Team::Player {
        draw_catapult_preview(gfx, colors, c, cam_x, shake);
    }
    draw_catapult_visual(gfx, colors, c, cam_x, shake, team);
}

/// A projectile node.
pub fn draw_projectile(gfx: &mut dyn RenderingServer, p: &Projectile, cam_x: f32, shake: Vec2) {
    let sx = p.pos.x - cam_x + shake.x;
    let sy = p.pos.y + shake.y;
    gfx.draw_circle(Vec2::new(sx, sy), p.radius, p.color);
}

/// What the HUD and the spawn bar read from the catapults and the units,
/// taken while the world holds them (they live in their nodes between
/// frames).
#[derive(Debug, Clone)]
pub struct Hud {
    pub catapult: CatapultHud,
    pub enemy_catapult: Option<CatapultHud>,
    /// Unit kinds the player has at their `max_alive` cap.
    pub at_cap: HashSet<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct CatapultHud {
    pub hp: f32,
    pub max_hp: f32,
    pub alive: bool,
    pub shot_kind: ShotKind,
}

impl CatapultHud {
    fn of(c: &Catapult) -> Self {
        Self {
            hp: c.hp,
            max_hp: c.max_hp,
            alive: c.is_alive(),
            shot_kind: c.shot_kind,
        }
    }
}

impl Hud {
    pub fn of(game: &Game) -> Self {
        Self {
            catapult: CatapultHud::of(&game.catapult),
            enemy_catapult: game.enemy_catapult.as_ref().map(CatapultHud::of),
            at_cap: all_units()
                .iter()
                .filter(|s| !can_spawn(&s.id, Team::Player, &game.units))
                .map(|s| s.id.clone())
                .collect(),
        }
    }
}

fn draw_local_hp_bar(
    gfx: &mut dyn RenderingServer,
    ctx: &GameContext,
    tower: &Tower,
    cam_x: f32,
    gy: f32,
    shake: Vec2,
) {
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
    gfx.draw_rect(
        Rect::new(hp_x, hp_y, hp_w, hp_h),
        Color::new(0.05, 0.05, 0.08, 1.0),
    );
    gfx.draw_rect(
        Rect::new(hp_x, hp_y, hp_w * tower.hp_fraction(), hp_h),
        hp_col,
    );
    gfx.draw_rect_outline(Rect::new(hp_x, hp_y, hp_w, hp_h), 1.0, ctx.colors.accent);

    let txt = format!("{:.0} / {:.0}", tower.hp.max(0.0), tower.max_hp);
    let dim = gfx.measure_text(&txt, 14.0);
    gfx.draw_text(
        &txt,
        Vec2::new(sx - dim.width * 0.5, hp_y - 5.0),
        14.0,
        ctx.colors.text,
    );
}

// ---------- Catapulte ----------

fn draw_catapult_visual(
    gfx: &mut dyn RenderingServer,
    colors: &Colors,
    c: &Catapult,
    cam_x: f32,
    shake: Vec2,
    team: Team,
) {
    let sx = c.pos.x - cam_x + shake.x;
    let sy = c.pos.y + shake.y;

    if c.is_rebuilding() {
        draw_catapult_ghost(gfx, c, sx, sy, team);
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
    gfx.draw_rect(Rect::new(base_x, base_y, base_w, base_h), wood);
    gfx.draw_rect_outline(Rect::new(base_x, base_y, base_w, base_h), 1.5, wood_dark);
    for i in 1..3 {
        let y = base_y + i as f32 * (base_h / 3.0);
        gfx.draw_line(
            Vec2::new(base_x + 3.0, y),
            Vec2::new(base_x + base_w - 3.0, y),
            0.8,
            wood_dark,
        );
    }
    gfx.draw_rect(
        Rect::new(base_x + 2.0, base_y + base_h - 2.5, base_w - 4.0, 2.0),
        accent,
    );

    // Roues
    let wheel_r = 4.5;
    let wheel_y = sy - wheel_r;
    gfx.draw_circle(Vec2::new(base_x + 7.0, wheel_y), wheel_r, wood_dark);
    gfx.draw_circle_outline(Vec2::new(base_x + 7.0, wheel_y), wheel_r, 1.0, wood_light);
    gfx.draw_circle(
        Vec2::new(base_x + base_w - 7.0, wheel_y),
        wheel_r,
        wood_dark,
    );
    gfx.draw_circle_outline(
        Vec2::new(base_x + base_w - 7.0, wheel_y),
        wheel_r,
        1.0,
        wood_light,
    );

    // A-frame
    let pivot_h = 45.0;
    let pivot = Vec2::new(sx, sy - base_h - pivot_h);

    gfx.draw_line(
        Vec2::new(base_x + 8.0, base_y),
        Vec2::new(pivot.x - 3.0, pivot.y),
        5.0,
        wood,
    );
    gfx.draw_line(
        Vec2::new(base_x + base_w - 8.0, base_y),
        Vec2::new(pivot.x + 3.0, pivot.y),
        5.0,
        wood,
    );
    let leg_mid_y = (base_y + pivot.y) * 0.5;
    let leg_a_x = (base_x + 8.0 + pivot.x - 3.0) * 0.5;
    let leg_b_x = (base_x + base_w - 8.0 + pivot.x + 3.0) * 0.5;
    gfx.draw_line(
        Vec2::new(leg_a_x, leg_mid_y),
        Vec2::new(leg_b_x, leg_mid_y),
        2.0,
        wood_dark,
    );

    // Plaque pivot
    let plate_w = 12.0;
    let plate_h = 12.0;
    gfx.draw_rect(
        Rect::new(
            pivot.x - plate_w * 0.5,
            pivot.y - plate_h * 0.5,
            plate_w,
            plate_h,
        ),
        metal,
    );
    gfx.draw_rect_outline(
        Rect::new(
            pivot.x - plate_w * 0.5,
            pivot.y - plate_h * 0.5,
            plate_w,
            plate_h,
        ),
        1.0,
        metal_dark,
    );
    for (dx, dy) in [(-3.0, -3.0), (3.0, -3.0), (-3.0, 3.0), (3.0, 3.0)] {
        gfx.draw_circle(Vec2::new(pivot.x + dx, pivot.y + dy), 1.1, metal_dark);
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

    gfx.draw_line(
        Vec2::new(counter_pos.x, counter_pos.y),
        Vec2::new(tip.x, tip.y),
        6.0,
        wood,
    );
    gfx.draw_line(
        Vec2::new(counter_pos.x, counter_pos.y),
        Vec2::new(tip.x, tip.y),
        2.0,
        wood_light,
    );

    // Contrepoids
    let cw = 13.0;
    gfx.draw_rect(
        Rect::new(counter_pos.x - cw * 0.5, counter_pos.y - cw * 0.5, cw, cw),
        wood_dark,
    );
    gfx.draw_rect_outline(
        Rect::new(counter_pos.x - cw * 0.5, counter_pos.y - cw * 0.5, cw, cw),
        1.5,
        wood_light,
    );
    gfx.draw_line(
        Vec2::new(counter_pos.x - cw * 0.5, counter_pos.y - cw * 0.5),
        Vec2::new(counter_pos.x + cw * 0.5, counter_pos.y + cw * 0.5),
        1.0,
        wood_light,
    );
    gfx.draw_line(
        Vec2::new(counter_pos.x - cw * 0.5, counter_pos.y + cw * 0.5),
        Vec2::new(counter_pos.x + cw * 0.5, counter_pos.y - cw * 0.5),
        1.0,
        wood_light,
    );

    // Fronde
    let sling_len = 14.0;
    let sling_bottom = Vec2::new(tip.x, tip.y + sling_len);
    gfx.draw_line(
        Vec2::new(tip.x, tip.y),
        Vec2::new(sling_bottom.x, sling_bottom.y),
        1.5,
        rope,
    );
    gfx.draw_circle(
        Vec2::new(sling_bottom.x, sling_bottom.y),
        5.0,
        Color::new(0.55, 0.42, 0.28, 1.0),
    );
    gfx.draw_circle_outline(
        Vec2::new(sling_bottom.x, sling_bottom.y),
        5.0,
        1.0,
        wood_dark,
    );
    gfx.draw_circle(
        Vec2::new(sling_bottom.x, sling_bottom.y),
        2.0,
        Color::new(0.85, 0.85, 0.85, 1.0),
    );

    // Barre HP
    let bar_w = 62.0;
    let bar_h = 5.0;
    let bar_x = sx - bar_w * 0.5;
    let bar_y = pivot.y - 26.0;
    gfx.draw_rect(
        Rect::new(bar_x, bar_y, bar_w, bar_h),
        Color::new(0.05, 0.05, 0.08, 1.0),
    );
    gfx.draw_rect(
        Rect::new(bar_x, bar_y, bar_w * c.hp_fraction(), bar_h),
        Color::new(0.95, 0.55, 0.30, 1.0),
    );
    gfx.draw_rect_outline(Rect::new(bar_x, bar_y, bar_w, bar_h), 1.0, colors.accent);
}

fn draw_catapult_ghost(gfx: &mut dyn RenderingServer, c: &Catapult, sx: f32, sy: f32, team: Team) {
    let ghost = Color::new(0.35, 0.35, 0.40, 0.4);
    let accent = match team {
        Team::Player => Color::new(0.35, 0.65, 1.0, 0.35),
        Team::Enemy => Color::new(1.0, 0.35, 0.35, 0.35),
    };

    let base_w = 90.0;
    let base_h = 10.0;
    let base_x = sx - base_w * 0.5;
    let base_y = sy - base_h;
    gfx.draw_rect(Rect::new(base_x, base_y, base_w, base_h), ghost);

    let pivot_h = 45.0;
    let pivot = Vec2::new(sx, sy - base_h - pivot_h);
    gfx.draw_line(
        Vec2::new(base_x + 8.0, base_y),
        Vec2::new(pivot.x - 3.0, pivot.y),
        5.0,
        ghost,
    );
    gfx.draw_line(
        Vec2::new(base_x + base_w - 8.0, base_y),
        Vec2::new(pivot.x + 3.0, pivot.y),
        5.0,
        ghost,
    );

    gfx.draw_rect(
        Rect::new(base_x + 2.0, base_y + base_h - 2.5, base_w - 4.0, 2.0),
        accent,
    );

    let txt = format!("REBUILD {:.1}s", c.rebuild_remaining());
    let dim = gfx.measure_text(&txt, 14.0);
    gfx.draw_text(
        &txt,
        Vec2::new(sx - dim.width * 0.5, pivot.y - 12.0),
        14.0,
        ghost,
    );
}

fn draw_catapult_preview(
    gfx: &mut dyn RenderingServer,
    colors: &Colors,
    c: &Catapult,
    cam_x: f32,
    shake: Vec2,
) {
    if !c.is_alive() {
        return;
    }
    let points = c.preview_points(24);
    if points.len() < 2 {
        return;
    }
    let accent = colors.accent;
    let n = points.len() as f32;
    for (i, p) in points.iter().enumerate() {
        let alpha = 1.0 - (i as f32 / n);
        let sx = p.x - cam_x + shake.x;
        let sy = p.y + shake.y;
        let r = 2.5 * alpha + 0.8;
        let col = Color::new(accent.r, accent.g, accent.b, 0.65 * alpha);
        gfx.draw_circle(Vec2::new(sx, sy), r, col);
    }
}

// ---------- Unités ----------

/// A unit node.
pub fn draw_unit(
    gfx: &mut dyn RenderingServer,
    colors: &Colors,
    u: &Unit,
    cam_x: f32,
    shake: Vec2,
) {
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
        Color::WHITE
    } else {
        match u.team {
            Team::Player => tint(stats.color(), colors.player, 0.3),
            Team::Enemy => tint(stats.color(), colors.enemy, 0.3),
        }
    };

    let facing = match u.team {
        Team::Player => 1.0,
        Team::Enemy => -1.0,
    };

    crate::stickman::draw(
        gfx,
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
        gfx.draw_rect(
            Rect::new(bx, by, bar_w, bar_h),
            Color::new(0.1, 0.1, 0.12, 1.0),
        );
        let hp_col = if u.team == Team::Player {
            colors.hp_player
        } else {
            colors.hp_enemy
        };
        gfx.draw_rect(Rect::new(bx, by, bar_w * u.hp_fraction(), bar_h), hp_col);
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

fn draw_hud(gfx: &mut dyn RenderingServer, ctx: &GameContext, game: &Game, hud: &Hud) {
    let x = 16.0;
    let y = 16.0;
    let w = 220.0;
    let h = 20.0;
    gfx.draw_rect(Rect::new(x, y, w, h), Color::new(0.1, 0.1, 0.12, 1.0));
    gfx.draw_rect(
        Rect::new(x, y, w * game.player_mana.fraction(), h),
        ctx.colors.mana,
    );
    gfx.draw_rect_outline(Rect::new(x, y, w, h), 1.5, ctx.colors.accent);
    let mana_txt = format!(
        "MANA {:.0}/{:.0}",
        game.player_mana.current, game.player_mana.max
    );
    gfx.draw_text(
        &mana_txt,
        Vec2::new(x + 6.0, y + 15.0),
        13.0,
        ctx.colors.text,
    );

    let gold_txt = format!(
        "GOLD {:.0}   KILLS {}   {:.1}s",
        game.player_upgrades.gold, game.stats.kills, game.stats.elapsed
    );
    gfx.draw_text(&gold_txt, Vec2::new(x, y + 40.0), 14.0, ctx.colors.gold);

    let cat_txt = format!(
        "CATAPULT  {:.0}/{:.0}",
        hud.catapult.hp.max(0.0),
        hud.catapult.max_hp
    );
    let cat_col = if hud.catapult.alive {
        Color::new(0.95, 0.55, 0.30, 1.0)
    } else {
        Color::new(0.6, 0.35, 0.30, 1.0)
    };
    gfx.draw_text(&cat_txt, Vec2::new(x, y + 60.0), 13.0, cat_col);

    if let Some(ec) = &hud.enemy_catapult {
        let ec_txt = format!("ENEMY CAT  {:.0}/{:.0}", ec.hp.max(0.0), ec.max_hp);
        let ec_col = if ec.alive {
            Color::new(1.0, 0.40, 0.40, 1.0)
        } else {
            Color::new(0.6, 0.35, 0.35, 1.0)
        };
        gfx.draw_text(&ec_txt, Vec2::new(x, y + 78.0), 13.0, ec_col);
    }

    let title = "EMBER WARS";
    let dim = gfx.measure_text(title, 22.0);
    let tx = screen_width() - dim.width - 20.0;
    gfx.draw_text(title, Vec2::new(tx, 30.0), 22.0, ctx.colors.accent);

    let level_hint = format!("{} — {}", game.level.id.to_uppercase(), game.level.name);
    let dim2 = gfx.measure_text(&level_hint, 15.0);
    gfx.draw_text(
        &level_hint,
        Vec2::new(screen_width() - dim2.width - 20.0, 54.0),
        15.0,
        ctx.colors.text,
    );

    if let Some(remaining) = game.survive_remaining() {
        let txt = format!("SURVIVE  {:.0}s", remaining.ceil());
        let dim = gfx.measure_text(&txt, 20.0);
        gfx.draw_text(
            &txt,
            Vec2::new(screen_width() * 0.5 - dim.width * 0.5, 66.0),
            20.0,
            ctx.colors.accent,
        );
    }

    let kind_label = hud.catapult.shot_kind.label();
    let kind_txt = format!("[1/2/3] {}", kind_label);
    let dim3 = gfx.measure_text(&kind_txt, 14.0);
    gfx.draw_text(
        &kind_txt,
        Vec2::new(screen_width() - dim3.width - 20.0, 80.0),
        14.0,
        ctx.colors.text,
    );
}

pub fn draw_end_overlay(gfx: &mut dyn RenderingServer, ctx: &GameContext, game: &Game) {
    if game.phase == Phase::Playing {
        return;
    }
    let vw = screen_width();
    let vh = screen_height();
    gfx.draw_rect(Rect::new(0.0, 0.0, vw, vh), Color::new(0.0, 0.0, 0.0, 0.65));

    let (title, color) = match game.phase {
        Phase::Won => ("VICTORY", ctx.colors.hp_player),
        Phase::Lost => ("DEFEAT", ctx.colors.hp_enemy),
        Phase::Playing => unreachable!(),
    };
    let dim = gfx.measure_text(title, 72.0);
    gfx.draw_text(
        title,
        Vec2::new(vw * 0.5 - dim.width * 0.5, vh * 0.4),
        72.0,
        color,
    );

    let sub = format!(
        "Kills {}  |  Gold {:.0}  |  {:.1}s",
        game.stats.kills, game.stats.gold_earned, game.stats.elapsed
    );
    let dim2 = gfx.measure_text(&sub, 24.0);
    gfx.draw_text(
        &sub,
        Vec2::new(vw * 0.5 - dim2.width * 0.5, vh * 0.4 + 44.0),
        24.0,
        ctx.colors.text,
    );

    let hint = "Press R to retry, Enter for menu";
    let dim3 = gfx.measure_text(hint, 20.0);
    gfx.draw_text(
        hint,
        Vec2::new(vw * 0.5 - dim3.width * 0.5, vh * 0.4 + 90.0),
        20.0,
        ctx.colors.text,
    );
}
