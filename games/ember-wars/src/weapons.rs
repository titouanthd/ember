//! Armes procédurales — dessinées depuis le bout du bras.

use glam::Vec2;
use macroquad::prelude::*;

/// Arme tenue par une unité.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Weapon {
    Sword,
    Club,
    Bow,
    Staff,
    Bomb,
    Longsword,
}

impl Weapon {
    pub fn for_kind(kind: &str) -> Weapon {
        match kind {
            "grunt" => Weapon::Sword,
            "brute" => Weapon::Club,
            "archer" => Weapon::Bow,
            "healer" => Weapon::Staff,
            "bomber" => Weapon::Bomb,
            "hero" => Weapon::Longsword,
            _ => Weapon::Sword,
        }
    }

    pub fn length(self) -> f32 {
        match self {
            Weapon::Sword => 18.0,
            Weapon::Club => 22.0,
            Weapon::Bow => 16.0,
            Weapon::Staff => 24.0,
            Weapon::Bomb => 6.0,
            Weapon::Longsword => 30.0,
        }
    }

    pub fn thickness(self) -> f32 {
        match self {
            Weapon::Sword => 2.0,
            Weapon::Club => 4.0,
            Weapon::Bow => 1.5,
            Weapon::Staff => 2.5,
            Weapon::Bomb => 5.0,
            Weapon::Longsword => 2.5,
        }
    }

    /// Orientation de l'arme **par rapport au bras** (0 = dans l'axe du bras).
    /// Positif = tourne vers l'avant.
    pub fn wrist_offset(self) -> f32 {
        match self {
            Weapon::Sword => 0.3,
            Weapon::Club => 0.2,
            Weapon::Bow => 0.0,
            Weapon::Staff => 0.1,
            Weapon::Bomb => 0.0,
            Weapon::Longsword => 0.25,
        }
    }

    /// Amplitude de rotation pendant le coup (par-dessus le mouvement du bras).
    pub fn swing_amplitude(self) -> f32 {
        match self {
            Weapon::Sword => 0.5,
            Weapon::Club => 0.7,
            Weapon::Bow => 0.2,
            Weapon::Staff => 0.3,
            Weapon::Bomb => 0.4,
            Weapon::Longsword => 0.6,
        }
    }

    pub fn color(self, camp_color: Color) -> Color {
        match self {
            Weapon::Sword | Weapon::Longsword => Color::new(
                (camp_color.r * 0.5 + 0.95 * 0.5).min(1.0),
                (camp_color.g * 0.5 + 0.95 * 0.5).min(1.0),
                (camp_color.b * 0.5 + 0.95 * 0.5).min(1.0),
                1.0,
            ),
            Weapon::Club => Color::new(0.45, 0.30, 0.18, 1.0),
            Weapon::Bow => Color::new(0.70, 0.50, 0.30, 1.0),
            Weapon::Staff => Color::new(1.0, 0.85, 0.35, 1.0),
            Weapon::Bomb => Color::new(0.12, 0.12, 0.15, 1.0),
        }
    }

    pub fn is_special(self) -> bool {
        matches!(self, Weapon::Bow | Weapon::Bomb)
    }
}

/// Ancrage de l'arme — dérivé du bout du bras.
pub struct WeaponAnchor {
    pub hand: Vec2,
    pub facing: f32,
    /// Angle du bras (celui utilisé pour dessiner le bras droit).
    pub arm_angle: f32,
    /// -0.5 = recule (windup), 0 = repos, 1.0 = strike à fond.
    pub attack_t: f32,
}

