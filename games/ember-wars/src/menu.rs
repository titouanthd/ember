//! Menu principal — sélection linéaire de niveaux + accès à l'arbre.

use macroquad::prelude::*;

use ember_stdlib::ui::hit;

use crate::config::{self, ChapterConfig, GameContext, Objective};
use crate::fonts;
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
    pub upgrade_scroll: Vec2,
    pub scroll_y: f32,
    pub max_scroll: f32,
}

impl MenuState {
    pub fn new() -> Self {
        Self::default()
    }
}

// ---------- Layout constants ----------

const HEADER_H: f32 = 96.0;
const FOOTER_H: f32 = 46.0;
const PAD_X: f32 = 60.0;
const PAD_TOP: f32 = 32.0;

const LEVEL_H: f32 = 88.0;
const LEVEL_GAP: f32 = 12.0;

const CHAPTER_HEADER_H: f32 = 96.0;
const CHAPTER_GAP: f32 = 44.0;
const CHAPTER_SEP_OFFSET: f32 = 20.0;

const INFO_PANEL_GAP: f32 = 32.0;
const INFO_PANEL_MIN_W: f32 = 320.0;
const INFO_PANEL_PAD: f32 = 24.0;

const SCROLLBAR_W: f32 = 6.0;
const SCROLLBAR_MIN_H: f32 = 40.0;

// ---------- Star colors (by tier) ----------

const STAR_LOCKED: Color = Color::new(0.26, 0.26, 0.30, 1.0);
const STAR_EMPTY: Color = Color::new(0.42, 0.42, 0.46, 1.0);
const STAR_BRONZE: Color = Color::new(0.72, 0.48, 0.28, 1.0);
const STAR_SILVER: Color = Color::new(0.80, 0.84, 0.90, 1.0);
const STAR_GOLD: Color = Color::new(1.00, 0.82, 0.30, 1.0);

/// Couleur d'une note selon le palier (0-3 étoiles).
fn star_tier_color(stars: u8) -> Color {
    match stars {
        0 => STAR_EMPTY,
        1 => STAR_BRONZE,
        2 => STAR_SILVER,
        3 => STAR_GOLD,
        _ => STAR_GOLD,
    }
}

/// Y de départ de la zone scrollable (juste en dessous du header).
pub const CONTENT_TOP: f32 = HEADER_H + PAD_TOP;

/// Réserve en bas pour le footer.
pub const CONTENT_BOTTOM_RESERVE: f32 = FOOTER_H + 20.0;

// ---------- Layout helpers ----------

/// Largeur des cartes de niveau.
/// Laisse la place au panneau d'info à droite.
fn level_w(vw: f32) -> f32 {
    let reserved = PAD_X * 2.0 + INFO_PANEL_GAP + INFO_PANEL_MIN_W;
    (vw - reserved).clamp(460.0, 820.0)
}

/// Rect d'un niveau dans un chapitre.
pub fn level_rect(row: usize, vw: f32, chapter_top_y: f32) -> Rect {
    let w = level_w(vw);
    let x = PAD_X;
    let y = chapter_top_y + CHAPTER_HEADER_H + row as f32 * (LEVEL_H + LEVEL_GAP);
    Rect { x, y, w, h: LEVEL_H }
}

/// Y du haut du chapitre `chapter_idx` (dans la coordonnée de scroll).
pub fn chapter_top_y(chapter_idx: usize, base_y: f32) -> f32 {
    let chapters = config::all_chapters();
    let mut y = base_y;
    for c in chapters.iter().take(chapter_idx) {
        y += CHAPTER_HEADER_H + c.levels.len() as f32 * (LEVEL_H + LEVEL_GAP) + CHAPTER_GAP;
    }
    y
}

/// Hauteur totale du contenu scrollable.
pub fn total_height() -> f32 {
    let chapters = config::all_chapters();
    let mut h = 0.0;
    for c in chapters {
        h += CHAPTER_HEADER_H + c.levels.len() as f32 * (LEVEL_H + LEVEL_GAP) + CHAPTER_GAP;
    }
    h
}

