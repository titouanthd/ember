//! Menu principal — sélection linéaire de niveaux + accès à l'arbre.

use macroquad::prelude::*;

use ember_stdlib::ui::hit;

use crate::config::{all_chapters, GameContext, Objective};
use crate::progress::Progress;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppScreen {
    Menu,
    Game,
}

#[derive(Debug, Default)]
pub struct MenuState {
    pub hovered_level: Option<String>,
    pub upgrade_modal_open: bool,
    pub upgrade_focus: Option<usize>,
}

impl MenuState {
    pub fn new() -> Self {
        Self::default()
    }
}

const CHAPTER_HEADER_H: f32 = 70.0;
const CHAPTER_SEPARATOR_H: f32 = 30.0;
const LEVEL_H: f32 = 72.0;
const LEVEL_W: f32 = 700.0;
const LEVEL_GAP: f32 = 10.0;

/// Rect d'un bouton de niveau dans la disposition linéaire verticale.
/// `row` est la position du niveau dans le chapitre (0-based).
/// `chapter_top_y` est la coordonnée Y du haut du chapitre.
pub fn level_rect(row: usize, vw: f32, chapter_top_y: f32) -> Rect {
    let x = (vw - LEVEL_W) * 0.5;
    let y = chapter_top_y + CHAPTER_HEADER_H + row as f32 * (LEVEL_H + LEVEL_GAP);
    Rect {
        x,
        y,
        w: LEVEL_W,
        h: LEVEL_H,
    }
}

/// Calcule la coordonnée Y du haut d'un chapitre, en tenant compte des
/// chapitres précédents.
pub fn chapter_top_y(chapter_idx: usize, vw: f32, top_y: f32) -> f32 {
    let chapters = all_chapters();
    let mut y = top_y;
    for c in chapters.iter().take(chapter_idx) {
        y += CHAPTER_HEADER_H
            + c.levels.len() as f32 * (LEVEL_H + LEVEL_GAP)
            + CHAPTER_SEPARATOR_H;
    }
    let _ = vw;
    y
}

/// Hauteur totale occupée par tous les chapitres (pour scroll éventuel).
pub fn total_height(top_y: f32) -> f32 {
    let chapters = all_chapters();
    let mut h = 0.0;
    for c in chapters {
        h += CHAPTER_HEADER_H
            + c.levels.len() as f32 * (LEVEL_H + LEVEL_GAP)
            + CHAPTER_SEPARATOR_H;
    }
    top_y + h
}

