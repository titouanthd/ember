//! Caméra 2D locale. À extraire dans `ember_stdlib` au 2e jeu à scroll.

use crate::components::{Tower, Unit};

#[allow(dead_code)]
const CAMERA_SMOOTH_K: f32 = 8.0;

pub const CAMERA_SCROLL_SPEED: f32 = 900.0;

#[derive(Debug, Clone)]
pub struct Camera2D {
    pub x: f32,
    pub viewport_w: f32,
    pub map_w: f32,
}

impl Camera2D {
    pub fn new(viewport_w: f32, map_w: f32) -> Self {
        Self {
            x: 0.0,
            viewport_w,
            map_w,
        }
    }

    pub fn max_x(&self) -> f32 {
        (self.map_w - self.viewport_w).max(0.0)
    }

    pub fn clamp(&mut self) {
        self.x = self.x.clamp(0.0, self.max_x());
    }

    pub fn scroll(&mut self, dt: f32, dir: f32) {
        if dt <= 0.0 || dir == 0.0 {
            return;
        }
        self.x += dir.clamp(-1.0, 1.0) * CAMERA_SCROLL_SPEED * dt;
        self.clamp();
    }

    #[allow(dead_code)]
    pub fn follow_front(&mut self, units: &[Unit], towers: &[Tower], dt: f32) {
        let target_x = compute_target_x(units, towers);
        let desired = target_x - self.viewport_w * 0.5;
        let t = 1.0 - (-dt * CAMERA_SMOOTH_K).exp();
        self.x += (desired - self.x) * t;
        self.clamp();
    }

    pub fn world_to_screen(&self, world_x: f32) -> f32 {
        world_x - self.x
    }

    pub fn screen_to_world(&self, screen_x: f32) -> f32 {
        screen_x + self.x
    }
}

#[allow(dead_code)]
pub fn compute_target_x(units: &[Unit], towers: &[Tower]) -> f32 {
    let alive: Vec<&Unit> = units.iter().filter(|u| u.is_alive()).collect();
    if alive.is_empty() {
        return towers_center_x(towers);
    }

    let total_hp: f32 = alive.iter().map(|u| u.hp.max(0.0)).sum();
    if total_hp <= 0.0 {
        return towers_center_x(towers);
    }

    alive
        .iter()
        .map(|u| u.pos.x * u.hp.max(0.0))
        .sum::<f32>()
        / total_hp
}