/// Rect du panneau d'info (à droite).
fn info_panel_rect(vw: f32, vh: f32) -> Rect {
    let x = PAD_X + level_w(vw) + INFO_PANEL_GAP;
    let y = CONTENT_TOP;
    let w = (vw - x - PAD_X).max(INFO_PANEL_MIN_W);
    let h = vh - CONTENT_TOP - CONTENT_BOTTOM_RESERVE;
    Rect { x, y, w, h }
}

/// X de la scrollbar : juste à gauche du panneau d'info, dans le gap.
fn scrollbar_x(vw: f32) -> f32 {
    PAD_X + level_w(vw) + INFO_PANEL_GAP - SCROLLBAR_W - 6.0
}

// ---------- Main draw ----------

pub fn draw_menu(ctx: &GameContext, state: &MenuState, progress: &Progress) {
    let vw = screen_width();
    let vh = screen_height();
    let base_y = CONTENT_TOP - state.scroll_y;

    // Ordre : chapitres (peuvent déborder) → scrollbar → panneau d'info
    // → header/footer opaques par-dessus pour masquer la superposition.
    draw_chapters(ctx, state, progress, vw, vh, base_y);
    draw_scrollbar(ctx, state, vw, vh);
    draw_info_panel(ctx, state, progress, vw, vh);
    draw_header(ctx, progress, vw);
    draw_footer(ctx, vw, vh);
}

fn draw_chapters(
    ctx: &GameContext,
    state: &MenuState,
    progress: &Progress,
    vw: f32,
    vh: f32,
    base_y: f32,
) {
    let chapters = config::all_chapters();
    let bottom_limit = vh - CONTENT_BOTTOM_RESERVE;

    for (ci, chapter) in chapters.iter().enumerate() {
        let chapter_y = chapter_top_y(ci, base_y);
        let chapter_h =
            CHAPTER_HEADER_H + chapter.levels.len() as f32 * (LEVEL_H + LEVEL_GAP);

        if chapter_y + chapter_h < 0.0 || chapter_y > bottom_limit {
            continue;
        }

        let chapter_unlocked = progress.is_chapter_unlocked(ci);

        if chapter_y + CHAPTER_HEADER_H >= CONTENT_TOP {
            draw_chapter_header(ctx, ci, chapter, chapter_y, chapter_unlocked, vw);
        }

        for (li, level) in chapter.levels.iter().enumerate() {
            let r = level_rect(li, vw, chapter_y);

            if r.y + r.h < 0.0 || r.y > bottom_limit {
                continue;
            }

            let unlocked = chapter_unlocked && progress.is_level_unlocked(&level.id);
            let stars = progress.stars_for(&level.id);
            let hovered = state.hovered_level.as_deref() == Some(level.id.as_str()) && unlocked;

            draw_level_card(ctx, (ci, li), level, r, stars, unlocked, hovered);
        }
    }
}

fn draw_chapter_header(
    ctx: &GameContext,
    chapter_idx: usize,
    chapter: &ChapterConfig,
    y: f32,
    unlocked: bool,
    vw: f32,
) {
    let lw = level_w(vw);

    fonts::draw_text_regular(
        &format!("CHAPTER {}", chapter_idx + 1),
        PAD_X,
        y + 22.0,
        14.0,
        Color::new(0.52, 0.54, 0.60, 1.0),
    );

    let name_color = if unlocked {
        ctx.colors.accent
    } else {
        Color::new(0.42, 0.42, 0.46, 1.0)
    };
    fonts::draw_text_bold(&chapter.name, PAD_X, y + 58.0, 34.0, name_color);

    let sub_color = if unlocked {
        Color::new(0.66, 0.68, 0.72, 1.0)
    } else {
        Color::new(0.38, 0.38, 0.42, 1.0)
    };
    fonts::draw_text_regular(
        &chapter.subtitle,
        PAD_X + 280.0,
        y + 58.0,
        17.0,
        sub_color,
    );

    let sep_y = y + CHAPTER_HEADER_H - CHAPTER_SEP_OFFSET;
    draw_line(
        PAD_X,
        sep_y,
        PAD_X + lw,
        sep_y,
        1.5,
        Color::new(0.20, 0.22, 0.28, 1.0),
    );
}