pub fn draw_menu(ctx: &GameContext, _state: &MenuState, progress: &Progress) {
    let vw = screen_width();
    let vh = screen_height();
    let mouse = Vec2::new(mouse_position().0, mouse_position().1);

    // Titre + bank gold.
    let title = "EMBER WARS";
    let dim = measure_text(title, None, 60, 1.0);
    draw_text(title, vw * 0.5 - dim.width * 0.5, 70.0, 60.0, ctx.colors.accent);

    let sub = "Choose a mission";
    let dim2 = measure_text(sub, None, 20, 1.0);
    draw_text(sub, vw * 0.5 - dim2.width * 0.5, 102.0, 20.0, ctx.colors.text);

    // Bank gold (top-right).
    let gold_txt = format!("GOLD {:.0}", progress.tree.gold);
    let dim_g = measure_text(&gold_txt, None, 26, 1.0);
    draw_text(
        &gold_txt,
        vw - dim_g.width - 30.0,
        50.0,
        26.0,
        ctx.colors.gold,
    );

    // Hint "Tab: Upgrades" (top-right, sous gold).
    let hint = "[Tab] Upgrades";
    let dim_h = measure_text(hint, None, 16, 1.0);
    draw_text(
        hint,
        vw - dim_h.width - 30.0,
        78.0,
        16.0,
        Color::new(0.75, 0.75, 0.80, 1.0),
    );

    // Hint en bas.
    let bottom_hint = "Click a mission   |   Esc to quit";
    let dim_b = measure_text(bottom_hint, None, 16, 1.0);
    draw_text(
        bottom_hint,
        vw * 0.5 - dim_b.width * 0.5,
        vh - 20.0,
        16.0,
        Color::new(0.65, 0.65, 0.70, 1.0),
    );

    // Chapters + levels (linéaire vertical).
    let top_y = 130.0;
    let chapters = all_chapters();
    for (ci, chapter) in chapters.iter().enumerate() {
        let chapter_y = chapter_top_y(ci, vw, top_y);
        let chapter_unlocked = progress.is_chapter_unlocked(ci);

        // Header.
        draw_text(
            format!("CHAPTER {}", ci + 1),
            60.0,
            chapter_y + 24.0,
            16.0,
            Color::new(0.65, 0.65, 0.70, 1.0),
        );
        let name_color = if chapter_unlocked {
            ctx.colors.accent
        } else {
            Color::new(0.45, 0.45, 0.50, 1.0)
        };
        draw_text(&chapter.name, 60.0, chapter_y + 52.0, 30.0, name_color);
        let sub_color = if chapter_unlocked {
            Color::new(0.70, 0.70, 0.75, 1.0)
        } else {
            Color::new(0.35, 0.35, 0.40, 1.0)
        };
        draw_text(
            &chapter.subtitle,
            280.0,
            chapter_y + 52.0,
            18.0,
            sub_color,
        );

        // Ligne horizontale de séparation.
        draw_line(
            60.0,
            chapter_y + CHAPTER_HEADER_H - 4.0,
            vw - 60.0,
            chapter_y + CHAPTER_HEADER_H - 4.0,
            1.0,
            Color::new(0.25, 0.28, 0.34, 1.0),
        );

        // Niveaux.
        for (li, level) in chapter.levels.iter().enumerate() {
            let r = level_rect(li, vw, chapter_y);
            let unlocked = chapter_unlocked && progress.is_level_unlocked(&level.id);
            let stars = progress.stars_for(&level.id);
            let hovered = hit::contains(r, mouse) && unlocked;

            let (bg, border) = if !unlocked {
                (
                    Color::new(0.08, 0.08, 0.10, 1.0),
                    Color::new(0.22, 0.22, 0.26, 1.0),
                )
            } else if stars > 0 {
                (
                    Color::new(0.10, 0.20, 0.12, 1.0),
                    Color::new(0.40, 0.85, 0.40, 1.0),
                )
            } else if hovered {
                (Color::new(0.22, 0.26, 0.34, 1.0), ctx.colors.accent)
            } else {
                (
                    Color::new(0.14, 0.18, 0.24, 1.0),
                    Color::new(0.55, 0.55, 0.60, 1.0),
                )
            };

            draw_rectangle(r.x, r.y, r.w, r.h, bg);
            draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, border);

            // Numéro + nom (à gauche).
            let name_color = if unlocked {
                ctx.colors.text
            } else {
                Color::new(0.40, 0.40, 0.45, 1.0)
            };
            let name = format!("{}-{}  {}", ci + 1, li + 1, level.name);
            draw_text(&name, r.x + 20.0, r.y + 30.0, 22.0, name_color);

            // Objectif (en dessous du nom).
            let info = if !unlocked {
                "Locked".to_string()
            } else {
                match &level.objective {
                    Objective::Destroy => "Destroy the enemy tower".to_string(),
                    Objective::Survive(s) => format!("Survive {:.0} seconds", s),
                }
            };
            let info_color = if unlocked {
                Color::new(0.75, 0.75, 0.80, 1.0)
            } else {
                Color::new(0.38, 0.38, 0.42, 1.0)
            };
            draw_text(&info, r.x + 20.0, r.y + 56.0, 15.0, info_color);

            // Étoiles (à droite).
            let star_text = if unlocked {
                let mut s = String::new();
                for _ in 0..stars {
                    s.push('★');
                }
                for _ in stars..3 {
                    s.push('☆');
                }
                s
            } else {
                "Locked".to_string()
            };

            let dim_s = measure_text(&star_text, None, 26, 1.0);
            let star_color = if stars == 3 {
                ctx.colors.gold
            } else if stars > 0 {
                ctx.colors.accent
            } else {
                Color::new(0.45, 0.45, 0.50, 1.0)
            };
            draw_text(
                &star_text,
                r.x + r.w - dim_s.width - 100.0,
                r.y + 44.0,
                26.0,
                star_color,
            );

            // Récompense (tout à droite).
            let reward = format!("+{:.0}g", level.reward_gold);
            let dim_r = measure_text(&reward, None, 18, 1.0);
            let reward_color = if unlocked { ctx.colors.gold } else { Color::new(0.40, 0.40, 0.45, 1.0) };
            draw_text(
                &reward,
                r.x + r.w - dim_r.width - 20.0,
                r.y + 44.0,
                18.0,
                reward_color,
            );
        }
    }

    let _ = vh;
}

/// Traite l'input du menu principal (hors modal). Retourne l'id du
/// niveau sélectionné si on démarre.
pub fn handle_menu_input(state: &mut MenuState, progress: &Progress) -> Option<String> {
    let vw = screen_width();
    let top_y = 130.0;
    let mouse = Vec2::new(mouse_position().0, mouse_position().1);
    let chapters = all_chapters();

    state.hovered_level = None;
    for (ci, chapter) in chapters.iter().enumerate() {
        let chapter_y = chapter_top_y(ci, vw, top_y);
        let chapter_unlocked = progress.is_chapter_unlocked(ci);
        for (li, level) in chapter.levels.iter().enumerate() {
            let r = level_rect(li, vw, chapter_y);
            if hit::contains(r, mouse) {
                state.hovered_level = Some(level.id.clone());
                if !chapter_unlocked {
                    // Rien à faire, juste signaler le hover.
                }
            }
        }
    }

    if is_mouse_button_pressed(MouseButton::Left) {
        for (ci, chapter) in chapters.iter().enumerate() {
            let chapter_y = chapter_top_y(ci, vw, top_y);
            let chapter_unlocked = progress.is_chapter_unlocked(ci);
            if !chapter_unlocked {
                continue;
            }
            for (li, level) in chapter.levels.iter().enumerate() {
                let r = level_rect(li, vw, chapter_y);
                if hit::contains(r, mouse) && progress.is_level_unlocked(&level.id) {
                    return Some(level.id.clone());
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_stack_vertically_within_chapter() {
        let a = level_rect(0, 1920.0, 130.0);
        let b = level_rect(1, 1920.0, 130.0);
        assert!(b.y > a.y + a.h);
    }

    #[test]
    fn level_rect_is_centered() {
        let r = level_rect(0, 1920.0, 130.0);
        let center = r.x + r.w * 0.5;
        assert!((center - 960.0).abs() < 1e-3);
    }

    #[test]
    fn chapters_stack_vertically() {
        let y0 = chapter_top_y(0, 1920.0, 130.0);
        let y1 = chapter_top_y(1, 1920.0, 130.0);
        assert!(y1 > y0);
    }
}