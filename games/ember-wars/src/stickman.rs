//! Stick men procéduraux — corps en 6 points, animation par sin/cos.

use glam::Vec2;
use macroquad::prelude::*;

use crate::components::Unit;
use crate::weapons::{self, Weapon, WeaponAnchor};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pose {
    Idle,
    Walking,
    Attacking,
    Dying,
}

pub const WALK_FREQ: f32 = 2.2;
pub const IDLE_FREQ: f32 = 1.2;
pub const ATTACK_DURATION: f32 = 0.55;
pub const DYING_DURATION: f32 = 0.6;

pub const WINDUP_END: f32 = 0.30;
pub const STRIKE_END: f32 = 0.65;

/// Angle du bras droit (celui qui porte l'arme) au repos. Doit être
/// identique à `REST_ARM_R` pour qu'il n'y ait pas de saut au début de
/// l'animation.
const REST_ARM_R: f32 = -0.15;
const REST_ARM_L: f32 = 0.15;

struct Proportions {
    head_r: f32,
    torso_len: f32,
    arm_len: f32,
    leg_len: f32,
    line_w: f32,
}

fn proportions(size: Vec2) -> Proportions {
    let h = size.y.max(10.0);
    Proportions {
        head_r: (h * 0.13).max(2.5),
        torso_len: h * 0.40,
        arm_len: h * 0.28,
        leg_len: h * 0.42,
        line_w: (h * 0.09).clamp(1.5, 3.5),
    }
}

/// Résultat d'une phase d'attaque : progression dans la phase [0, 1]
/// et phase active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AttackPhase {
    Windup,
    Strike,
    Recovery,
}

/// Retourne (phase, progression). Toujours défini — jamais de "aucune
/// phase" — pour éviter les discontinuités à t = 0.
fn attack_phase(t: f32) -> (AttackPhase, f32) {
    let t = t.clamp(0.0, 1.0);
    if t < WINDUP_END {
        (AttackPhase::Windup, if WINDUP_END > 0.0 { t / WINDUP_END } else { 0.0 })
    } else if t < STRIKE_END {
        let span = STRIKE_END - WINDUP_END;
        (AttackPhase::Strike, if span > 0.0 { (t - WINDUP_END) / span } else { 1.0 })
    } else {
        let span = 1.0 - STRIKE_END;
        (AttackPhase::Recovery, if span > 0.0 { (t - STRIKE_END) / span } else { 1.0 })
    }
}

/// Angles des membres (bras gauche, bras droit, jambe gauche, jambe droite)
/// pour une phase et une progression données.
fn attack_member_angles(phase: AttackPhase, progress: f32) -> (f32, f32, f32, f32) {
    let p = progress.clamp(0.0, 1.0);
    match phase {
        AttackPhase::Windup => {
            // Bras droit : part du repos, monte derrière.
            let arm_r = REST_ARM_R + p * (-1.65);
            // Bras gauche : contre-balance en avant.
            let arm_l = REST_ARM_L + p * 0.50;
            (arm_l, arm_r, 0.30, -0.35)
        }
        AttackPhase::Strike => {
            // Interpolation smoothstep pour un mouvement naturel.
            let s = p * p * (3.0 - 2.0 * p);
            // Bras droit : de l'arrière du windup (-1.80) part devant (+1.60).
            let arm_r = -1.80 + s * 3.40;
            // Bras gauche : accompagne en arrière.
            let arm_l = 0.65 - s * 0.50;
            (arm_l, arm_r, 0.30, -0.35 - s * 0.15)
        }
        AttackPhase::Recovery => {
            let s = p * p * (3.0 - 2.0 * p);
            // Bras droit : revient de +1.60 vers REST_ARM_R.
            let arm_r = 1.60 + s * (REST_ARM_R - 1.60);
            // Bras gauche : revient vers REST_ARM_L.
            let arm_l = 0.15 + s * (REST_ARM_L - 0.15);
            (arm_l, arm_r, 0.30, -0.35)
        }
    }
}

fn member_angles(pose: Pose, phase: f32) -> (f32, f32, f32, f32) {
    match pose {
        Pose::Idle => {
            let sway = (phase * IDLE_FREQ * std::f32::consts::TAU).sin() * 0.08;
            (REST_ARM_L + sway, REST_ARM_R - sway, 0.0, 0.0)
        }
        Pose::Walking => {
            let swing = (phase * WALK_FREQ * std::f32::consts::TAU).sin() * 0.55;
            (swing, -swing, -swing, swing)
        }
        Pose::Attacking => {
            let t = (phase / ATTACK_DURATION).clamp(0.0, 1.0);
            let (phase_kind, progress) = attack_phase(t);
            attack_member_angles(phase_kind, progress)
        }
        Pose::Dying => {
            let t = (phase / DYING_DURATION).clamp(0.0, 1.0);
            let spread = t * 1.2;
            (spread, -spread, spread * 0.8, -spread * 0.8)
        }
    }
}

