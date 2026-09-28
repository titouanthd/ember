//! Targeting, attaques, projectiles, dégâts, juice.

use glam::Vec2;
use macroquad::prelude::Color;

use crate::catapult::{Catapult, CATAPULT_HITBOX_H};
use crate::components::{
    rgba, AttackTarget, PendingAttack, Projectile, ProjectileKind, Team, Tower, Unit, UnitId,
};
use crate::juice::{HIT_FLASH_DURATION, Juice};
use crate::units::{unit_stats, UnitStats};

pub const MAX_PROJECTILES: usize = 200;
pub const HEALER_MIDLINE_MARGIN: f32 = 300.0;

const DMG_COLOR_PLAYER: Color = Color::new(0.75, 0.92, 1.00, 1.0);
const DMG_COLOR_ENEMY: Color = Color::new(1.00, 0.72, 0.72, 1.0);
const HEAL_COLOR: Color = Color::new(0.55, 1.00, 0.65, 1.0);

fn unit_radius(unit: &Unit) -> f32 {
    unit_stats(&unit.kind)
        .map(|s| s.size.0.max(s.size.1) * 0.5)
        .unwrap_or(10.0)
}

fn team_juice_color(team: Team) -> Color {
    match team {
        Team::Player => Color::new(0.45, 0.75, 1.0, 1.0),
        Team::Enemy => Color::new(1.0, 0.45, 0.40, 1.0),
    }
}

/// Bundle des deux catapultes (joueur toujours présente, ennemie optionnelle).
type Catapults<'a> = (&'a mut Catapult, &'a mut Option<Catapult>);
type Towers<'a> = (&'a mut Tower, &'a mut Tower);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Unit(UnitId),
    Tower(Team),
    Catapult(Team),
}

pub fn acquire_target(
    unit: &Unit,
    units: &[Unit],
    player_tower: &Tower,
    enemy_tower: &Tower,
    player_catapult: &Catapult,
    enemy_catapult: Option<&Catapult>,
) -> Option<Target> {
    let stats = unit_stats(&unit.kind)?;
    let attack_range = stats.attack_range;
    if attack_range <= 0.0 {
        return None;
    }

    let enemy_team = unit.team.opponent();

    // 1. Unités ennemies à portée.
    let mut best: Option<(f32, UnitId)> = None;
    for other in units {
        if other.team != enemy_team || !other.is_alive() || other.id == unit.id {
            continue;
        }
        let dist = (other.pos - unit.pos).length();
        if dist > attack_range {
            continue;
        }
        if best.is_none_or(|(d, _)| dist < d) {
            best = Some((dist, other.id));
        }
    }
    if let Some((_, id)) = best {
        return Some(Target::Unit(id));
    }

    // 2. Structures ennemies (tour + catapulte).
    let tower = if enemy_team == Team::Player {
        player_tower
    } else {
        enemy_tower
    };
    let tower_dist = (tower.x - unit.pos.x).abs();
    let tower_in_range = tower_dist <= attack_range;

    let catapult_ref: Option<&Catapult> = if enemy_team == Team::Player {
        Some(player_catapult)
    } else {
        enemy_catapult
    };

    let (catapult_in_range, catapult_dist) = if let Some(cat) = catapult_ref
        && cat.is_alive()
    {
        let d = (cat.pos.x - unit.pos.x).abs();
        (d <= attack_range, d)
    } else {
        (false, f32::INFINITY)
    };

    if catapult_in_range && (!tower_in_range || catapult_dist < tower_dist) {
        return Some(Target::Catapult(enemy_team));
    }
    if tower_in_range {
        return Some(Target::Tower(enemy_team));
    }

    None
}

fn advance_toward_tower(unit: &mut Unit, speed: f32, dt: f32) {
    let dir = if unit.team == Team::Player { 1.0 } else { -1.0 };
    unit.pos.x += dir * speed * dt;
}

fn speed_mult(team: Team, unit_speed_mult: f32) -> f32 {
    if team == Team::Player {
        unit_speed_mult
    } else {
        1.0
    }
}

pub fn strike_delay() -> f32 {
    crate::stickman::ATTACK_DURATION * crate::stickman::WINDUP_END
}

fn tick_unit_timers(units: &mut [Unit], dt: f32) {
    for u in units.iter_mut() {
        u.attack_cd.tick(dt);
        u.heal_cd.tick(dt);
        u.hit_flash.tick(dt);
        u.pose_phase += dt;

        // Le pending attack n'est PAS consommé ici : c'est la Phase 1
        // qui l'applique et le clear. `time_to_hit` reste un f32 brut,
        // c'est un sous-compteur dans un concept plus large.
        if let Some(mut pa) = u.pending_attack {
            pa.time_to_hit -= dt;
            u.pending_attack = Some(pa);
        }
    }
}

fn apply_damage_to_unit(unit: &mut Unit, amount: f32, source_team: Team, juice: &mut Juice) {
    let was_alive = unit.is_alive();
    unit.hp -= amount;
    unit.hit_flash.trigger(HIT_FLASH_DURATION);

    let color = team_juice_color(unit.team);
    juice.spawn_hit_spark(unit.pos, color);

    let dmg_color = if source_team == Team::Player {
        DMG_COLOR_PLAYER
    } else {
        DMG_COLOR_ENEMY
    };
    let text_pos = unit.pos + Vec2::new(0.0, -25.0);
    juice.spawn_floating_text(text_pos, format!("-{:.0}", amount), dmg_color);

    if was_alive && !unit.is_alive() {
        juice.spawn_death_burst(unit.pos, color, 6);
    }
}

fn apply_damage_to_tower(tower: &mut Tower, amount: f32, ground_y: f32, juice: &mut Juice) {
    tower.hp -= amount;
    let frac = if tower.max_hp > 0.0 {
        (amount / tower.max_hp).clamp(0.0, 1.0)
    } else {
        0.0
    };
    juice.shake(4.0 + frac * 30.0, 0.12);
    let color = team_juice_color(tower.team);
    juice.spawn_hit_spark(Vec2::new(tower.x, ground_y - 120.0), color);

    let dmg_color = if tower.team == Team::Enemy {
        DMG_COLOR_PLAYER
    } else {
        DMG_COLOR_ENEMY
    };
    juice.spawn_floating_text(
        Vec2::new(tower.x, ground_y - 200.0),
        format!("-{:.0}", amount),
        dmg_color,
    );
}

fn apply_damage_to_catapult(catapult: &mut Catapult, amount: f32, juice: &mut Juice) {
    let was_alive = catapult.is_alive();
    catapult.take_damage(amount);
    let pos = catapult.pos;
    juice.spawn_hit_spark(pos, Color::new(1.0, 0.6, 0.3, 1.0));
    juice.shake(3.0, 0.1);

    juice.spawn_floating_text(
        pos + Vec2::new(0.0, -CATAPULT_HITBOX_H - 12.0),
        format!("-{:.0}", amount),
        DMG_COLOR_ENEMY,
    );

    if was_alive && !catapult.is_alive() {
        juice.spawn_death_burst(pos, Color::new(1.0, 0.5, 0.2, 1.0), 12);
        juice.shake(10.0, 0.4);
    }
}