fn draw_level_card(
    ctx: &GameContext,
    idx: (usize, usize),
    level: &config::LevelConfig,
    r: Rect,
    stars: u8,
    unlocked: bool,
    hovered: bool,
) {
    let (chapter_idx, level_idx) = idx;
    let accent = star_tier_color(stars);

    let (bg, border) = if !unlocked {
        (
            Color::new(0.07, 0.07, 0.09, 1.0),
            Color::new(0.18, 0.18, 0.22, 1.0),
        )
    } else if stars > 0 {
        (Color::new(0.10, 0.12, 0.14, 1.0), accent)
    } else if hovered {
        (Color::new(0.16, 0.18, 0.22, 1.0), ctx.colors.accent)
    } else {
        (
            Color::new(0.11, 0.12, 0.15, 1.0),
            Color::new(0.30, 0.32, 0.38, 1.0),
        )
    };

    let bg = if hovered && unlocked {
        Color::new(
            (bg.r * 1.35).min(1.0),
            (bg.g * 1.35).min(1.0),
            (bg.b * 1.35).min(1.0),
            bg.a,
        )
    } else {
        bg
    };

    draw_rectangle(r.x, r.y, r.w, r.h, bg);
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.5, border);

    if unlocked && stars > 0 {
        draw_rectangle(r.x, r.y, 4.0, r.h, accent);
    }

    let num_txt = format!("{}-{}", chapter_idx + 1, level_idx + 1);
    let num_color = if unlocked {
        Color::new(0.62, 0.64, 0.70, 1.0)
    } else {
        Color::new(0.35, 0.35, 0.40, 1.0)
    };
    fonts::draw_text_regular(&num_txt, r.x + 24.0, r.y + 26.0, 14.0, num_color);

    let name_color = if unlocked {
        ctx.colors.text
    } else {
        Color::new(0.40, 0.40, 0.45, 1.0)
    };
    fonts::draw_text_bold(&level.name, r.x + 24.0, r.y + 54.0, 22.0, name_color);

    let obj_txt = match &level.objective {
        Objective::Destroy => "Destroy the enemy tower".to_string(),
        Objective::Survive(s) => format!("Survive {:.0} seconds", s),
    };
    let obj_color = if unlocked {
        Color::new(0.58, 0.60, 0.66, 1.0)
    } else {
        Color::new(0.32, 0.32, 0.36, 1.0)
    };
    fonts::draw_text_regular(&obj_txt, r.x + 24.0, r.y + 76.0, 13.0, obj_color);

    draw_star_rating(r, stars, unlocked);

    let reward = format!("+{:.0}g", level.reward_gold);
    let dim = fonts::measure_bold(&reward, 15.0);
    let reward_color = if unlocked {
        ctx.colors.gold
    } else {
        Color::new(0.38, 0.38, 0.42, 1.0)
    };
    fonts::draw_text_bold(
        &reward,
        r.x + r.w - 22.0 - dim.width,
        r.y + r.h - 14.0,
        15.0,
        reward_color,
    );
}

fn draw_star_rating(r: Rect, stars: u8, unlocked: bool) {
    let star_y = r.y + 34.0;
    let star_x_end = r.x + r.w - 22.0;
    let gap = 8.0;
    let star_w = 26.0;

    let total_w = 3.0 * star_w + 2.0 * gap;
    let start_x = star_x_end - total_w;

    let t = get_time() as f32;

    for i in 0..3u8 {
        let filled = i < stars;
        let x = start_x + i as f32 * (star_w + gap);

        let color = if !unlocked {
            STAR_LOCKED
        } else if filled {
            star_tier_color(stars)
        } else {
            STAR_EMPTY
        };

        let scale = if filled && unlocked {
            let phase = i as f32 * 0.7;
            1.0 + 0.05 * (t * 2.2 + phase).sin()
        } else {
            1.0
        };

        let size = 28.0 * scale;
        let y_off = (28.0 - size) * 0.5;
        let glyph = if filled { "★" } else { "☆" };
        fonts::draw_text_bold(glyph, x, star_y + y_off, size, color);
    }
}

// ---------- Scrollbar ----------

/// Fraction de scroll [0, 1], ou `None` si le contenu tient à l'écran.
pub fn scroll_fraction(scroll_y: f32, viewport_h: f32) -> Option<f32> {
    let content_h = total_height();
    let visible_h = (viewport_h - CONTENT_BOTTOM_RESERVE - CONTENT_TOP).max(1.0);
    let max_scroll = (content_h - visible_h).max(0.0);
    if max_scroll <= 0.0 {
        return None;
    }
    Some((scroll_y / max_scroll).clamp(0.0, 1.0))
}