/// Inclinaison du torse (décalage horizontal du cou).
fn torso_lean(pose: Pose, phase: f32) -> f32 {
    if pose != Pose::Attacking {
        return 0.0;
    }
    let t = (phase / ATTACK_DURATION).clamp(0.0, 1.0);
    let (phase_kind, progress) = attack_phase(t);
    let p = progress.clamp(0.0, 1.0);
    match phase_kind {
        AttackPhase::Windup => -0.12 * p,
        AttackPhase::Strike => {
            let s = p * p * (3.0 - 2.0 * p);
            0.30 * s
        }
        AttackPhase::Recovery => {
            let s = p * p * (3.0 - 2.0 * p);
            0.30 * (1.0 - s)
        }
    }
}

/// Taux d'attaque (0..1) pour l'arme. Utilisé par `weapons.rs`.
fn attack_t(pose: Pose, phase: f32) -> f32 {
    if pose != Pose::Attacking {
        return 0.0;
    }
    let t = (phase / ATTACK_DURATION).clamp(0.0, 1.0);
    let (phase_kind, progress) = attack_phase(t);
    let p = progress.clamp(0.0, 1.0);
    match phase_kind {
        AttackPhase::Windup => -0.6 * p,
        AttackPhase::Strike => {
            let s = p * p * (3.0 - 2.0 * p);
            -0.6 + s * 1.6
        }
        AttackPhase::Recovery => {
            let s = p * p * (3.0 - 2.0 * p);
            1.0 * (1.0 - s)
        }
    }
}

pub fn draw(
    unit: &Unit,
    pos: Vec2,
    size: Vec2,
    facing: f32,
    pose: Pose,
    phase: f32,
    color: Color,
) {
    let p = proportions(size);
    let line_w = p.line_w;

    let pelvis = pos - Vec2::new(0.0, p.leg_len);
    let neck = pelvis - Vec2::new(0.0, p.torso_len);
    let head_center = neck - Vec2::new(0.0, p.head_r + 2.0);

    let lean = torso_lean(pose, phase);
    let lean_vec = Vec2::new(facing * lean * p.torso_len, 0.0);
    let neck = neck + lean_vec;
    let head_center = head_center + lean_vec;

    let (arm_l, arm_r, leg_l, leg_r) = member_angles(pose, phase);

    let f = facing;

    // Jambes.
    let leg_base = pelvis;
    let leg_l_vec = Vec2::new(f * leg_l.sin() * p.leg_len, leg_l.cos() * p.leg_len);
    let leg_r_vec = Vec2::new(f * leg_r.sin() * p.leg_len, leg_r.cos() * p.leg_len);
    let leg_l_end = leg_base + leg_l_vec;
    let leg_r_end = leg_base + leg_r_vec;
    draw_line(leg_base.x, leg_base.y, leg_l_end.x, leg_l_end.y, line_w, color);
    draw_line(leg_base.x, leg_base.y, leg_r_end.x, leg_r_end.y, line_w, color);

    // Torse.
    draw_line(pelvis.x, pelvis.y, neck.x, neck.y, line_w, color);

    // Bras.
    let arm_l_vec = Vec2::new(f * arm_l.sin() * p.arm_len, arm_l.cos() * p.arm_len);
    let arm_r_vec = Vec2::new(f * arm_r.sin() * p.arm_len, arm_r.cos() * p.arm_len);
    let arm_l_end = neck + arm_l_vec;
    let arm_r_end = neck + arm_r_vec;
    draw_line(neck.x, neck.y, arm_l_end.x, arm_l_end.y, line_w * 0.85, color);
    draw_line(neck.x, neck.y, arm_r_end.x, arm_r_end.y, line_width_safe(line_w), color);

    // Tête.
    draw_circle(head_center.x, head_center.y, p.head_r, color);

    // Arme ancrée à la main droite.
    let weapon = Weapon::for_kind(&unit.kind);
    let attack = attack_t(pose, phase);
    let anchor = WeaponAnchor {
        hand: arm_r_end,
        facing,
        arm_angle: arm_r,
        attack_t: attack,
    };
    weapons::draw_weapon(weapon, anchor, color);
}

/// Petit helper pour éviter un warning si on change la largeur plus tard.
fn line_width_safe(w: f32) -> f32 {
    w * 0.85
}

pub fn rotate_down(angle: f32) -> Vec2 {
    Vec2::new(angle.sin(), angle.cos())
}