fn detonate_bomber(
    units: &mut [Unit],
    bomber_idx: usize,
    pos: Vec2,
    team: Team,
    damage: f32,
    radius: f32,
    juice: &mut Juice,
) {
    let enemy_team = team.opponent();
    for u in units.iter_mut() {
        if u.team != enemy_team || !u.is_alive() {
            continue;
        }
        if (u.pos - pos).length() <= radius {
            apply_damage_to_unit(u, damage, team, juice);
        }
    }
    units[bomber_idx].hp = 0.0;
    juice.spawn_death_burst(pos, team_juice_color(team), 10);
    juice.shake(10.0, 0.2);
}

fn healer_can_advance(unit: &Unit, player_tower: &Tower, enemy_tower: &Tower) -> bool {
    match unit.team {
        Team::Player => unit.pos.x < enemy_tower.x - HEALER_MIDLINE_MARGIN,
        Team::Enemy => unit.pos.x > player_tower.x + HEALER_MIDLINE_MARGIN,
    }
}

struct StrikeContext {
    attacker_idx: usize,
    attacker_team: Team,
    attacker_pos: Vec2,
    damage: f32,
    target: AttackTarget,
}

fn apply_pending_attack(
    units: &mut [Unit],
    towers: Towers,
    catapults: Catapults,
    projectiles: &mut Vec<Projectile>,
    juice: &mut Juice,
    ctx: StrikeContext,
    kind_stats: &UnitStats,
) {
    let (player_tower, enemy_tower) = towers;
    let (player_catapult, enemy_catapult) = catapults;
    let ground_y = player_catapult.pos.y;

    match ctx.target {
        AttackTarget::Unit(id) => {
            if let Some(pstats) = &kind_stats.projectile {
                if let Some(tpos) = units.iter().find(|u| u.id == id).map(|u| u.pos)
                    && projectiles.len() < MAX_PROJECTILES
                {
                    let dir = (tpos - ctx.attacker_pos).normalize_or_zero();
                    projectiles.push(Projectile {
                        pos: ctx.attacker_pos,
                        vel: dir * pstats.speed,
                        gravity: 0.0,
                        damage: ctx.damage,
                        radius: pstats.radius,
                        team: ctx.attacker_team,
                        kind: ProjectileKind::Archer,
                        pierce_remaining: 0,
                        color: rgba(pstats.color),
                    });
                }
            } else if let Some(radius) = kind_stats.explosion_radius {
                detonate_bomber(
                    units,
                    ctx.attacker_idx,
                    ctx.attacker_pos,
                    ctx.attacker_team,
                    ctx.damage,
                    radius,
                    juice,
                );
            } else if let Some(j) = units.iter().position(|u| u.id == id) {
                apply_damage_to_unit(&mut units[j], ctx.damage, ctx.attacker_team, juice);
            }
        }
        AttackTarget::Tower(t) => {
            let tower_x = if t == Team::Player {
                player_tower.x
            } else {
                enemy_tower.x
            };
            if let Some(pstats) = &kind_stats.projectile {
                if projectiles.len() < MAX_PROJECTILES {
                    let tpos = Vec2::new(tower_x, ctx.attacker_pos.y);
                    let dir = (tpos - ctx.attacker_pos).normalize_or_zero();
                    projectiles.push(Projectile {
                        pos: ctx.attacker_pos,
                        vel: dir * pstats.speed,
                        gravity: 0.0,
                        damage: ctx.damage,
                        radius: pstats.radius,
                        team: ctx.attacker_team,
                        kind: ProjectileKind::Archer,
                        pierce_remaining: 0,
                        color: rgba(pstats.color),
                    });
                }
            } else if let Some(radius) = kind_stats.explosion_radius {
                detonate_bomber(
                    units,
                    ctx.attacker_idx,
                    ctx.attacker_pos,
                    ctx.attacker_team,
                    ctx.damage,
                    radius,
                    juice,
                );
                let tower: &mut Tower = match t {
                    Team::Player => &mut *player_tower,
                    Team::Enemy => &mut *enemy_tower,
                };
                apply_damage_to_tower(tower, ctx.damage, ground_y, juice);
            } else {
                let tower: &mut Tower = match t {
                    Team::Player => &mut *player_tower,
                    Team::Enemy => &mut *enemy_tower,
                };
                apply_damage_to_tower(tower, ctx.damage, ground_y, juice);
            }
        }
        AttackTarget::Catapult(t) => {
            if let Some(radius) = kind_stats.explosion_radius {
                detonate_bomber(
                    units,
                    ctx.attacker_idx,
                    ctx.attacker_pos,
                    ctx.attacker_team,
                    ctx.damage,
                    radius,
                    juice,
                );
            }
            match t {
                Team::Player => apply_damage_to_catapult(player_catapult, ctx.damage, juice),
                Team::Enemy => {
                    if let Some(ec) = enemy_catapult.as_mut() {
                        apply_damage_to_catapult(ec, ctx.damage, juice);
                    }
                }
            }
        }
    }
}

pub fn resolve_attacks(
    units: &mut [Unit],
    towers: Towers,
    catapults: Catapults,
    projectiles: &mut Vec<Projectile>,
    juice: &mut Juice,
    dt: f32,
    unit_speed_mult: f32,
) {
    let (player_tower, enemy_tower) = towers;
    tick_unit_timers(units, dt);

    // Phase 1 : pending attacks
    let mut to_apply: Vec<(usize, PendingAttack, Vec2, Team)> = Vec::new();
    for (i, u) in units.iter().enumerate() {
        if let Some(pa) = u.pending_attack
            && pa.time_to_hit <= 0.0
        {
            to_apply.push((i, pa, u.pos, u.team));
        }
    }
    for (i, pa, attacker_pos, attacker_team) in to_apply {
        if !units[i].is_alive() {
            units[i].pending_attack = None;
            continue;
        }
        let kind = units[i].kind.clone();
        if let Some(stats) = unit_stats(&kind) {
            let ctx = StrikeContext {
                attacker_idx: i,
                attacker_team,
                attacker_pos,
                damage: pa.damage,
                target: pa.target,
            };
            apply_pending_attack(
                units,
                (&mut *player_tower, &mut *enemy_tower),
                (&mut *catapults.0, catapults.1),
                projectiles,
                juice,
                ctx,
                stats,
            );
        }
        units[i].pending_attack = None;
    }

    // Phase 2 : décision + mouvement
    for i in 0..units.len() {
        if !units[i].is_alive() {
            continue;
        }
        let kind = units[i].kind.clone();
        let stats = match unit_stats(&kind) {
            Some(s) => s,
            None => continue,
        };

        if stats.is_healer() {
            let healed = resolve_healer(units, i, stats, juice);
            if !healed && healer_can_advance(&units[i], player_tower, enemy_tower) {
                let mult = speed_mult(units[i].team, unit_speed_mult);
                advance_toward_tower(&mut units[i], stats.speed * mult, dt);
            }
            continue;
        }

        let target = acquire_target(
            &units[i],
            units,
            player_tower,
            enemy_tower,
            catapults.0,
            catapults.1.as_ref(),
        );
        units[i].target = match target {
            Some(Target::Unit(id)) => Some(id),
            _ => None,
        };

        if target.is_none() {
            let mult = speed_mult(units[i].team, unit_speed_mult);
            advance_toward_tower(&mut units[i], stats.speed * mult, dt);
            continue;
        }

        if units[i].pending_attack.is_some() || units[i].attack_cd.is_active() {
            continue;
        }

        let at = match target.unwrap() {
            Target::Unit(id) => AttackTarget::Unit(id),
            Target::Tower(t) => AttackTarget::Tower(t),
            Target::Catapult(t) => AttackTarget::Catapult(t),
        };

        let effective_cd = stats
            .attack_cooldown
            .max(crate::stickman::ATTACK_DURATION);
        units[i].pending_attack = Some(PendingAttack {
            target: at,
            damage: units[i].damage,
            time_to_hit: strike_delay(),
        });
        units[i].attack_cd.trigger(effective_cd);
    }
}