/// Dessine l'arme à l'ancre donnée.
pub fn draw_weapon(weapon: Weapon, anchor: WeaponAnchor, camp_color: Color) {
    // Angle de base = celui du bras + offset de poignet.
    let base_angle = anchor.arm_angle + weapon.wrist_offset();

    // Rotation supplémentaire pendant le coup (pivot du poignet).
    // Amplifie le mouvement du bras.
    let swing = anchor.attack_t * weapon.swing_amplitude();
    let final_angle = base_angle + swing;

    // Direction dans le plan (0 = vers le bas, positif = vers l'avant).
    let dir = Vec2::new(
        anchor.facing * final_angle.sin(),
        final_angle.cos(),
    );

    let tip = anchor.hand + dir * weapon.length();
    let color = weapon.color(camp_color);

    match weapon {
        Weapon::Sword | Weapon::Longsword | Weapon::Staff | Weapon::Club => {
            draw_line(
                anchor.hand.x,
                anchor.hand.y,
                tip.x,
                tip.y,
                weapon.thickness(),
                color,
            );
            if matches!(weapon, Weapon::Sword | Weapon::Longsword) {
                draw_circle(tip.x, tip.y, weapon.thickness() * 0.8, color);
            }
            if matches!(weapon, Weapon::Staff) {
                draw_circle(tip.x, tip.y, 3.0, color);
                draw_circle(tip.x, tip.y, 5.0, Color::new(color.r, color.g, color.b, 0.3));
            }
        }
        Weapon::Bow => {
            let perp = Vec2::new(-dir.y, dir.x) * weapon.length() * 0.4;
            let tip_a = anchor.hand + dir * weapon.length() * 0.3 + perp;
            let tip_b = anchor.hand + dir * weapon.length() * 0.3 - perp;
            draw_line(anchor.hand.x, anchor.hand.y, tip_a.x, tip_a.y, weapon.thickness(), color);
            draw_line(anchor.hand.x, anchor.hand.y, tip_b.x, tip_b.y, weapon.thickness(), color);
            if anchor.attack_t > 0.3 {
                draw_line(tip_a.x, tip_a.y, tip_b.x, tip_b.y, 1.0, color);
            }
        }
        Weapon::Bomb => {
            draw_circle(anchor.hand.x, anchor.hand.y, weapon.length() * 0.5, color);
            draw_circle(
                anchor.hand.x - 1.5,
                anchor.hand.y - 1.5,
                1.2,
                Color::new(0.4, 0.4, 0.5, 0.9),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weapon_for_kind_maps_all_units() {
        assert_eq!(Weapon::for_kind("grunt"), Weapon::Sword);
        assert_eq!(Weapon::for_kind("brute"), Weapon::Club);
        assert_eq!(Weapon::for_kind("archer"), Weapon::Bow);
        assert_eq!(Weapon::for_kind("healer"), Weapon::Staff);
        assert_eq!(Weapon::for_kind("bomber"), Weapon::Bomb);
        assert_eq!(Weapon::for_kind("hero"), Weapon::Longsword);
    }

    #[test]
    fn weapon_for_unknown_kind_defaults_to_sword() {
        assert_eq!(Weapon::for_kind("nope"), Weapon::Sword);
    }

    #[test]
    fn all_weapons_have_positive_length() {
        for w in [
            Weapon::Sword,
            Weapon::Club,
            Weapon::Bow,
            Weapon::Staff,
            Weapon::Bomb,
            Weapon::Longsword,
        ] {
            assert!(w.length() > 0.0);
        }
    }

    #[test]
    fn all_weapons_have_positive_thickness() {
        for w in [
            Weapon::Sword,
            Weapon::Club,
            Weapon::Bow,
            Weapon::Staff,
            Weapon::Bomb,
            Weapon::Longsword,
        ] {
            assert!(w.thickness() > 0.0);
        }
    }

    #[test]
    fn all_weapons_have_non_negative_swing_amplitude() {
        for w in [
            Weapon::Sword,
            Weapon::Club,
            Weapon::Bow,
            Weapon::Staff,
            Weapon::Bomb,
            Weapon::Longsword,
        ] {
            assert!(w.swing_amplitude() >= 0.0);
        }
    }

    #[test]
    fn bow_and_bomb_are_special() {
        assert!(Weapon::Bow.is_special());
        assert!(Weapon::Bomb.is_special());
        assert!(!Weapon::Sword.is_special());
    }

    #[test]
    fn staff_color_is_golden() {
        let c = Weapon::Staff.color(WHITE);
        assert!(c.r > 0.9 && c.g > 0.7 && c.b < 0.5);
    }

    #[test]
    fn sword_color_brightens_camp() {
        let dark = Color::new(0.2, 0.2, 0.2, 1.0);
        let c = Weapon::Sword.color(dark);
        assert!(c.r > dark.r);
        assert!(c.g > dark.g);
        assert!(c.b > dark.b);
    }

    #[test]
    fn weapon_angle_increases_with_attack() {
        // À bras constant, un attack_t plus grand donne un angle plus
        // élevé (vers l'avant).
        let base_angle = 0.5;
        let anchor_a = WeaponAnchor {
            hand: Vec2::ZERO,
            facing: 1.0,
            arm_angle: base_angle,
            attack_t: 0.0,
        };
        let anchor_b = WeaponAnchor {
            hand: Vec2::ZERO,
            facing: 1.0,
            arm_angle: base_angle,
            attack_t: 1.0,
        };
        let angle_a = anchor_a.arm_angle + Weapon::Sword.wrist_offset()
            + anchor_a.attack_t * Weapon::Sword.swing_amplitude();
        let angle_b = anchor_b.arm_angle + Weapon::Sword.wrist_offset()
            + anchor_b.attack_t * Weapon::Sword.swing_amplitude();
        assert!(angle_b > angle_a);
    }
}