fn draw_scrollbar(ctx: &GameContext, state: &MenuState, vw: f32, vh: f32) {
    let Some(frac) = scroll_fraction(state.scroll_y, vh) else {
        return;
    };

    let visible_h = (vh - CONTENT_BOTTOM_RESERVE - CONTENT_TOP).max(1.0);
    let content_h = total_height();
    let track_h = visible_h;

    let bar_x = scrollbar_x(vw);
    let bar_y = CONTENT_TOP;

    draw_rectangle(
        bar_x,
        bar_y,
        SCROLLBAR_W,
        track_h,
        Color::new(0.10, 0.11, 0.14, 1.0),
    );

    let ratio = (visible_h / content_h).clamp(0.0, 1.0);
    let thumb_h = (track_h * ratio).clamp(SCROLLBAR_MIN_H, track_h);
    let thumb_y = bar_y + (track_h - thumb_h) * frac;

    let accent = ctx.colors.accent;
    let thumb_color = Color::new(accent.r, accent.g, accent.b, 0.75);
    draw_rectangle(bar_x, thumb_y, SCROLLBAR_W, thumb_h, thumb_color);
}

// ---------- Info panel ----------

fn draw_info_panel(
    ctx: &GameContext,
    state: &MenuState,
    progress: &Progress,
    vw: f32,
    vh: f32,
) {
    let panel = info_panel_rect(vw, vh);

    draw_rectangle(
        panel.x,
        panel.y,
        panel.w,
        panel.h,
        Color::new(0.06, 0.07, 0.09, 0.75),
    );
    draw_rectangle_lines(
        panel.x,
        panel.y,
        panel.w,
        panel.h,
        1.0,
        Color::new(0.18, 0.20, 0.24, 1.0),
    );

    let x = panel.x + INFO_PANEL_PAD;
    let w = panel.w - INFO_PANEL_PAD * 2.0;

    let hovered_id = match &state.hovered_level {
        Some(id) => id,
        None => {
            fonts::draw_text_regular(
                "Hover a mission",
                x,
                panel.y + 40.0,
                15.0,
                Color::new(0.45, 0.45, 0.50, 1.0),
            );
            return;
        }
    };

    let level = match config::level_by_id(hovered_id) {
        Some(l) => l,
        None => return,
    };

    let chapter = config::chapter_of_level(hovered_id);
    let chapter_unlocked = chapter
        .and_then(|c| config::all_chapters().iter().position(|x| x.id == c.id))
        .map(|idx| progress.is_chapter_unlocked(idx))
        .unwrap_or(false);
    let level_unlocked = chapter_unlocked && progress.is_level_unlocked(&level.id);
    let stars = progress.stars_for(&level.id);

    let mut y = panel.y + 36.0;

    if let Some(c) = chapter {
        let chapter_idx = config::all_chapters()
            .iter()
            .position(|x| x.id == c.id)
            .unwrap_or(0);
        fonts::draw_text_regular(
            &format!("CHAPTER {} — {}", chapter_idx + 1, c.name),
            x,
            y,
            13.0,
            Color::new(0.55, 0.58, 0.62, 1.0),
        );
        y += 28.0;
    }

    fonts::draw_text_bold(&level.name, x, y + 12.0, 26.0, ctx.colors.text);
    y += 40.0;

    draw_line(x, y, x + w, y, 1.0, Color::new(0.22, 0.24, 0.30, 1.0));
    y += 22.0;

    fonts::draw_text_regular(
        "OBJECTIVE",
        x,
        y,
        11.0,
        Color::new(0.50, 0.52, 0.58, 1.0),
    );
    y += 20.0;
    let obj_txt = match &level.objective {
        Objective::Destroy => "Destroy the enemy tower".to_string(),
        Objective::Survive(s) => format!("Survive {:.0} seconds", s),
    };
    fonts::draw_text_regular(&obj_txt, x, y, 15.0, ctx.colors.text);
    y += 32.0;

    draw_line(x, y, x + w, y, 1.0, Color::new(0.22, 0.24, 0.30, 1.0));
    y += 22.0;

    fonts::draw_text_regular(
        "STARS",
        x,
        y,
        11.0,
        Color::new(0.50, 0.52, 0.58, 1.0),
    );
    y += 28.0;

    let (star_str, star_color) = if !level_unlocked {
        ("☆ ☆ ☆".to_string(), STAR_LOCKED)
    } else if stars == 0 {
        ("☆ ☆ ☆".to_string(), STAR_EMPTY)
    } else {
        let mut s = String::new();
        for _ in 0..stars {
            s.push('★');
        }
        for _ in stars..3 {
            s.push('☆');
        }
        (s, star_tier_color(stars))
    };
    fonts::draw_text_bold(&star_str, x, y, 32.0, star_color);
    y += 38.0;

    if level_unlocked {
        fonts::draw_text_regular(
            "REWARD",
            x,
            y,
            11.0,
            Color::new(0.50, 0.52, 0.58, 1.0),
        );
        y += 20.0;
        fonts::draw_text_bold(
            &format!("+{:.0} gold", level.reward_gold),
            x,
            y,
            18.0,
            ctx.colors.gold,
        );
    }

    if !level_unlocked {
        let notice = if !chapter_unlocked {
            "Complete the previous chapter"
        } else {
            "Complete the previous mission"
        };
        fonts::draw_text_regular(
            notice,
            x,
            panel.y + panel.h - 28.0,
            14.0,
            Color::new(0.65, 0.48, 0.48, 1.0),
        );
    }
}