fn resolve_healer(units: &mut [Unit], i: usize, stats: &UnitStats, juice: &mut Juice) -> bool {
    let heal_range = stats.heal_range.unwrap_or(0.0);
    let heal_amount = stats.heal.unwrap_or(0.0);
    let heal_cooldown = stats.heal_cooldown.unwrap_or(1.0);

    let team = units[i].team;
    let pos = units[i].pos;

    let mut best: Option<(f32, usize)> = None;
    for (j, other) in units.iter().enumerate() {
        if j == i || other.team != team || !other.is_alive() {
            continue;
        }
        if other.hp >= other.max_hp {
            continue;
        }
        let dist = (other.pos - pos).length();
        if dist > heal_range {
            continue;
        }
        let frac = other.hp_fraction();
        if best.is_none_or(|(f, _)| frac < f) {
            best = Some((frac, j));
        }
    }

    if let Some((_, j)) = best {
        if units[i].heal_cd.is_ready() {
            let missing = units[j].max_hp - units[j].hp;
            let actual = heal_amount.min(missing);
            units[j].hp = (units[j].hp + heal_amount).min(units[j].max_hp);
            units[i].heal_cd.trigger(heal_cooldown);
            let text_pos = units[j].pos + Vec2::new(0.0, -25.0);
            juice.spawn_floating_text(text_pos, format!("+{:.0}", actual), HEAL_COLOR);
        }
        true
    } else {
        false
    }
}