#[allow(dead_code)]
fn towers_center_x(towers: &[Tower]) -> f32 {
    if towers.is_empty() {
        return 0.0;
    }
    towers.iter().map(|t| t.x).sum::<f32>() / towers.len() as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    use glam::Vec2;

    use crate::components::{Team, UnitId};
    use crate::units::unit_stats;

    fn mk_unit(id: u32, kind: &str, team: Team, x: f32, hp_frac: f32) -> Unit {
        let stats = unit_stats(kind).expect("kind exists");
        Unit {
            id: UnitId(id),
            kind: kind.to_owned(),
            team,
            pos: Vec2::new(x, 0.0),
            hp: stats.hp * hp_frac,
            max_hp: stats.hp,
            damage: stats.damage,
            attack_cd: 0.0,
            heal_cd: 0.0,
            hit_flash: 0.0,
            target: None,
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

    // ---------- scroll ----------

    #[test]
    fn scroll_right_moves_camera_forward() {
        let mut cam = Camera2D::new(1280.0, 2400.0);
        cam.scroll(1.0, 1.0);
        assert!((cam.x - CAMERA_SCROLL_SPEED).abs() < 1e-3);
    }

    #[test]
    fn scroll_left_moves_camera_backward() {
        let mut cam = Camera2D::new(1280.0, 2400.0);
        cam.x = 500.0;
        cam.scroll(1.0, -1.0);
        assert!(cam.x < 500.0);
    }

    #[test]
    fn scroll_clamps_at_right_edge() {
        let mut cam = Camera2D::new(1280.0, 2400.0);
        cam.scroll(10.0, 1.0);
        assert!((cam.x - 1120.0).abs() < 1e-3);
    }

    #[test]
    fn scroll_clamps_at_left_edge() {
        let mut cam = Camera2D::new(1280.0, 2400.0);
        cam.x = 100.0;
        cam.scroll(10.0, -1.0);
        assert_eq!(cam.x, 0.0);
    }

    #[test]
    fn scroll_ignores_zero_dir_or_zero_dt() {
        let mut cam = Camera2D::new(1280.0, 2400.0);
        cam.x = 500.0;
        cam.scroll(1.0, 0.0);
        assert_eq!(cam.x, 500.0);
        cam.scroll(0.0, 1.0);
        assert_eq!(cam.x, 500.0);
    }

    #[test]
    fn scroll_dir_is_clamped() {
        let mut cam = Camera2D::new(1280.0, 100_000.0);
        cam.scroll(1.0, 5.0);
        assert!((cam.x - CAMERA_SCROLL_SPEED).abs() < 1e-3);
        cam.x = 0.0;
        cam.scroll(1.0, -5.0);
        assert_eq!(cam.x, 0.0);
    }

    #[test]
    fn scroll_does_nothing_when_map_fits_viewport() {
        let mut cam = Camera2D::new(2000.0, 1000.0);
        cam.scroll(1.0, 1.0);
        assert_eq!(cam.x, 0.0);
    }

    // ---------- world/screen ----------

    #[test]
    fn world_screen_round_trip() {
        let cam = Camera2D::new(1280.0, 2000.0);
        let w = 950.0;
        let s = cam.world_to_screen(w);
        assert!((cam.screen_to_world(s) - w).abs() < 1e-6);
    }

    #[test]
    fn clamp_keeps_in_bounds() {
        let mut cam = Camera2D::new(1280.0, 2000.0);
        cam.x = -100.0;
        cam.clamp();
        assert_eq!(cam.x, 0.0);
        cam.x = 9999.0;
        cam.clamp();
        assert_eq!(cam.x, 720.0);
    }

    // ---------- compute_target_x ----------

    #[test]
    fn target_uses_hp_weighted_barycenter() {
        let mut u1 = mk_unit(0, "grunt", Team::Player, 0.0, 1.0);
        u1.hp = 100.0;
        let mut u2 = mk_unit(1, "grunt", Team::Enemy, 100.0, 1.0);
        u2.hp = 300.0;
        let units = vec![u1, u2];
        let towers = vec![mk_tower(Team::Player, -100.0), mk_tower(Team::Enemy, 1000.0)];
        assert!((compute_target_x(&units, &towers) - 75.0).abs() < 1e-6);
    }

    #[test]
    fn target_returns_towers_center_when_no_units() {
        let towers = vec![mk_tower(Team::Player, 100.0), mk_tower(Team::Enemy, 500.0)];
        assert!((compute_target_x(&[], &towers) - 300.0).abs() < 1e-6);
    }

    #[test]
    fn target_ignores_dead_units() {
        let dead = {
            let mut u = mk_unit(0, "grunt", Team::Player, 1000.0, 1.0);
            u.hp = 0.0;
            u
        };
        let alive = mk_unit(1, "grunt", Team::Enemy, 200.0, 1.0);
        let units = vec![dead, alive];
        let towers = vec![mk_tower(Team::Player, 0.0), mk_tower(Team::Enemy, 1000.0)];
        assert!((compute_target_x(&units, &towers) - 200.0).abs() < 1e-6);
    }

    #[test]
    fn target_returns_towers_center_when_all_dead() {
        let mut d1 = mk_unit(0, "grunt", Team::Player, 50.0, 1.0);
        d1.hp = 0.0;
        let mut d2 = mk_unit(1, "grunt", Team::Enemy, 900.0, 1.0);
        d2.hp = 0.0;
        let units = vec![d1, d2];
        let towers = vec![mk_tower(Team::Player, 100.0), mk_tower(Team::Enemy, 500.0)];
        assert!((compute_target_x(&units, &towers) - 300.0).abs() < 1e-6);
    }

    #[test]
    fn follow_front_no_move_when_dt_zero() {
        let mut cam = Camera2D::new(1280.0, 2400.0);
        cam.x = 100.0;
        let units = vec![mk_unit(0, "grunt", Team::Player, 1200.0, 1.0)];
        let towers = vec![mk_tower(Team::Player, 0.0), mk_tower(Team::Enemy, 2400.0)];
        cam.follow_front(&units, &towers, 0.0);
        assert!((cam.x - 100.0).abs() < 1e-6);
    }

    #[test]
    fn follow_front_converges_to_target_with_large_dt() {
        let mut cam = Camera2D::new(1280.0, 2400.0);
        let units = vec![mk_unit(0, "grunt", Team::Player, 1200.0, 1.0)];
        let towers = vec![mk_tower(Team::Player, 0.0), mk_tower(Team::Enemy, 2400.0)];
        cam.follow_front(&units, &towers, 10.0);
        assert!((cam.x - 560.0).abs() < 1e-3);
    }

    #[test]
    fn follow_front_uses_towers_when_no_units() {
        let mut cam = Camera2D::new(1280.0, 2400.0);
        let towers = vec![mk_tower(Team::Player, 200.0), mk_tower(Team::Enemy, 1800.0)];
        cam.follow_front(&[], &towers, 10.0);
        assert!((cam.x - 360.0).abs() < 1e-3);
    }
}