// ---------- Header / Footer ----------

fn draw_header(ctx: &GameContext, progress: &Progress, vw: f32) {
    draw_rectangle(0.0, 0.0, vw, HEADER_H, ctx.colors.bg);

    draw_line(
        0.0,
        HEADER_H - 1.0,
        vw,
        HEADER_H - 1.0,
        1.0,
        Color::new(0.18, 0.20, 0.24, 1.0),
    );

    let title = "EMBER WARS";
    let dim = fonts::measure_bold(title, 38.0);
    fonts::draw_text_bold(title, PAD_X, 58.0, 38.0, ctx.colors.accent);

    fonts::draw_text_regular(
        "CAMPAIGN",
        PAD_X + dim.width + 22.0,
        58.0,
        15.0,
        Color::new(0.50, 0.52, 0.58, 1.0),
    );

    let gold_txt = format!("GOLD  {:.0}", progress.tree.gold);
    let dim_g = fonts::measure_bold(&gold_txt, 22.0);
    fonts::draw_text_bold(
        &gold_txt,
        vw - PAD_X - dim_g.width,
        50.0,
        22.0,
        ctx.colors.gold,
    );

    let hint = "[Tab]  Upgrades";
    let dim_h = fonts::measure_regular(hint, 14.0);
    fonts::draw_text_regular(
        hint,
        vw - PAD_X - dim_h.width,
        76.0,
        14.0,
        Color::new(0.60, 0.62, 0.68, 1.0),
    );
}

fn draw_footer(ctx: &GameContext, vw: f32, vh: f32) {
    draw_rectangle(
        0.0,
        vh - FOOTER_H,
        vw,
        FOOTER_H,
        ctx.colors.bg,
    );

    draw_line(
        0.0,
        vh - FOOTER_H + 1.0,
        vw,
        vh - FOOTER_H + 1.0,
        1.0,
        Color::new(0.18, 0.20, 0.24, 1.0),
    );

    let hint =
        "Click a mission   |   Scroll ↑↓ or wheel   |   [Tab] Upgrades   |   [Esc] Quit";
    let dim = fonts::measure_regular(hint, 14.0);
    fonts::draw_text_regular(
        hint,
        vw * 0.5 - dim.width * 0.5,
        vh - 16.0,
        14.0,
        Color::new(0.55, 0.58, 0.62, 1.0),
    );
}

// ---------- Input ----------