pub fn resolve_projectiles(
    projectiles: &mut Vec<Projectile>,
    units: &mut [Unit],
    towers: Towers,
    catapults: Catapults,
    ground_y: f32,
    juice: &mut Juice,
    dt: f32,
) {
    let (player_tower, enemy_tower) = towers;
    let (player_catapult, enemy_catapult) = catapults;
    let mut i = 0;
    while i < projectiles.len() {
        let g = projectiles[i].gravity;
        if g != 0.0 {
            projectiles[i].vel.y += g * dt;
        }
        let vel = projectiles[i].vel;
        projectiles[i].pos += vel * dt;

        let enemy_team = projectiles[i].team.opponent();
        let proj_team = projectiles[i].team;
        let proj_pos = projectiles[i].pos;
        let proj_radius = projectiles[i].radius;
        let proj_damage = projectiles[i].damage;
        let pierce_remaining = projectiles[i].pierce_remaining;
        let kind = projectiles[i].kind;

        if g != 0.0 && proj_pos.y > ground_y + 80.0 {
            projectiles.remove(i);
            continue;
        }

        let mut hit_index: Option<usize> = None;
        for (j, u) in units.iter().enumerate() {
            if u.team != enemy_team || !u.is_alive() {
                continue;
            }
            let dist = (u.pos - proj_pos).length();
            if dist <= proj_radius + unit_radius(u) {
                hit_index = Some(j);
                break;
            }
        }

        if let Some(j) = hit_index {
            apply_damage_to_unit(&mut units[j], proj_damage, proj_team, juice);

            if matches!(kind, ProjectileKind::Explosive) {
                const EXPLOSION_RADIUS: f32 = 50.0;
                let splash = proj_damage * 0.5;
                let mut hits: Vec<usize> = Vec::new();
                for (k, u) in units.iter().enumerate() {
                    if k == j || u.team != enemy_team || !u.is_alive() {
                        continue;
                    }
                    if (u.pos - proj_pos).length() <= EXPLOSION_RADIUS {
                        hits.push(k);
                    }
                }
                for k in hits {
                    apply_damage_to_unit(&mut units[k], splash, proj_team, juice);
                }
                juice.shake(3.0, 0.1);
            }

            if pierce_remaining > 1 {
                projectiles[i].pierce_remaining = pierce_remaining - 1;
                i += 1;
            } else {
                projectiles.remove(i);
            }
            continue;
        }

        let tower_x = if enemy_team == Team::Player {
            player_tower.x
        } else {
            enemy_tower.x
        };
        if (proj_pos.x - tower_x).abs() <= 20.0 {
            let tower: &mut Tower = match enemy_team {
                Team::Player => &mut *player_tower,
                Team::Enemy => &mut *enemy_tower,
            };
            apply_damage_to_tower(tower, proj_damage, ground_y, juice);
            projectiles.remove(i);
            continue;
        }

        // Catapulte du camp visé.
        let hit_catapult = if enemy_team == Team::Player {
            let c = &*player_catapult;
            if c.is_alive() {
                let (rx, ry, rw, rh) = c.rect();
                proj_pos.x >= rx && proj_pos.x <= rx + rw
                    && proj_pos.y >= ry && proj_pos.y <= ry + rh
            } else {
                false
            }
        } else {
            match enemy_catapult.as_ref() {
                Some(c) if c.is_alive() => {
                    let (rx, ry, rw, rh) = c.rect();
                    proj_pos.x >= rx && proj_pos.x <= rx + rw
                        && proj_pos.y >= ry && proj_pos.y <= ry + rh
                }
                _ => false,
            }
        };

        if hit_catapult {
            match enemy_team {
                Team::Player => apply_damage_to_catapult(player_catapult, proj_damage, juice),
                Team::Enemy => {
                    if let Some(ec) = enemy_catapult.as_mut() {
                        apply_damage_to_catapult(ec, proj_damage, juice);
                    }
                }
            }
            projectiles.remove(i);
            continue;
        }

        if proj_pos.x < -500.0 || proj_pos.x > 100_000.0 {
            projectiles.remove(i);
            continue;
        }

        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catapult::Catapult;
    use ember_stdlib::time::Cooldown;

    fn mk_unit(id: u32, kind: &str, team: Team, x: f32) -> Unit {
        let stats = unit_stats(kind).expect("kind exists");
        Unit {
            id: UnitId(id),
            kind: kind.to_owned(),
            team,
            pos: Vec2::new(x, 0.0),
            hp: stats.hp,
            max_hp: stats.hp,
            damage: stats.damage,
            attack_cd: Cooldown::new(stats.attack_cooldown),
            heal_cd: Cooldown::new(stats.heal_cooldown.unwrap_or(0.0)),
            hit_flash: Cooldown::default(),
            pose_phase: 0.0,
            target: None,
            pending_attack: None,
        }
    }

    fn mk_tower(team: Team, x: f32) -> Tower {
        Tower {
            team,
            hp: 500.0,
            max_hp: 500.0,
            x,
        }
    }

    fn mk_catapult(x: f32) -> Catapult {
        Catapult::new(Vec2::new(x, 100.0))
    }

    fn no_enemy_catapult() -> Option<Catapult> {
        None
    }

    fn run_attacks_for(
        units: &mut [Unit],
        pt: &mut Tower,
        et: &mut Tower,
        catapults: Catapults,
        projs: &mut Vec<Projectile>,
        juice: &mut Juice,
        seconds: f32,
    ) {
        let dt = 1.0 / 60.0;
        let steps = (seconds / dt).ceil() as u32;
        for _ in 0..steps {
            resolve_attacks(
                units,
                (&mut *pt, &mut *et),
                (&mut *catapults.0, &mut *catapults.1),
                projs,
                juice,
                dt,
                1.0,
            );
        }
    }

    // ---------- acquire_target ----------

    #[test]
    fn acquire_target_returns_none_for_healer() {
        let healer = mk_unit(0, "healer", Team::Player, 0.0);
        let enemy = mk_unit(1, "grunt", Team::Enemy, 5.0);
        let units = vec![enemy];
        let pt = mk_tower(Team::Player, -100.0);
        let et = mk_tower(Team::Enemy, 100.0);
        let cat = mk_catapult(-50.0);
        assert!(acquire_target(&healer, &units, &pt, &et, &cat, None).is_none());
    }

    #[test]
    fn acquire_target_finds_nearest_enemy_in_range() {
        let shooter = mk_unit(0, "grunt", Team::Player, 0.0);
        let e1 = mk_unit(1, "grunt", Team::Enemy, 20.0);
        let e2 = mk_unit(2, "grunt", Team::Enemy, 10.0);
        let units = vec![e1, e2];
        let pt = mk_tower(Team::Player, -100.0);
        let et = mk_tower(Team::Enemy, 1000.0);
        let cat = mk_catapult(-50.0);
        assert_eq!(
            acquire_target(&shooter, &units, &pt, &et, &cat, None),
            Some(Target::Unit(UnitId(2)))
        );
    }

    #[test]
    fn acquire_target_ignores_enemies_out_of_range() {
        let shooter = mk_unit(0, "grunt", Team::Player, 0.0);
        let e = mk_unit(1, "grunt", Team::Enemy, 500.0);
        let units = vec![e];
        let pt = mk_tower(Team::Player, -100.0);
        let et = mk_tower(Team::Enemy, 1000.0);
        let cat = mk_catapult(-50.0);
        assert!(acquire_target(&shooter, &units, &pt, &et, &cat, None).is_none());
    }

    #[test]
    fn acquire_target_ignores_allies() {
        let shooter = mk_unit(0, "grunt", Team::Player, 0.0);
        let ally = mk_unit(1, "grunt", Team::Player, 10.0);
        let units = vec![ally];
        let pt = mk_tower(Team::Player, -100.0);
        let et = mk_tower(Team::Enemy, 1000.0);
        let cat = mk_catapult(-50.0);
        assert!(acquire_target(&shooter, &units, &pt, &et, &cat, None).is_none());
    }

    #[test]
    fn acquire_target_ignores_dead_units() {
        let shooter = mk_unit(0, "grunt", Team::Player, 0.0);
        let mut dead = mk_unit(1, "grunt", Team::Enemy, 10.0);
        dead.hp = 0.0;
        let units = vec![dead];
        let pt = mk_tower(Team::Player, -100.0);
        let et = mk_tower(Team::Enemy, 1000.0);
        let cat = mk_catapult(-50.0);
        assert!(acquire_target(&shooter, &units, &pt, &et, &cat, None).is_none());
    }

    #[test]
    fn acquire_target_falls_back_to_tower() {
        let shooter = mk_unit(0, "grunt", Team::Player, 100.0);
        let pt = mk_tower(Team::Player, 0.0);
        let et = mk_tower(Team::Enemy, 120.0);
        let cat = mk_catapult(-50.0);
        assert_eq!(
            acquire_target(&shooter, &[], &pt, &et, &cat, None),
            Some(Target::Tower(Team::Enemy))
        );
    }

    #[test]
    fn acquire_target_returns_none_when_nothing_in_range() {
        let shooter = mk_unit(0, "grunt", Team::Player, 100.0);
        let pt = mk_tower(Team::Player, 0.0);
        let et = mk_tower(Team::Enemy, 1000.0);
        let cat = mk_catapult(-50.0);
        assert!(acquire_target(&shooter, &[], &pt, &et, &cat, None).is_none());
    }

    #[test]
    fn player_units_target_enemy_catapult_when_closer() {
        let shooter = mk_unit(0, "archer", Team::Player, 200.0);
        let pt = mk_tower(Team::Player, 0.0);
        let et = mk_tower(Team::Enemy, 5000.0);
        let pc = mk_catapult(-100.0);
        let mut ec = Catapult::with_facing(Vec2::new(300.0, 100.0), -1.0);
        ec.take_damage(0.0);
        let t = acquire_target(&shooter, &[], &pt, &et, &pc, Some(&ec));
        assert_eq!(t, Some(Target::Catapult(Team::Enemy)));
    }

    #[test]
    fn acquire_target_ignores_absent_enemy_catapult() {
        let shooter = mk_unit(0, "archer", Team::Player, 200.0);
        let pt = mk_tower(Team::Player, 0.0);
        let et = mk_tower(Team::Enemy, 5000.0);
        let pc = mk_catapult(-100.0);
        let t = acquire_target(&shooter, &[], &pt, &et, &pc, None);
        assert_ne!(t, Some(Target::Catapult(Team::Enemy)));
    }

    // ---------- resolve_attacks : timers / mouvement ----------

    #[test]
    fn resolve_attacks_decrements_cooldowns() {
        let mut u = mk_unit(0, "grunt", Team::Player, 0.0);
        u.attack_cd = Cooldown::running(0.5);
        u.heal_cd = Cooldown::running(0.3);
        let mut units = vec![u];
        let mut pt = mk_tower(Team::Player, -100.0);
        let mut et = mk_tower(Team::Enemy, 1000.0);
        let mut pc = mk_catapult(-50.0);
        let mut ec = no_enemy_catapult();
        let mut projs = vec![];
        let mut juice = Juice::new();
        resolve_attacks(
            &mut units,
            (&mut pt, &mut et),
            (&mut pc, &mut ec),
            &mut projs,
            &mut juice,
            0.1,
            1.0,
        );
        assert!((units[0].attack_cd.remaining() - 0.4).abs() < 1e-6);
        assert!((units[0].heal_cd.remaining() - 0.2).abs() < 1e-6);
    }

    #[test]
    fn resolve_attacks_moves_unit_with_no_target() {
        let u = mk_unit(0, "grunt", Team::Player, 0.0);
        let mut units = vec![u];
        let mut pt = mk_tower(Team::Player, -100.0);
        let mut et = mk_tower(Team::Enemy, 10_000.0);
        let mut pc = mk_catapult(-50.0);
        let mut ec = no_enemy_catapult();
        let mut projs = vec![];
        let mut juice = Juice::new();
        resolve_attacks(
            &mut units,
            (&mut pt, &mut et),
            (&mut pc, &mut ec),
            &mut projs,
            &mut juice,
            0.1,
            1.0,
        );
        assert!((units[0].pos.x - 9.0).abs() < 1e-6);
    }

    #[test]
    fn resolve_attacks_speed_multiplies_player_only() {
        let p = mk_unit(0, "grunt", Team::Player, 0.0);
        let e = mk_unit(1, "grunt", Team::Enemy, 5000.0);
        let mut units = vec![p, e];
        let mut pt = mk_tower(Team::Player, -100.0);
        let mut et = mk_tower(Team::Enemy, 10_000.0);
        let mut pc = mk_catapult(-50.0);
        let mut ec = no_enemy_catapult();
        let mut projs = vec![];
        let mut juice = Juice::new();
        resolve_attacks(
            &mut units,
            (&mut pt, &mut et),
            (&mut pc, &mut ec),
            &mut projs,
            &mut juice,
            0.1,
            2.0,
        );
        assert!((units[0].pos.x - 18.0).abs() < 1e-6);
        assert!((units[1].pos.x - 4991.0).abs() < 1e-6);
    }

    #[test]
    fn resolve_attacks_enemy_moves_left() {
        let u = mk_unit(0, "grunt", Team::Enemy, 500.0);
        let mut units = vec![u];
        let mut pt = mk_tower(Team::Player, 0.0);
        let mut et = mk_tower(Team::Enemy, 10_000.0);
        let mut pc = mk_catapult(-50.0);
        let mut ec = no_enemy_catapult();
        let mut projs = vec![];
        let mut juice = Juice::new();
        resolve_attacks(
            &mut units,
            (&mut pt, &mut et),
            (&mut pc, &mut ec),
            &mut projs,
            &mut juice,
            0.1,
            1.0,
        );
        assert!((units[0].pos.x - 491.0).abs() < 1e-6);
    }

    #[test]
    fn resolve_attacks_stops_unit_when_target_in_range() {
        let p = mk_unit(0, "grunt", Team::Player, 0.0);
        let e = mk_unit(1, "grunt", Team::Enemy, 10.0);
        let mut units = vec![p, e];
        let mut pt = mk_tower(Team::Player, -100.0);
        let mut et = mk_tower(Team::Enemy, 1000.0);
        let mut pc = mk_catapult(-50.0);
        let mut ec = no_enemy_catapult();
        let mut projs = vec![];
        let mut juice = Juice::new();
        resolve_attacks(
            &mut units,
            (&mut pt, &mut et),
            (&mut pc, &mut ec),
            &mut projs,
            &mut juice,
            0.1,
            1.0,
        );
        assert_eq!(units[0].pos.x, 0.0);
        assert_eq!(units[1].pos.x, 10.0);
    }

    // ---------- resolve_attacks : combat ----------

    #[test]
    fn resolve_attacks_melee_deals_damage() {
        let p = mk_unit(0, "grunt", Team::Player, 0.0);
        let e = mk_unit(1, "grunt", Team::Enemy, 10.0);
        let mut units = vec![p, e];
        let mut pt = mk_tower(Team::Player, -100.0);
        let mut et = mk_tower(Team::Enemy, 1000.0);
        let mut pc = mk_catapult(-50.0);
        let mut ec = no_enemy_catapult();
        let mut projs = vec![];
        let mut juice = Juice::new();
        run_attacks_for(
            &mut units,
            &mut pt,
            &mut et,
            (&mut pc, &mut ec),
            &mut projs,
            &mut juice,
            0.3,
        );
        assert!((units[0].hp - 25.0).abs() < 1e-6);
        assert!((units[1].hp - 25.0).abs() < 1e-6);
    }

    #[test]
    fn melee_hit_spawns_damage_text() {
        let p = mk_unit(0, "grunt", Team::Player, 0.0);
        let e = mk_unit(1, "grunt", Team::Enemy, 10.0);
        let mut units = vec![p, e];
        let mut pt = mk_tower(Team::Player, -100.0);
        let mut et = mk_tower(Team::Enemy, 1000.0);
        let mut pc = mk_catapult(-50.0);
        let mut ec = no_enemy_catapult();
        let mut projs = vec![];
        let mut juice = Juice::new();
        run_attacks_for(
            &mut units,
            &mut pt,
            &mut et,
            (&mut pc, &mut ec),
            &mut projs,
            &mut juice,
            0.3,
        );
        assert_eq!(juice.floating_texts.len(), 2);
    }

    #[test]
    fn resolve_attacks_respects_attack_cooldown() {
        let mut p = mk_unit(0, "grunt", Team::Player, 0.0);
        p.attack_cd = Cooldown::running(0.5);
        let e = mk_unit(1, "grunt", Team::Enemy, 10.0);
        let mut units = vec![p, e];
        let mut pt = mk_tower(Team::Player, -100.0);
        let mut et = mk_tower(Team::Enemy, 1000.0);
        let mut pc = mk_catapult(-50.0);
        let mut ec = no_enemy_catapult();
        let mut projs = vec![];
        let mut juice = Juice::new();
        run_attacks_for(
            &mut units,
            &mut pt,
            &mut et,
            (&mut pc, &mut ec),
            &mut projs,
            &mut juice,
            0.3,
        );
        assert!((units[1].hp - 30.0).abs() < 1e-6);
    }

    #[test]
    fn resolve_attacks_ranged_spawns_projectile() {
        let a = mk_unit(0, "archer", Team::Player, 0.0);
        let e = mk_unit(1, "grunt", Team::Enemy, 100.0);
        let mut units = vec![a, e];
        let mut pt = mk_tower(Team::Player, -100.0);
        let mut et = mk_tower(Team::Enemy, 1000.0);
        let mut pc = mk_catapult(-50.0);
        let mut ec = no_enemy_catapult();
        let mut projs = vec![];
        let mut juice = Juice::new();
        run_attacks_for(
            &mut units,
            &mut pt,
            &mut et,
            (&mut pc, &mut ec),
            &mut projs,
            &mut juice,
            0.3,
        );
        assert_eq!(projs.len(), 1);
        assert_eq!(projs[0].kind, ProjectileKind::Archer);
        assert_eq!(projs[0].gravity, 0.0);
    }

    #[test]
    fn resolve_attacks_unit_attacks_tower_melee() {
        let u = mk_unit(0, "grunt", Team::Player, 100.0);
        let mut units = vec![u];
        let mut pt = mk_tower(Team::Player, 0.0);
        let mut et = mk_tower(Team::Enemy, 120.0);
        let mut pc = mk_catapult(-50.0);
        let mut ec = no_enemy_catapult();
        let mut projs = vec![];
        let mut juice = Juice::new();
        run_attacks_for(
            &mut units,
            &mut pt,
            &mut et,
            (&mut pc, &mut ec),
            &mut projs,
            &mut juice,
            0.3,
        );
        assert!((et.hp - 495.0).abs() < 1e-6);
    }

    #[test]
    fn attack_cd_is_at_least_attack_animation_duration() {
        let p = mk_unit(0, "grunt", Team::Player, 0.0);
        let e = mk_unit(1, "grunt", Team::Enemy, 10.0);
        let mut units = vec![p, e];
        let mut pt = mk_tower(Team::Player, -100.0);
        let mut et = mk_tower(Team::Enemy, 1000.0);
        let mut pc = mk_catapult(-50.0);
        let mut ec = no_enemy_catapult();
        let mut projs = vec![];
        let mut juice = Juice::new();
        resolve_attacks(
            &mut units,
            (&mut pt, &mut et),
            (&mut pc, &mut ec),
            &mut projs,
            &mut juice,
            0.05,
            1.0,
        );
        assert!(units[0].attack_cd.duration() >= crate::stickman::ATTACK_DURATION - 1e-6);
    }

    // ---------- Healer ----------

    #[test]
    fn resolve_attacks_healer_heals_wounded_ally() {
        let h = mk_unit(0, "healer", Team::Player, 0.0);
        let mut wounded = mk_unit(1, "grunt", Team::Player, 50.0);
        wounded.hp = 10.0;
        let mut units = vec![h, wounded];
        let mut pt = mk_tower(Team::Player, -100.0);
        let mut et = mk_tower(Team::Enemy, 1000.0);
        let mut pc = mk_catapult(-50.0);
        let mut ec = no_enemy_catapult();
        let mut projs = vec![];
        let mut juice = Juice::new();
        resolve_attacks(
            &mut units,
            (&mut pt, &mut et),
            (&mut pc, &mut ec),
            &mut projs,
            &mut juice,
            0.1,
            1.0,
        );
        assert!((units[1].hp - 18.0).abs() < 1e-6);
    }

    #[test]
    fn healer_spawns_heal_text() {
        let h = mk_unit(0, "healer", Team::Player, 0.0);
        let mut wounded = mk_unit(1, "grunt", Team::Player, 50.0);
        wounded.hp = 10.0;
        let mut units = vec![h, wounded];
        let mut pt = mk_tower(Team::Player, -100.0);
        let mut et = mk_tower(Team::Enemy, 1000.0);
        let mut pc = mk_catapult(-50.0);
        let mut ec = no_enemy_catapult();
        let mut projs = vec![];
        let mut juice = Juice::new();
        resolve_attacks(
            &mut units,
            (&mut pt, &mut et),
            (&mut pc, &mut ec),
            &mut projs,
            &mut juice,
            0.1,
            1.0,
        );
        assert_eq!(juice.floating_texts.len(), 1);
        assert!(juice.floating_texts[0].text.starts_with('+'));
    }

    #[test]
    fn healer_respects_cooldown() {
        let mut h = mk_unit(0, "healer", Team::Player, 0.0);
        h.heal_cd = Cooldown::running(2.0);
        let mut wounded = mk_unit(1, "grunt", Team::Player, 50.0);
        wounded.hp = 10.0;
        let mut units = vec![h, wounded];
        let mut pt = mk_tower(Team::Player, -100.0);
        let mut et = mk_tower(Team::Enemy, 1000.0);
        let mut pc = mk_catapult(-50.0);
        let mut ec = no_enemy_catapult();
        let mut projs = vec![];
        let mut juice = Juice::new();
        resolve_attacks(
            &mut units,
            (&mut pt, &mut et),
            (&mut pc, &mut ec),
            &mut projs,
            &mut juice,
            0.1,
            1.0,
        );
        assert!((units[1].hp - 10.0).abs() < 1e-6);
    }

    // ---------- Bomber ----------

    #[test]
    fn bomber_suicide_damages_all_enemies_in_radius() {
        let p = mk_unit(0, "bomber", Team::Player, 0.0);
        let e1 = mk_unit(1, "grunt", Team::Enemy, 20.0);
        let e2 = mk_unit(2, "grunt", Team::Enemy, 50.0);
        let e3_far = mk_unit(3, "grunt", Team::Enemy, 200.0);
        let mut units = vec![p, e1, e2, e3_far];
        let mut pt = mk_tower(Team::Player, -100.0);
        let mut et = mk_tower(Team::Enemy, 1000.0);
        let mut pc = mk_catapult(-50.0);
        let mut ec = no_enemy_catapult();
        let mut projs = vec![];
        let mut juice = Juice::new();
        run_attacks_for(
            &mut units,
            &mut pt,
            &mut et,
            (&mut pc, &mut ec),
            &mut projs,
            &mut juice,
            0.3,
        );
        assert_eq!(units[0].hp, 0.0);
        assert!(units[1].hp <= 0.0);
        assert!(units[2].hp <= 0.0);
        assert!((units[3].hp - 30.0).abs() < 1e-3);
    }

    #[test]
    fn bomber_damages_tower_and_dies() {
        let p = mk_unit(0, "bomber", Team::Player, 100.0);
        let mut units = vec![p];
        let mut pt = mk_tower(Team::Player, 0.0);
        let mut et = mk_tower(Team::Enemy, 120.0);
        let mut pc = mk_catapult(-50.0);
        let mut ec = no_enemy_catapult();
        let mut projs = vec![];
        let mut juice = Juice::new();
        run_attacks_for(
            &mut units,
            &mut pt,
            &mut et,
            (&mut pc, &mut ec),
            &mut projs,
            &mut juice,
            0.3,
        );
        assert_eq!(units[0].hp, 0.0);
        assert!((et.hp - (500.0 - 45.0)).abs() < 1e-3);
    }

    // ---------- Enemy attacks catapult ----------

    #[test]
    fn enemy_unit_attacks_catapult_when_in_range() {
        let mut shooter = mk_unit(0, "brute", Team::Enemy, 230.0);
        shooter.attack_cd = Cooldown::default();
        let mut units = vec![shooter];
        let mut pt = mk_tower(Team::Player, -100.0);
        let mut et = mk_tower(Team::Enemy, 5000.0);
        let mut pc = mk_catapult(200.0);
        let mut ec = no_enemy_catapult();
        let mut projs = vec![];
        let mut juice = Juice::new();
        let hp_before = pc.hp;
        run_attacks_for(
            &mut units,
            &mut pt,
            &mut et,
            (&mut pc, &mut ec),
            &mut projs,
            &mut juice,
            0.3,
        );
        assert!(pc.hp < hp_before, "catapulte non endommagée");
    }

    // ---------- resolve_projectiles ----------

    #[test]
    fn resolve_projectiles_moves_by_velocity() {
        let mut projs = vec![Projectile {
            pos: Vec2::ZERO,
            vel: Vec2::new(100.0, 0.0),
            gravity: 0.0,
            damage: 10.0,
            radius: 5.0,
            team: Team::Player,
            kind: ProjectileKind::Basic,
            pierce_remaining: 0,
            color: macroquad::prelude::WHITE,
        }];
        let mut units = vec![];
        let mut pt = mk_tower(Team::Player, -100.0);
        let mut et = mk_tower(Team::Enemy, 1000.0);
        let mut pc = mk_catapult(-50.0);
        let mut ec = no_enemy_catapult();
        let mut juice = Juice::new();
        resolve_projectiles(
            &mut projs,
            &mut units,
            (&mut pt, &mut et),
            (&mut pc, &mut ec),
            500.0,
            &mut juice,
            0.1,
        );
        assert!((projs[0].pos.x - 10.0).abs() < 1e-6);
    }

    #[test]
    fn resolve_projectiles_deals_damage_to_enemy() {
        let mut projs = vec![Projectile {
            pos: Vec2::ZERO,
            vel: Vec2::new(100.0, 0.0),
            gravity: 0.0,
            damage: 10.0,
            radius: 5.0,
            team: Team::Player,
            kind: ProjectileKind::Basic,
            pierce_remaining: 0,
            color: macroquad::prelude::WHITE,
        }];
        let e = mk_unit(0, "grunt", Team::Enemy, 10.0);
        let mut units = vec![e];
        let mut pt = mk_tower(Team::Player, -100.0);
        let mut et = mk_tower(Team::Enemy, 1000.0);
        let mut pc = mk_catapult(-50.0);
        let mut ec = no_enemy_catapult();
        let mut juice = Juice::new();
        resolve_projectiles(
            &mut projs,
            &mut units,
            (&mut pt, &mut et),
            (&mut pc, &mut ec),
            500.0,
            &mut juice,
            0.1,
        );
        assert!((units[0].hp - 20.0).abs() < 1e-6);
        assert!(projs.is_empty());
    }

    #[test]
    fn resolve_projectiles_player_hit_spawns_floating_text() {
        let mut projs = vec![Projectile {
            pos: Vec2::ZERO,
            vel: Vec2::new(100.0, 0.0),
            gravity: 0.0,
            damage: 10.0,
            radius: 5.0,
            team: Team::Player,
            kind: ProjectileKind::Basic,
            pierce_remaining: 0,
            color: macroquad::prelude::WHITE,
        }];
        let e = mk_unit(0, "grunt", Team::Enemy, 10.0);
        let mut units = vec![e];
        let mut pt = mk_tower(Team::Player, -100.0);
        let mut et = mk_tower(Team::Enemy, 1000.0);
        let mut pc = mk_catapult(-50.0);
        let mut ec = no_enemy_catapult();
        let mut juice = Juice::new();
        resolve_projectiles(
            &mut projs,
            &mut units,
            (&mut pt, &mut et),
            (&mut pc, &mut ec),
            500.0,
            &mut juice,
            0.1,
        );
        assert_eq!(juice.floating_texts.len(), 1);
    }

    #[test]
    fn resolve_projectiles_enemy_hit_spawns_floating_text() {
        let mut projs = vec![Projectile {
            pos: Vec2::ZERO,
            vel: Vec2::new(100.0, 0.0),
            gravity: 0.0,
            damage: 10.0,
            radius: 5.0,
            team: Team::Enemy,
            kind: ProjectileKind::Basic,
            pierce_remaining: 0,
            color: macroquad::prelude::WHITE,
        }];
        let e = mk_unit(0, "grunt", Team::Player, 10.0);
        let mut units = vec![e];
        let mut pt = mk_tower(Team::Player, -100.0);
        let mut et = mk_tower(Team::Enemy, 1000.0);
        let mut pc = mk_catapult(-50.0);
        let mut ec = no_enemy_catapult();
        let mut juice = Juice::new();
        resolve_projectiles(
            &mut projs,
            &mut units,
            (&mut pt, &mut et),
            (&mut pc, &mut ec),
            500.0,
            &mut juice,
            0.1,
        );
        assert_eq!(juice.floating_texts.len(), 1);
    }

    #[test]
    fn resolve_projectiles_ignores_allied_units() {
        let mut projs = vec![Projectile {
            pos: Vec2::ZERO,
            vel: Vec2::new(100.0, 0.0),
            gravity: 0.0,
            damage: 10.0,
            radius: 5.0,
            team: Team::Player,
            kind: ProjectileKind::Basic,
            pierce_remaining: 0,
            color: macroquad::prelude::WHITE,
        }];
        let ally = mk_unit(0, "grunt", Team::Player, 10.0);
        let mut units = vec![ally];
        let mut pt = mk_tower(Team::Player, -100.0);
        let mut et = mk_tower(Team::Enemy, 1000.0);
        let mut pc = mk_catapult(-50.0);
        let mut ec = no_enemy_catapult();
        let mut juice = Juice::new();
        resolve_projectiles(
            &mut projs,
            &mut units,
            (&mut pt, &mut et),
            (&mut pc, &mut ec),
            500.0,
            &mut juice,
            0.1,
        );
        assert!((units[0].hp - 30.0).abs() < 1e-6);
        assert_eq!(projs.len(), 1);
        assert!(juice.floating_texts.is_empty());
    }

    #[test]
    fn resolve_projectiles_piercing_keeps_going() {
        let mut projs = vec![Projectile {
            pos: Vec2::ZERO,
            vel: Vec2::new(100.0, 0.0),
            gravity: 0.0,
            damage: 5.0,
            radius: 5.0,
            team: Team::Player,
            kind: ProjectileKind::Basic,
            pierce_remaining: 3,
            color: macroquad::prelude::WHITE,
        }];
        let e1 = mk_unit(0, "grunt", Team::Enemy, 10.0);
        let e2 = mk_unit(1, "grunt", Team::Enemy, 60.0);
        let e3 = mk_unit(2, "grunt", Team::Enemy, 110.0);
        let mut units = vec![e1, e2, e3];
        let mut pt = mk_tower(Team::Player, -100.0);
        let mut et = mk_tower(Team::Enemy, 1000.0);
        let mut pc = mk_catapult(-50.0);
        let mut ec = no_enemy_catapult();
        let mut juice = Juice::new();

        resolve_projectiles(
            &mut projs,
            &mut units,
            (&mut pt, &mut et),
            (&mut pc, &mut ec),
            500.0,
            &mut juice,
            0.1,
        );
        assert_eq!(projs.len(), 1);
        assert_eq!(projs[0].pierce_remaining, 2);

        resolve_projectiles(
            &mut projs,
            &mut units,
            (&mut pt, &mut et),
            (&mut pc, &mut ec),
            500.0,
            &mut juice,
            0.5,
        );
        assert_eq!(projs.len(), 1);
        assert_eq!(projs[0].pierce_remaining, 1);

        resolve_projectiles(
            &mut projs,
            &mut units,
            (&mut pt, &mut et),
            (&mut pc, &mut ec),
            500.0,
            &mut juice,
            0.5,
        );
        assert!(projs.is_empty());
    }

    #[test]
    fn resolve_projectiles_hits_tower() {
        let mut projs = vec![Projectile {
            pos: Vec2::new(990.0, 0.0),
            vel: Vec2::new(100.0, 0.0),
            gravity: 0.0,
            damage: 20.0,
            radius: 5.0,
            team: Team::Player,
            kind: ProjectileKind::Basic,
            pierce_remaining: 0,
            color: macroquad::prelude::WHITE,
        }];
        let mut units = vec![];
        let mut pt = mk_tower(Team::Player, -1000.0);
        let mut et = mk_tower(Team::Enemy, 1000.0);
        let mut pc = mk_catapult(-5000.0);
        let mut ec = no_enemy_catapult();
        let mut juice = Juice::new();
        resolve_projectiles(
            &mut projs,
            &mut units,
            (&mut pt, &mut et),
            (&mut pc, &mut ec),
            500.0,
            &mut juice,
            0.1,
        );
        assert!((et.hp - 480.0).abs() < 1e-6);
        assert!(projs.is_empty());
    }

    #[test]
    fn enemy_projectile_hits_player_catapult() {
        let mut pc = mk_catapult(200.0);
        let (rx, ry, _rw, _rh) = pc.rect();
        let mut projs = vec![Projectile {
            pos: Vec2::new(rx - 10.0, ry + 20.0),
            vel: Vec2::new(200.0, 0.0),
            gravity: 0.0,
            damage: 25.0,
            radius: 5.0,
            team: Team::Enemy,
            kind: ProjectileKind::Archer,
            pierce_remaining: 0,
            color: macroquad::prelude::WHITE,
        }];
        let mut units = vec![];
        let mut pt = mk_tower(Team::Player, -1000.0);
        let mut et = mk_tower(Team::Enemy, 5000.0);
        let mut ec = no_enemy_catapult();
        let mut juice = Juice::new();
        let hp_before = pc.hp;
        for _ in 0..3 {
            resolve_projectiles(
                &mut projs,
                &mut units,
                (&mut pt, &mut et),
                (&mut pc, &mut ec),
                500.0,
                &mut juice,
                0.05,
            );
        }
        assert!(pc.hp < hp_before);
    }

    #[test]
    fn player_projectile_hits_enemy_catapult() {
        let ec_cat = Catapult::with_facing(Vec2::new(800.0, 100.0), -1.0);
        let (rx, ry, _rw, _rh) = ec_cat.rect();
        let mut projs = vec![Projectile {
            pos: Vec2::new(rx - 10.0, ry + 20.0),
            vel: Vec2::new(200.0, 0.0),
            gravity: 0.0,
            damage: 25.0,
            radius: 5.0,
            team: Team::Player,
            kind: ProjectileKind::Archer,
            pierce_remaining: 0,
            color: macroquad::prelude::WHITE,
        }];
        let mut units = vec![];
        let mut pt = mk_tower(Team::Player, -1000.0);
        let mut et = mk_tower(Team::Enemy, 5000.0);
        let mut pc = mk_catapult(-100.0);
        let mut ec = Some(ec_cat);
        let mut juice = Juice::new();
        let hp_before = ec.as_ref().unwrap().hp;
        for _ in 0..3 {
            resolve_projectiles(
                &mut projs,
                &mut units,
                (&mut pt, &mut et),
                (&mut pc, &mut ec),
                500.0,
                &mut juice,
                0.05,
            );
        }
        assert!(ec.as_ref().unwrap().hp < hp_before);
    }

    #[test]
    fn resolve_projectiles_explosive_deals_aoe() {
        let mut projs = vec![Projectile {
            pos: Vec2::ZERO,
            vel: Vec2::new(100.0, 0.0),
            gravity: 0.0,
            damage: 20.0,
            radius: 5.0,
            team: Team::Player,
            kind: ProjectileKind::Explosive,
            pierce_remaining: 0,
            color: macroquad::prelude::WHITE,
        }];
        let e1 = mk_unit(0, "grunt", Team::Enemy, 10.0);
        let e2 = mk_unit(1, "grunt", Team::Enemy, 25.0);
        let mut units = vec![e1, e2];
        let mut pt = mk_tower(Team::Player, -100.0);
        let mut et = mk_tower(Team::Enemy, 1000.0);
        let mut pc = mk_catapult(-50.0);
        let mut ec = no_enemy_catapult();
        let mut juice = Juice::new();
        resolve_projectiles(
            &mut projs,
            &mut units,
            (&mut pt, &mut et),
            (&mut pc, &mut ec),
            500.0,
            &mut juice,
            0.1,
        );
        assert!((units[0].hp - 10.0).abs() < 1e-6);
        assert!((units[1].hp - 20.0).abs() < 1e-6);
    }

    #[test]
    fn catapult_projectile_is_removed_below_ground() {
        let mut projs = vec![Projectile {
            pos: Vec2::new(300.0, 900.0),
            vel: Vec2::new(50.0, 200.0),
            gravity: 900.0,
            damage: 10.0,
            radius: 7.0,
            team: Team::Player,
            kind: ProjectileKind::Basic,
            pierce_remaining: 0,
            color: macroquad::prelude::WHITE,
        }];
        let mut units = vec![];
        let mut pt = mk_tower(Team::Player, -1000.0);
        let mut et = mk_tower(Team::Enemy, 5000.0);
        let mut pc = mk_catapult(-5000.0);
        let mut ec = no_enemy_catapult();
        let mut juice = Juice::new();
        resolve_projectiles(
            &mut projs,
            &mut units,
            (&mut pt, &mut et),
            (&mut pc, &mut ec),
            500.0,
            &mut juice,
            0.05,
        );
        assert!(projs.is_empty());
    }

    #[test]
    fn resolve_projectiles_removes_out_of_bounds() {
        let mut projs = vec![Projectile {
            pos: Vec2::new(-400.0, 0.0),
            vel: Vec2::new(-500.0, 0.0),
            gravity: 0.0,
            damage: 10.0,
            radius: 5.0,
            team: Team::Player,
            kind: ProjectileKind::Basic,
            pierce_remaining: 0,
            color: macroquad::prelude::WHITE,
        }];
        let mut units = vec![];
        let mut pt = mk_tower(Team::Player, -1000.0);
        let mut et = mk_tower(Team::Enemy, 1000.0);
        let mut pc = mk_catapult(-5000.0);
        let mut ec = no_enemy_catapult();
        let mut juice = Juice::new();
        resolve_projectiles(
            &mut projs,
            &mut units,
            (&mut pt, &mut et),
            (&mut pc, &mut ec),
            500.0,
            &mut juice,
            0.5,
        );
        assert!(projs.is_empty());
    }
}