pub fn auto_pose(unit: &Unit, is_attacking: bool) -> Pose {
    if !unit.is_alive() {
        Pose::Dying
    } else if is_attacking {
        Pose::Attacking
    } else {
        Pose::Walking
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{Team, UnitId};
    use ember_stdlib::time::Cooldown;

    fn mk_unit(id: u32, hp: f32) -> Unit {
        Unit {
            id: UnitId(id),
            kind: "grunt".to_owned(),
            team: Team::Player,
            pos: Vec2::ZERO,
            hp,
            max_hp: 30.0,
            damage: 5.0,
            attack_cd: Cooldown::new(0.8),
            heal_cd: Cooldown::default(),
            hit_flash: Cooldown::default(),
            pose_phase: 0.0,
            target: None,
            pending_attack: None,
        }
    }

    #[test]
    fn proportions_scale_with_size() {
        let p_small = proportions(Vec2::new(10.0, 20.0));
        let p_big = proportions(Vec2::new(30.0, 60.0));
        assert!(p_big.torso_len > p_small.torso_len);
        assert!(p_big.head_r > p_small.head_r);
        assert!(p_big.leg_len > p_small.leg_len);
    }

    #[test]
    fn proportions_clamp_line_width() {
        let p_tiny = proportions(Vec2::new(5.0, 8.0));
        assert!(p_tiny.line_w >= 1.5);
        let p_huge = proportions(Vec2::new(100.0, 200.0));
        assert!(p_huge.line_w <= 3.5);
    }

    #[test]
    fn idle_arms_are_near_vertical() {
        let (al, ar, _, _) = member_angles(Pose::Idle, 0.0);
        assert!(al.abs() < 0.3);
        assert!(ar.abs() < 0.3);
    }

    #[test]
    fn walking_legs_are_opposite() {
        let (_, _, ll, lr) = member_angles(Pose::Walking, 0.1);
        assert!((ll + lr).abs() < 1e-4);
        assert!(ll.abs() > 0.05);
    }

    /// Le tout premier frame de l'attaque doit être identique au repos,
    /// pour éviter un "saut" visuel avant le windup.
    #[test]
    fn attack_starts_at_rest_position() {
        let (_, ar, _, _) = member_angles(Pose::Attacking, 0.0);
        assert!(
            (ar - REST_ARM_R).abs() < 1e-4,
            "attack should start at REST_ARM_R ({}), got {}",
            REST_ARM_R,
            ar
        );
    }

    /// Le dernier frame de l'attaque doit aussi être au repos, pour
    /// éviter un saut au retour en walking.
    #[test]
    fn attack_ends_at_rest_position() {
        let (_, ar, _, _) = member_angles(Pose::Attacking, ATTACK_DURATION);
        assert!(
            (ar - REST_ARM_R).abs() < 0.1,
            "attack should end near REST_ARM_R ({}), got {}",
            REST_ARM_R,
            ar
        );
    }

    #[test]
    fn attack_windup_pulls_arm_back() {
        let (_, ar, _, _) = member_angles(Pose::Attacking, ATTACK_DURATION * 0.15);
        assert!(ar < -0.5, "arm should be pulled back: {}", ar);
    }

    #[test]
    fn attack_windup_fully_back_at_end() {
        let (_, ar, _, _) = member_angles(Pose::Attacking, ATTACK_DURATION * (WINDUP_END - 0.001));
        assert!(ar < -1.5, "arm should be fully behind: {}", ar);
    }

    #[test]
    fn attack_strike_pushes_arm_forward() {
        let (_, ar, _, _) = member_angles(Pose::Attacking, ATTACK_DURATION * 0.5);
        assert!(ar > -0.5, "arm should be moving forward: {}", ar);
    }

    #[test]
    fn attack_strike_ends_fully_forward() {
        let (_, ar, _, _) = member_angles(Pose::Attacking, ATTACK_DURATION * (STRIKE_END - 0.001));
        assert!(ar > 1.4, "arm should be fully forward: {}", ar);
    }

    #[test]
    fn attack_arm_range_is_large() {
        let (_, ar_back, _, _) =
            member_angles(Pose::Attacking, ATTACK_DURATION * (WINDUP_END - 0.001));
        let (_, ar_forward, _, _) =
            member_angles(Pose::Attacking, ATTACK_DURATION * (STRIKE_END - 0.001));
        let amplitude = (ar_forward - ar_back).abs();
        assert!(amplitude > 3.0, "swing too small: {}", amplitude);
    }

    /// L'animation ne doit pas avoir de discontinuité majeure entre les
    /// phases — le saut max entre deux échantillons d'une frame doit
    /// rester raisonnable.
    #[test]
    fn attack_animation_is_continuous() {
        let dt_norm = (1.0 / 60.0) / ATTACK_DURATION;
        let mut prev = member_angles(Pose::Attacking, 0.0).1;
        let mut max_jump = 0.0_f32;
        let steps = 40;
        for i in 1..=steps {
            let t = (i as f32 / steps as f32) * ATTACK_DURATION;
            let cur = member_angles(Pose::Attacking, t).1;
            let jump = (cur - prev).abs();
            if jump > max_jump {
                max_jump = jump;
            }
            prev = cur;
            let _ = dt_norm;
        }
        // Le saut max doit être contenu (le strike est rapide mais pas
        // brutal au point de sauter de > 1.5 rad en une frame).
        assert!(
            max_jump < 1.5,
            "animation has a discontinuity: max jump = {}",
            max_jump
        );
    }

    #[test]
    fn attack_t_negative_at_windup() {
        let t = attack_t(Pose::Attacking, ATTACK_DURATION * 0.15);
        assert!(t < 0.0);
    }

    #[test]
    fn attack_t_positive_at_strike() {
        let t = attack_t(Pose::Attacking, ATTACK_DURATION * 0.5);
        assert!(t > 0.0);
    }

    #[test]
    fn attack_t_zero_when_not_attacking() {
        assert_eq!(attack_t(Pose::Idle, 0.1), 0.0);
        assert_eq!(attack_t(Pose::Walking, 0.1), 0.0);
        assert_eq!(attack_t(Pose::Dying, 0.1), 0.0);
    }

    #[test]
    fn torso_lean_forward_at_strike() {
        let lean = torso_lean(Pose::Attacking, ATTACK_DURATION * 0.5);
        assert!(lean > 0.1);
    }

    #[test]
    fn torso_lean_backward_at_windup() {
        let lean = torso_lean(Pose::Attacking, ATTACK_DURATION * 0.15);
        assert!(lean < 0.0);
    }

    #[test]
    fn torso_lean_zero_outside_attack() {
        assert_eq!(torso_lean(Pose::Idle, 0.1), 0.0);
        assert_eq!(torso_lean(Pose::Walking, 0.1), 0.0);
    }

    #[test]
    fn attack_phase_windup() {
        let (kind, p) = attack_phase(0.15);
        assert_eq!(kind, AttackPhase::Windup);
        assert!(p > 0.0);
    }

    #[test]
    fn attack_phase_strike() {
        let (kind, p) = attack_phase(0.5);
        assert_eq!(kind, AttackPhase::Strike);
        assert!(p > 0.0);
    }

    #[test]
    fn attack_phase_recovery() {
        let (kind, p) = attack_phase(0.8);
        assert_eq!(kind, AttackPhase::Recovery);
        assert!(p > 0.0);
    }

    #[test]
    fn attack_phase_at_zero_is_windup_start() {
        let (kind, p) = attack_phase(0.0);
        assert_eq!(kind, AttackPhase::Windup);
        assert_eq!(p, 0.0);
    }

    #[test]
    fn attack_phase_at_one_is_recovery_end() {
        let (kind, p) = attack_phase(1.0);
        assert_eq!(kind, AttackPhase::Recovery);
        assert!((p - 1.0).abs() < 1e-6);
    }

    #[test]
    fn dying_spreads_over_time() {
        let (al_a, ar_a, _, _) = member_angles(Pose::Dying, 0.01);
        let (al_b, ar_b, _, _) = member_angles(Pose::Dying, DYING_DURATION);
        assert!(al_b.abs() > al_a.abs());
        assert!(ar_b.abs() > ar_a.abs());
    }

    #[test]
    fn auto_pose_dying_when_dead() {
        let u = mk_unit(0, 0.0);
        assert_eq!(auto_pose(&u, false), Pose::Dying);
    }

    #[test]
    fn auto_pose_attacking_when_flag_set() {
        let u = mk_unit(0, 10.0);
        assert_eq!(auto_pose(&u, true), Pose::Attacking);
    }

    #[test]
    fn auto_pose_walking_when_no_target() {
        let u = mk_unit(0, 10.0);
        assert_eq!(auto_pose(&u, false), Pose::Walking);
    }

    #[test]
    fn auto_pose_prioritizes_dying_over_attacking() {
        let u = mk_unit(0, 0.0);
        assert_eq!(auto_pose(&u, true), Pose::Dying);
    }

    #[test]
    fn rotate_down_zero_is_straight_down() {
        let v = rotate_down(0.0);
        assert!(v.x.abs() < 1e-6);
        assert!((v.y - 1.0).abs() < 1e-6);
    }

    #[test]
    fn rotate_down_pi_half_is_right() {
        let v = rotate_down(std::f32::consts::FRAC_PI_2);
        assert!((v.x - 1.0).abs() < 1e-6);
        assert!(v.y.abs() < 1e-6);
    }
}