pub fn handle_menu_input(state: &mut MenuState, progress: &Progress) -> Option<String> {
    let vw = screen_width();
    let vh = screen_height();
    let mouse = Vec2::new(mouse_position().0, mouse_position().1);

    let content_h = total_height();
    let visible_h = (vh - CONTENT_BOTTOM_RESERVE - CONTENT_TOP).max(1.0);
    state.max_scroll = (content_h - visible_h).max(0.0);

    let (_wx, wy) = mouse_wheel();
    if wy.abs() > 0.0 {
        state.scroll_y = (state.scroll_y - wy * 40.0).clamp(0.0, state.max_scroll);
    }

    if is_key_down(KeyCode::Down) {
        state.scroll_y =
            (state.scroll_y + 500.0 * get_frame_time()).min(state.max_scroll);
    }
    if is_key_down(KeyCode::Up) {
        state.scroll_y = (state.scroll_y - 500.0 * get_frame_time()).max(0.0);
    }

    let base_y = CONTENT_TOP - state.scroll_y;
    let chapters = config::all_chapters();
    let bottom_limit = vh - CONTENT_BOTTOM_RESERVE;

    state.hovered_level = None;
    if mouse.y >= CONTENT_TOP && mouse.y <= bottom_limit {
        for (ci, chapter) in chapters.iter().enumerate() {
            let chapter_y = chapter_top_y(ci, base_y);
            for (li, level) in chapter.levels.iter().enumerate() {
                let r = level_rect(li, vw, chapter_y);
                if r.y + r.h < CONTENT_TOP || r.y > bottom_limit {
                    continue;
                }
                if hit::contains(r, mouse) {
                    state.hovered_level = Some(level.id.clone());
                }
            }
        }
    }

    if is_mouse_button_pressed(MouseButton::Left) {
        if mouse.y < CONTENT_TOP || mouse.y > vh - CONTENT_BOTTOM_RESERVE {
            return None;
        }
        if hit::contains(info_panel_rect(vw, vh), mouse) {
            return None;
        }

        for (ci, chapter) in chapters.iter().enumerate() {
            let chapter_y = chapter_top_y(ci, base_y);
            let chapter_unlocked = progress.is_chapter_unlocked(ci);
            if !chapter_unlocked {
                continue;
            }
            for (li, level) in chapter.levels.iter().enumerate() {
                let r = level_rect(li, vw, chapter_y);
                if r.y + r.h < CONTENT_TOP || r.y > bottom_limit {
                    continue;
                }
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
    fn chapters_stack_vertically() {
        let y0 = chapter_top_y(0, 130.0);
        let y1 = chapter_top_y(1, 130.0);
        assert!(y1 > y0);
    }

    #[test]
    fn total_height_is_positive() {
        assert!(total_height() > 0.0);
    }

    #[test]
    fn chapter_top_y_scrolls_with_base() {
        let y_normal = chapter_top_y(1, 130.0);
        let y_scrolled = chapter_top_y(1, 130.0 - 100.0);
        assert!((y_scrolled - (y_normal - 100.0)).abs() < 1e-3);
    }

    #[test]
    fn level_w_clamps_on_small_screens() {
        assert!((level_w(800.0) - 460.0).abs() < 1e-6);
        assert!((level_w(3000.0) - 820.0).abs() < 1e-6);
    }

    #[test]
    fn star_tier_color_differs_by_stars() {
        assert_eq!(star_tier_color(0), STAR_EMPTY);
        assert_eq!(star_tier_color(1), STAR_BRONZE);
        assert_eq!(star_tier_color(2), STAR_SILVER);
        assert_eq!(star_tier_color(3), STAR_GOLD);
    }

    #[test]
    fn scroll_fraction_none_when_content_fits() {
        assert!(scroll_fraction(0.0, 10_000.0).is_none());
    }

    #[test]
    fn scroll_fraction_is_zero_at_top() {
        let f = scroll_fraction(0.0, 720.0).expect("content overflows at 720");
        assert!((f - 0.0).abs() < 1e-6);
    }

    #[test]
    fn scroll_fraction_is_one_at_bottom() {
        let f = scroll_fraction(99_999.0, 720.0).expect("content overflows");
        assert!((f - 1.0).abs() < 1e-6);
    }

    #[test]
    fn scroll_fraction_is_monotonic() {
        let f_mid = scroll_fraction(150.0, 720.0).expect("overflows");
        let f_top = scroll_fraction(0.0, 720.0).unwrap();
        let f_bot = scroll_fraction(99_999.0, 720.0).unwrap();
        assert!(f_top <= f_mid && f_mid <= f_bot);
    }
}