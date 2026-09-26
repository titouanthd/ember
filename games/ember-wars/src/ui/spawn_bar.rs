//! Barre de spawn en bas de l'écran — un bouton par unité.

use glam::Vec2;
use macroquad::prelude::*;

use ember_stdlib::ui::hit;

use crate::config::GameContext;
use crate::systems::Game;
use crate::units::{all_units, can_spawn};

pub const BAR_HEIGHT: f32 = 90.0;
pub const SLOT_WIDTH: f32 = 120.0;
pub const SLOT_GAP: f32 = 8.0;
pub const BOTTOM_MARGIN: f32 = 10.0;

#[derive(Debug, Default)]
pub struct SpawnBar {
    pub hovered: Option<usize>,
}

impl SpawnBar {
    pub fn new() -> Self {
        Self { hovered: None }
    }

    pub fn slot_rect(idx: usize, total: usize, viewport_w: f32, viewport_h: f32) -> Rect {
        let total_w = total as f32 * SLOT_WIDTH + (total as f32 - 1.0) * SLOT_GAP;
        let start_x = (viewport_w - total_w) * 0.5;
        let y = viewport_h - BAR_HEIGHT - BOTTOM_MARGIN + 10.0;
        Rect {
            x: start_x + idx as f32 * (SLOT_WIDTH + SLOT_GAP),
            y,
            w: SLOT_WIDTH,
            h: BAR_HEIGHT - 20.0,
        }
    }

    pub fn draw(&self, ctx: &GameContext, game: &Game) {
        let stats = all_units();
        let total = stats.len();
        let vw = screen_width();
        let vh = screen_height();
        let mouse = Vec2::new(mouse_position().0, mouse_position().1);
        let unlocked = game.unlocked_units();

        for (i, s) in stats.iter().enumerate() {
            let r = Self::slot_rect(i, total, vw, vh);
            let is_unlocked = unlocked.contains(&s.id);
            let cooldown = game
                .player_cooldowns
                .get(&s.id)
                .copied()
                .unwrap_or(0.0);
            let enough_mana = game.player_mana.current >= s.cost;
            let max_alive_ok = can_spawn(&s.id, crate::components::Team::Player, &game.units);
            let ready = is_unlocked && cooldown <= 0.0 && enough_mana && max_alive_ok;

            let hovered = hit::contains(r, mouse);
            let base = s.color();
            let (bg, border) = if !is_unlocked {
                (
                    Color::new(0.09, 0.09, 0.11, 1.0),
                    Color::new(0.25, 0.25, 0.30, 1.0),
                )
            } else if ready {
                if hovered {
                    (
                        Color::new(base.r * 0.55, base.g * 0.55, base.b * 0.55, 1.0),
                        ctx.colors.accent,
                    )
                } else {
                    (Color::new(base.r * 0.35, base.g * 0.35, base.b * 0.35, 1.0), base)
                }
            } else {
                (
                    Color::new(0.18, 0.18, 0.22, 1.0),
                    Color::new(0.35, 0.35, 0.40, 1.0),
                )
            };

            draw_rectangle(r.x, r.y, r.w, r.h, bg);
            draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, border);

            // Cooldown overlay.
            if is_unlocked && cooldown > 0.0 {
                let frac = (cooldown / s.cooldown).clamp(0.0, 1.0);
                let h = r.h * frac;
                draw_rectangle(r.x, r.y, r.w, h, Color::new(0.0, 0.0, 0.0, 0.55));
            }

            // Nom.
            let name_y = r.y + 22.0;
            let name_color = if is_unlocked {
                ctx.colors.text
            } else {
                Color::new(0.40, 0.40, 0.45, 1.0)
            };
            draw_text(&s.name, r.x + 8.0, name_y, 18.0, name_color);

            // Coût.
            let cost_txt = format!("{:.0} mana", s.cost);
            let cost_color = if is_unlocked {
                ctx.colors.mana
            } else {
                Color::new(0.30, 0.30, 0.35, 1.0)
            };
            draw_text(&cost_txt, r.x + 8.0, r.y + 44.0, 14.0, cost_color);

            // État.
            let state_txt = if !is_unlocked {
                "locked".to_string()
            } else if !max_alive_ok {
                "max".to_string()
            } else if !enough_mana {
                "no mana".to_string()
            } else if cooldown > 0.0 {
                format!("{:.1}s", cooldown)
            } else {
                String::new()
            };
            if !state_txt.is_empty() {
                draw_text(&state_txt, r.x + 8.0, r.y + 62.0, 12.0, ctx.colors.text);
            }
        }
    }

    pub fn handle_click(&self, game: &Game) -> Option<String> {
        if !is_mouse_button_pressed(MouseButton::Left) {
            return None;
        }
        let stats = all_units();
        let total = stats.len();
        let vw = screen_width();
        let vh = screen_height();
        let m = Vec2::new(mouse_position().0, mouse_position().1);
        let unlocked = game.unlocked_units();
        for (i, s) in stats.iter().enumerate() {
            let r = Self::slot_rect(i, total, vw, vh);
            if hit::contains(r, m) {
                if !unlocked.contains(&s.id) {
                    return None;
                }
                let cooldown = game
                    .player_cooldowns
                    .get(&s.id)
                    .copied()
                    .unwrap_or(0.0);
                let enough_mana = game.player_mana.current >= s.cost;
                let max_ok = can_spawn(&s.id, crate::components::Team::Player, &game.units);
                if cooldown <= 0.0 && enough_mana && max_ok {
                    return Some(s.id.clone());
                }
                return None;
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_rect_is_inside_viewport() {
        let r = SpawnBar::slot_rect(0, 6, 1280.0, 720.0);
        assert!(r.x >= 0.0);
        assert!(r.x + r.w <= 1280.0);
        assert!(r.y + r.h <= 720.0);
    }

    #[test]
    fn slot_rects_are_ordered_and_not_overlapping() {
        let a = SpawnBar::slot_rect(0, 6, 1280.0, 720.0);
        let b = SpawnBar::slot_rect(1, 6, 1280.0, 720.0);
        assert!(b.x > a.x + a.w);
    }

    #[test]
    fn slot_rect_center_matches_hit() {
        let r = SpawnBar::slot_rect(3, 6, 1280.0, 720.0);
        let center = Vec2::new(r.x + r.w * 0.5, r.y + r.h * 0.5);
        assert!(hit::contains(r, center));
    }
}