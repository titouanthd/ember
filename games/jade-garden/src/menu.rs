//! Main menu — chapter / level selection screen.
//!
//! Fully self-contained: draws its own dark lacquer background and uses
//! a local palette so `.env` overrides can't break menu readability.
//! The header and footer are drawn LAST with opaque backgrounds, so
//! level cards can never bleed into them.
//!
//! Layout: 4 chapters × 3 levels, scrollable. Right side is an info
//! panel for the hovered level. All user-facing strings are English.

use glam::Vec2;
use macroquad::prelude::*;

use crate::config::GameContext;
use crate::fonts::PoemFont;
use crate::level::{objective_label, star_tier_label, ChapterConfig, LevelConfig};
use crate::progress::Progress;

// ---------- Local palette (does not depend on .env) ----------

mod palette {
    use macroquad::prelude::*;

    // Dark lacquer background gradient.
    pub const BG_TOP: Color    = Color::new(0.105, 0.080, 0.055, 1.0);
    pub const BG_BOTTOM: Color = Color::new(0.045, 0.032, 0.022, 1.0);

    // Header / footer opaque bars.
    pub const BAR_BG: Color       = Color::new(0.075, 0.058, 0.040, 1.0);
    pub const BAR_BORDER: Color   = Color::new(0.42, 0.32, 0.16, 1.0);

    // Level card states.
    pub const CARD_BG: Color             = Color::new(0.135, 0.105, 0.070, 1.0);
    pub const CARD_BG_HOVER: Color       = Color::new(0.215, 0.165, 0.095, 1.0);
    pub const CARD_BORDER: Color         = Color::new(0.36, 0.28, 0.16, 1.0);
    pub const CARD_LOCKED_BG: Color      = Color::new(0.075, 0.062, 0.048, 1.0);
    pub const CARD_LOCKED_BORDER: Color  = Color::new(0.20, 0.165, 0.115, 1.0);

    // Info panel.
    pub const PANEL_BG: Color     = Color::new(0.090, 0.070, 0.048, 1.0);
    pub const PANEL_FRAME: Color  = Color::new(0.55, 0.42, 0.20, 1.0);
    pub const PANEL_INNER: Color  = Color::new(0.32, 0.24, 0.12, 1.0);

    // Text.
    pub const GOLD: Color        = Color::new(0.94, 0.78, 0.40, 1.0);
    pub const GOLD_SOFT: Color   = Color::new(0.80, 0.65, 0.32, 1.0);
    pub const IVORY: Color       = Color::new(0.96, 0.92, 0.82, 1.0);
    pub const SEPIA: Color       = Color::new(0.60, 0.51, 0.38, 1.0);
    pub const SEPIA_DIM: Color   = Color::new(0.42, 0.35, 0.25, 1.0);
    pub const CINNABAR: Color    = Color::new(0.86, 0.32, 0.20, 1.0);
}

/// Action emitted by the menu when the player clicks something.
pub enum MenuAction {
    StartLevel(usize),
    Quit,
    /// Debug only: jump straight to the epilogue cinematic.
    DebugEpilogue,
    /// Debug only: jump straight to the Halloween teaser.
    DebugHalloween,
}

pub struct Menu {
    pub chapters: Vec<ChapterConfig>,
    pub levels: Vec<LevelConfig>,
    pub progress: Progress,

    hovered: Option<usize>,
    scroll_y: f32,
    max_scroll: f32,
}

impl Menu {
    pub fn new(
        chapters: Vec<ChapterConfig>,
        levels: Vec<LevelConfig>,
        progress: Progress,
    ) -> Self {
        Self {
            chapters,
            levels,
            progress,
            hovered: None,
            scroll_y: 0.0,
            max_scroll: 0.0,
        }
    }

    // ---------- Public API ----------

    pub fn tick(
        &mut self,
        _dt: f32,
        mouse: Vec2,
        click: bool,
    ) -> Option<MenuAction> {
        let vw = screen_width();
        let vh = screen_height();

        let content_h = self.total_height();
        let visible_h =
            (vh - FOOTER_H - CONTENT_BOTTOM_RESERVE - CONTENT_TOP).max(1.0);
        self.max_scroll = (content_h - visible_h).max(0.0);

        let (_, wy) = mouse_wheel();
        if wy.abs() > 0.0 {
            self.scroll_y =
                (self.scroll_y - wy * 40.0).clamp(0.0, self.max_scroll);
        }
        let step = 500.0 * get_frame_time();
        if is_key_down(KeyCode::Down) {
            self.scroll_y = (self.scroll_y + step).min(self.max_scroll);
        }
        if is_key_down(KeyCode::Up) {
            self.scroll_y = (self.scroll_y - step).max(0.0);
        }

        let base_y = CONTENT_TOP - self.scroll_y;
        self.hovered = None;
        let bottom_limit = vh - FOOTER_H - CONTENT_BOTTOM_RESERVE;

        for ci in 0..self.chapters.len() {
            let ch_y = self.chapter_top_y(ci, base_y);
            let chapter_levels = self.levels_of_chapter(ci);
            for (row, &li) in chapter_levels.iter().enumerate() {
                let r = level_rect(row, vw, ch_y);
                if r.y + r.h < CONTENT_TOP || r.y > bottom_limit {
                    continue;
                }
                if rect_contains(r, mouse.x, mouse.y) {
                    self.hovered = Some(li);
                }
            }
        }

        // Debug-only buttons (visible only while Ctrl is held).
        #[cfg(debug_assertions)]
        {
            let debug_visible = is_key_down(KeyCode::LeftControl)
                || is_key_down(KeyCode::RightControl);
            if debug_visible {
                let scare = debug_scare_button_rect(vw, vh);
                if click && rect_contains(scare, mouse.x, mouse.y) {
                    return Some(MenuAction::DebugHalloween);
                }
                let epi = debug_epilogue_button_rect(vw, vh);
                if click && rect_contains(epi, mouse.x, mouse.y) {
                    return Some(MenuAction::DebugEpilogue);
                }
            }
        }

        if click
            && mouse.y >= CONTENT_TOP
            && mouse.y <= bottom_limit
            && !rect_contains(info_panel_rect(vw, vh), mouse.x, mouse.y)
            && let Some(idx) = self.hovered
            && self.is_level_unlocked(idx)
        {
            return Some(MenuAction::StartLevel(idx));
        }

        None
    }

    /// Render order matters:
    ///   1. Background gradient (covers everything)
    ///   2. Scrollable content (may bleed over header/footer)
    ///   3. Info panel (right side, within content region)
    ///   4. Header bar (opaque, drawn LAST to cover any bleed)
    ///   5. Footer bar (opaque, drawn LAST to cover any bleed)
    pub fn draw(&self, _ctx: &GameContext, _poem_font: &PoemFont) {
        let vw = screen_width();
        let vh = screen_height();

        draw_background(vw, vh);

        let base_y = CONTENT_TOP - self.scroll_y;
        self.draw_chapters(vw, base_y);
        self.draw_info_panel(vw, vh);

        // Opaque header & footer drawn LAST so nothing bleeds.
        self.draw_header(vw);
        self.draw_footer(vw, vh);
    }

    // ---------- Unlock logic ----------

    pub fn is_level_unlocked(&self, level_idx: usize) -> bool {
        if level_idx >= self.levels.len() {
            return false;
        }
        if level_idx == 0 {
            return true;
        }
        (0..level_idx)
            .all(|i| self.progress.level_completed(&self.levels[i].id))
    }

    pub fn is_chapter_unlocked(&self, chapter_idx: usize) -> bool {
        if chapter_idx == 0 {
            return true;
        }
        let prev = self.levels_of_chapter(chapter_idx - 1);
        !prev.is_empty()
            && prev
                .iter()
                .all(|&i| self.progress.level_completed(&self.levels[i].id))
    }

    // ---------- Layout ----------

    fn levels_of_chapter(&self, chapter_idx: usize) -> Vec<usize> {
        let chapter_id = match self.chapters.get(chapter_idx) {
            Some(c) => c.id,
            None => return Vec::new(),
        };
        self.levels
            .iter()
            .enumerate()
            .filter(|(_, l)| l.chapter == chapter_id)
            .map(|(i, _)| i)
            .collect()
    }

    fn chapter_top_y(&self, chapter_idx: usize, base_y: f32) -> f32 {
        let mut y = base_y;
        for ci in 0..chapter_idx {
            let count = self.levels_of_chapter(ci).len();
            y += CHAPTER_HEADER_H
                + count as f32 * (LEVEL_H + LEVEL_GAP)
                + CHAPTER_GAP;
        }
        y
    }

    fn total_height(&self) -> f32 {
        let mut h = 0.0;
        for ci in 0..self.chapters.len() {
            let count = self.levels_of_chapter(ci).len();
            h += CHAPTER_HEADER_H
                + count as f32 * (LEVEL_H + LEVEL_GAP)
                + CHAPTER_GAP;
        }
        h
    }

    // ---------- Drawing ----------

    fn draw_chapters(&self, vw: f32, base_y: f32) {
        for ci in 0..self.chapters.len() {
            let ch_y = self.chapter_top_y(ci, base_y);
            let chapter_levels = self.levels_of_chapter(ci);
            self.draw_chapter_header(ci, ch_y, vw);

            for (row, &li) in chapter_levels.iter().enumerate() {
                let r = level_rect(row, vw, ch_y);
                let level = &self.levels[li];
                let unlocked = self.is_level_unlocked(li);
                let stars = self.progress.level_stars(&level.id);
                let best = self.progress.level_best_score(&level.id);
                let hovered = self.hovered == Some(li) && unlocked;
                self.draw_level_card(level, r, stars, best, unlocked, hovered, li);
            }
        }
    }

    fn draw_chapter_header(&self, chapter_idx: usize, y: f32, vw: f32) {
        use palette::*;

        let cx = content_origin_x(vw);
        let lw = level_w(vw);
        let chapter = &self.chapters[chapter_idx];
        let unlocked = self.is_chapter_unlocked(chapter_idx);

        // "CHAPTER N" eyebrow.
        draw_text(
            format!("CHAPTER {}", chapter_idx + 1),
            cx,
            y + 22.0,
            11.0,
            if unlocked { SEPIA } else { SEPIA_DIM },
        );

        // Chapter name — large and proud.
        draw_text(
            &chapter.name,
            cx,
            y + 66.0,
            30.0,
            if unlocked { GOLD } else { SEPIA_DIM },
        );

        // Subtitle below.
        draw_text(
            &chapter.subtitle,
            cx,
            y + 98.0,
            12.0,
            if unlocked { SEPIA } else { SEPIA_DIM },
        );

        // Separator with a small gold diamond at the left.
        let sep_y = y + 122.0;
        draw_line(cx + 12.0, sep_y, cx + lw, sep_y, 1.0, PANEL_INNER);
        if unlocked {
            draw_rectangle(cx - 3.0, sep_y - 3.0, 6.0, 6.0, GOLD);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_level_card(
        &self,
        level: &LevelConfig,
        r: Rect,
        stars: u8,
        best: u32,
        unlocked: bool,
        hovered: bool,
        global_idx: usize,
    ) {
        use palette::*;

        let pulse = if hovered {
            0.72 + 0.28 * (get_time() as f32 * 3.5).sin()
        } else {
            1.0
        };

        let (bg, border, accent) = if !unlocked {
            (
                CARD_LOCKED_BG,
                CARD_LOCKED_BORDER,
                CARD_LOCKED_BORDER,
            )
        } else if hovered {
            (
                CARD_BG_HOVER,
                Color::new(GOLD.r, GOLD.g, GOLD.b, pulse),
                GOLD,
            )
        } else if stars > 0 {
            (CARD_BG, Color::new(0.55, 0.42, 0.20, 1.0), GOLD_SOFT)
        } else {
            (CARD_BG, CARD_BORDER, SEPIA_DIM)
        };

        draw_rectangle(r.x, r.y, r.w, r.h, bg);
        draw_rectangle(r.x, r.y, 3.0, r.h, accent);
        draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, border);

        // Level index "01".
        draw_text(
            format!("{:02}", global_idx + 1),
            r.x + 20.0,
            r.y + 36.0,
            13.0,
            if unlocked { GOLD_SOFT } else { SEPIA_DIM },
        );

        // Level name.
        draw_text(
            &level.name,
            r.x + 60.0,
            r.y + 38.0,
            20.0,
            if unlocked { IVORY } else { SEPIA_DIM },
        );

        // Meta line (objective · moves).
        let meta = format!(
            "{}  ·  {} moves",
            objective_label(level.objective),
            level.moves,
        );
        draw_text(
            meta,
            r.x + 60.0,
            r.y + 64.0,
            13.0,
            if unlocked { SEPIA } else { SEPIA_DIM },
        );

        // Stars + thresholds, top-right.
        draw_star_rating_with_thresholds(r, stars, unlocked, level);

        // Best score, bottom-right.
        if unlocked && best > 0 {
            let txt = format!("BEST {}", best);
            let dim = measure_text(&txt, None, 12, 1.0);
            draw_text(
                txt,
                r.x + r.w - 24.0 - dim.width,
                r.y + r.h - 14.0,
                12.0,
                GOLD_SOFT,
            );
        }
    }

    fn draw_info_panel(&self, vw: f32, vh: f32) {
        use palette::*;

        let panel = info_panel_rect(vw, vh);

        // Panel background + double gold frame (traditional mount).
        draw_rectangle(panel.x, panel.y, panel.w, panel.h, PANEL_BG);
        draw_rectangle_lines(panel.x, panel.y, panel.w, panel.h, 1.5, PANEL_FRAME);
        draw_rectangle_lines(
            panel.x + 5.0,
            panel.y + 5.0,
            panel.w - 10.0,
            panel.h - 10.0,
            1.0,
            PANEL_INNER,
        );

        let x = panel.x + INFO_PANEL_PAD;
        let w = panel.w - INFO_PANEL_PAD * 2.0;

        let level_idx = match self.hovered {
            Some(i) => i,
            None => {
                draw_text("Hover a mission", x, panel.y + 44.0, 14.0, SEPIA);
                return;
            }
        };

        let level = &self.levels[level_idx];
        let chapter = &self.chapters[level.chapter as usize];
        let unlocked = self.is_level_unlocked(level_idx);
        let stars = self.progress.level_stars(&level.id);
        let best = self.progress.level_best_score(&level.id);

        let mut y = panel.y + 38.0;

        // Eyebrow.
        draw_text(
            format!("CHAPTER {} — {}", level.chapter + 1, chapter.name),
            x,
            y,
            11.0,
            SEPIA,
        );
        y += 32.0;

        // Level name — big, ivory.
        draw_text(&level.name, x, y, 22.0, IVORY);
        y += 22.0;
        draw_line(x, y, x + w, y, 1.0, PANEL_INNER);
        y += 28.0;

        // Section: Objective.
        draw_section(x, y, "OBJECTIVE", &objective_label(level.objective));
        y += 62.0;

        // Section: Moves.
        draw_section(x, y, "MOVES", &format!("{}", level.moves));
        y += 62.0;

        draw_line(x, y, x + w, y, 1.0, PANEL_INNER);
        y += 28.0;

        // Section: Stars — one line per tier.
        //
        // Tier 1 shows "obj" for ClearJade / FillPoem levels, because
        // the actual requirement is completing the objective, not
        // reaching the numeric star_target (which only gates tiers 2
        // and 3). Tiers 2 and 3 are always score thresholds.
        draw_text("STARS", x, y, 10.0, SEPIA);
        y += 26.0;

        for tier in 1..=3u8 {
            let is_achieved = unlocked && stars >= tier;

            // "★", "★★", "★★★"
            let star_str: String = (0..tier).map(|_| '★').collect();

            let (star_color, thr_color) = if !unlocked {
                (SEPIA_DIM, SEPIA_DIM)
            } else if is_achieved {
                (GOLD, IVORY)
            } else {
                (SEPIA, SEPIA)
            };

            let label = star_tier_label(level, tier);

            draw_text(&star_str, x, y, 16.0, star_color);
            draw_text(&label, x + 62.0, y, 14.0, thr_color);
            if is_achieved {
                draw_text("✓", x + 130.0, y, 14.0, GOLD);
            }
            y += 22.0;
        }
        y += 8.0;

        // Best score + its time.
        if unlocked && best > 0 {
            draw_text("BEST SCORE", x, y, 10.0, SEPIA);
            y += 20.0;
            let score_line = if let Some(t) = self.progress.best_time_for_score(&level.id) {
                format!("{best}   ·   {}", fmt_time(t))
            } else {
                format!("{best}")
            };
            draw_text(&score_line, x, y, 16.0, GOLD);
            y += 26.0;

            // Star times, if we have any.
            if let Some(times) = self.progress.level_times(&level.id) {
                let has_any = times.time_1_star.is_some()
                    || times.time_2_star.is_some()
                    || times.time_3_star.is_some();
                if has_any {
                    draw_text("BEST TIMES", x, y, 10.0, SEPIA);
                    y += 18.0;
                    if let Some(s) = times.time_1_star {
                        draw_text(format!("★    {}", fmt_time(s)), x, y, 13.0, GOLD_SOFT);
                        y += 16.0;
                    }
                    if let Some(s) = times.time_2_star {
                        draw_text(format!("★★   {}", fmt_time(s)), x, y, 13.0, GOLD_SOFT);
                        y += 16.0;
                    }
                    if let Some(s) = times.time_3_star {
                        draw_text(format!("★★★  {}", fmt_time(s)), x, y, 13.0, GOLD_SOFT);
                    }
                }
            }
        }

        if !unlocked {
            draw_text(
                "Complete the previous mission",
                x,
                panel.y + panel.h - 28.0,
                12.0,
                CINNABAR,
            );
        }
    }

    fn draw_header(&self, vw: f32) {
        use palette::*;

        // Opaque bar.
        draw_rectangle(0.0, 0.0, vw, HEADER_H, BAR_BG);
        draw_line(0.0, HEADER_H, vw, HEADER_H, 1.5, BAR_BORDER);

        let cx = content_origin_x(vw);

        // Title + underline.
        draw_text("JADE GARDEN", cx, 50.0, 30.0, GOLD);
        let dim_title = measure_text("JADE GARDEN", None, 30, 1.0);
        draw_line(
            cx,
            58.0,
            cx + dim_title.width,
            58.0,
            1.0,
            GOLD_SOFT,
        );
        draw_text("CAMPAIGN", cx, 76.0, 11.0, SEPIA);

        // Right side: poem progress.
        let poem_txt = format!(
            "POEM  {} / {}",
            self.progress.poem_fragments,
            crate::poem::TOTAL_CHARS
        );
        let dim_p = measure_text(&poem_txt, None, 15, 1.0);
        draw_text(
            poem_txt,
            vw - PAD_X - dim_p.width,
            46.0,
            15.0,
            IVORY,
        );

        let stars_txt = format!("★ {} earned", self.progress.total_stars());
        let dim_s = measure_text(&stars_txt, None, 12, 1.0);
        draw_text(
            stars_txt,
            vw - PAD_X - dim_s.width,
            70.0,
            12.0,
            GOLD_SOFT,
        );
    }

    fn draw_footer(&self, vw: f32, vh: f32) {
        use palette::*;

        // Opaque bar.
        draw_rectangle(0.0, vh - FOOTER_H, vw, FOOTER_H, BAR_BG);
        draw_line(0.0, vh - FOOTER_H, vw, vh - FOOTER_H, 1.5, BAR_BORDER);

        let hint =
            "Click a mission   |   Scroll ↑↓ or wheel   |   [Tab] Help   |   [Esc] Quit";
        let dim = measure_text(hint, None, 14, 1.0);
        draw_text(
            hint,
            vw * 0.5 - dim.width * 0.5,
            vh - 16.0,
            14.0,
            SEPIA,
        );

        #[cfg(debug_assertions)]
        {
            let debug_visible = is_key_down(KeyCode::LeftControl)
                || is_key_down(KeyCode::RightControl);
            if debug_visible {
                use palette::*;
                let mouse = Vec2::new(mouse_position().0, mouse_position().1);

                // Epilogue button.
                let r1 = debug_epilogue_button_rect(vw, vh);
                let h1 = rect_contains(r1, mouse.x, mouse.y);
                let bg1 = if h1 {
                    Color::new(0.32, 0.20, 0.10, 1.0)
                } else {
                    Color::new(0.18, 0.11, 0.06, 1.0)
                };
                draw_rectangle(r1.x, r1.y, r1.w, r1.h, bg1);
                draw_rectangle_lines(r1.x, r1.y, r1.w, r1.h, 1.0, GOLD_SOFT);
                let t1 = "▶ Epilogue";
                let d1 = measure_text(t1, None, 11, 1.0);
                draw_text(
                    t1,
                    r1.x + (r1.w - d1.width) * 0.5,
                    r1.y + 15.0,
                    11.0,
                    Color::new(1.0, 0.85, 0.60, 1.0),
                );

                // Scare button.
                let r2 = debug_scare_button_rect(vw, vh);
                let h2 = rect_contains(r2, mouse.x, mouse.y);
                let bg2 = if h2 {
                    Color::new(0.50, 0.10, 0.08, 1.0)
                } else {
                    Color::new(0.28, 0.06, 0.05, 1.0)
                };
                draw_rectangle(r2.x, r2.y, r2.w, r2.h, bg2);
                draw_rectangle_lines(r2.x, r2.y, r2.w, r2.h, 1.0, CINNABAR);
                let t2 = "☠ Scare";
                let d2 = measure_text(t2, None, 11, 1.0);
                draw_text(
                    t2,
                    r2.x + (r2.w - d2.width) * 0.5,
                    r2.y + 15.0,
                    11.0,
                    Color::new(1.0, 0.80, 0.55, 1.0),
                );
            }
        }
    }
}

// ---------- Free functions ----------

/// Draw the vertical gradient background (covers the entire viewport).
fn draw_background(vw: f32, vh: f32) {
    use palette::*;
    const STRIPS: usize = 32;
    let strip_h = vh / STRIPS as f32;
    for i in 0..STRIPS {
        let t = i as f32 / (STRIPS - 1) as f32;
        let c = Color::new(
            BG_TOP.r * (1.0 - t) + BG_BOTTOM.r * t,
            BG_TOP.g * (1.0 - t) + BG_BOTTOM.g * t,
            BG_TOP.b * (1.0 - t) + BG_BOTTOM.b * t,
            1.0,
        );
        draw_rectangle(0.0, i as f32 * strip_h, vw, strip_h + 1.0, c);
    }
}

fn draw_section(x: f32, y: f32, label: &str, value: &str) {
    use palette::*;
    draw_text(label, x, y, 10.0, SEPIA);
    draw_text(value, x, y + 22.0, 15.0, IVORY);
}

fn rect_contains(r: Rect, x: f32, y: f32) -> bool {
    x >= r.x && x <= r.x + r.w && y >= r.y && y <= r.y + r.h
}

#[cfg(test)]
fn star_string(stars: u8, unlocked: bool) -> String {
    let mut s = String::with_capacity(3);
    let filled = if unlocked { stars.min(3) } else { 0 };
    for _ in 0..filled {
        s.push('★');
    }
    for _ in filled..3 {
        s.push('☆');
    }
    s
}

/// Formats a duration in seconds as `M:SS`.
fn fmt_time(secs: f32) -> String {
    let total = secs.round() as u32;
    format!("{}:{:02}", total / 60, total % 60)
}

/// Rect of the debug-only "Halloween teaser" button in the footer.
#[cfg(debug_assertions)]
fn debug_scare_button_rect(vw: f32, vh: f32) -> Rect {
    let w = 110.0;
    let h = 22.0;
    let x = vw - PAD_X - w;
    let y = vh - FOOTER_H + (FOOTER_H - h) * 0.5 - 30.0; // Positioned above the epilogue button
    Rect::new(x, y, w, h)
}

/// Rect of the debug-only "Epilogue" button in the footer.
#[cfg(debug_assertions)]
fn debug_epilogue_button_rect(vw: f32, vh: f32) -> Rect {
    let w = 110.0;
    let h = 22.0;
    let x = vw - PAD_X - w;
    let y = vh - FOOTER_H + (FOOTER_H - h) * 0.5;
    Rect::new(x, y, w, h)
}

/// Draws 3 stars right-aligned inside `r`, with a per-tier label
/// under each star. The label is either a numeric score threshold
/// (for Score levels, and for tiers 2-3 of every level) or "obj" for
/// tier 1 of ClearJade / FillPoem levels.
fn draw_star_rating_with_thresholds(
    r: Rect,
    stars: u8,
    unlocked: bool,
    level: &LevelConfig,
) {
    use palette::*;

    let star_y = r.y + 30.0;
    let star_size = 24.0;
    let gap = 10.0;
    let slot_w = star_size + gap;
    let total_w = 3.0 * star_size + 2.0 * gap;
    let start_x = r.x + r.w - 22.0 - total_w;

    let t = get_time() as f32;

    for i in 0..3u8 {
        let tier = i + 1;
        let filled = unlocked && i < stars;
        let x = start_x + i as f32 * slot_w;
        let glyph = if filled { "★" } else { "☆" };

        let color = if !unlocked {
            SEPIA_DIM
        } else if filled {
            GOLD
        } else {
            SEPIA
        };

        let scale = if filled {
            1.0 + 0.05 * (t * 2.2 + i as f32 * 0.7).sin()
        } else {
            1.0
        };
        let size = star_size * scale;
        let y_off = (star_size - size) * 0.5;

        if filled {
            let glow = Color::new(GOLD.r, GOLD.g, GOLD.b, 0.25);
            draw_text(glyph, x - 2.0, star_y + y_off - 1.5, size + 4.0, glow);
        }
        draw_text(glyph, x, star_y + y_off, size, color);

        // Tier label below the star.
        let label = star_tier_label(level, tier);
        let dim = measure_text(&label, None, 10, 1.0);
        let tx = x + (star_size - dim.width) * 0.5;
        let tc = if !unlocked {
            SEPIA_DIM
        } else if filled {
            GOLD_SOFT
        } else {
            SEPIA
        };
        draw_text(&label, tx, star_y + 16.0, 10.0, tc);
    }
}

// ---------- Layout constants ----------

const HEADER_H: f32 = 96.0;
const FOOTER_H: f32 = 44.0;
const PAD_X: f32 = 64.0;
const PAD_TOP: f32 = 32.0;

const LEVEL_H: f32 = 88.0;
const LEVEL_GAP: f32 = 10.0;

const CHAPTER_HEADER_H: f32 = 140.0;
const CHAPTER_GAP: f32 = 56.0;

const INFO_PANEL_GAP: f32 = 40.0;
const INFO_PANEL_PAD: f32 = 30.0;

pub const CONTENT_TOP: f32 = HEADER_H + PAD_TOP;
pub const CONTENT_BOTTOM_RESERVE: f32 = 24.0;

// ---------- Layout helpers ----------

pub fn info_panel_w(vw: f32) -> f32 {
    (vw * 0.30).clamp(300.0, 420.0)
}

pub fn level_w(vw: f32) -> f32 {
    let reserved = PAD_X * 2.0 + INFO_PANEL_GAP + info_panel_w(vw);
    (vw - reserved).clamp(300.0, 900.0)
}

pub fn content_origin_x(vw: f32) -> f32 {
    let total = level_w(vw) + INFO_PANEL_GAP + info_panel_w(vw);
    ((vw - total) * 0.5).max(PAD_X)
}

pub fn level_rect(row: usize, vw: f32, chapter_top_y: f32) -> Rect {
    let w = level_w(vw);
    let x = content_origin_x(vw);
    let y = chapter_top_y + CHAPTER_HEADER_H + row as f32 * (LEVEL_H + LEVEL_GAP);
    Rect::new(x, y, w, LEVEL_H)
}

pub fn info_panel_rect(vw: f32, vh: f32) -> Rect {
    let w = info_panel_w(vw);
    let x = content_origin_x(vw) + level_w(vw) + INFO_PANEL_GAP;
    let y = CONTENT_TOP;
    let h = vh - CONTENT_TOP - FOOTER_H - CONTENT_BOTTOM_RESERVE;
    Rect::new(x, y, w, h)
}

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::Jade;

    fn chapter(id: u32, name: &str) -> ChapterConfig {
        ChapterConfig {
            id,
            name: name.into(),
            subtitle: format!("Chapter {id}"),
            intro: String::new(),
        }
    }

    fn level(id: &str, chapter: u32, seed: u32) -> LevelConfig {
        LevelConfig {
            id: id.into(),
            name: format!("Level {id}"),
            chapter,
            moves: 20,
            objective: crate::level::Objective::Score(1000),
            star_target: 1000,
            poem_reveal: 0,
            seed,
        }
    }

    fn sample_menu() -> Menu {
        let chapters = vec![chapter(0, "C0"), chapter(1, "C1")];
        let levels = vec![
            level("a", 0, 1),
            level("b", 0, 2),
            level("c", 1, 3),
        ];
        Menu::new(chapters, levels, Progress::default())
    }

    #[test]
    fn levels_rect_stack_vertically() {
        let a = level_rect(0, 1200.0, 100.0);
        let b = level_rect(1, 1200.0, 100.0);
        assert!(b.y > a.y + a.h);
    }

    #[test]
    fn level_rect_starts_below_chapter_header() {
        let r = level_rect(0, 1200.0, 100.0);
        assert!(r.y >= 100.0 + CHAPTER_HEADER_H - 0.5);
    }

    #[test]
    fn level_rect_height_is_level_h() {
        let r = level_rect(0, 1200.0, 100.0);
        assert!((r.h - LEVEL_H).abs() < 1e-6);
    }

    #[test]
    fn info_panel_sits_right_of_levels() {
        let vw = 1200.0;
        let r = level_rect(0, vw, 100.0);
        let panel = info_panel_rect(vw, 900.0);
        assert!(panel.x >= r.x + r.w - 0.5);
    }

    #[test]
    fn info_panel_is_within_viewport() {
        for vw in [1200.0, 1440.0, 1920.0] {
            let panel = info_panel_rect(vw, 900.0);
            assert!(panel.x + panel.w <= vw - PAD_X + 1.0);
            assert!(panel.x >= 0.0);
        }
    }

    #[test]
    fn level_w_has_a_floor_and_ceiling() {
        assert!(level_w(400.0) >= 300.0);
        assert!((level_w(4000.0) - 900.0).abs() < 1e-6);
    }

    #[test]
    fn info_panel_w_is_bounded() {
        assert!((info_panel_w(800.0) - 300.0).abs() < 1e-6);
        assert!((info_panel_w(3000.0) - 420.0).abs() < 1e-6);
    }

    #[test]
    fn content_is_centered_on_wide_screens() {
        let vw = 1920.0;
        let cx = content_origin_x(vw);
        let total = level_w(vw) + INFO_PANEL_GAP + info_panel_w(vw);
        let right = vw - (cx + total);
        assert!((cx - right).abs() < 1.0);
    }

    #[test]
    fn content_is_pinned_on_narrow_screens() {
        assert!((content_origin_x(800.0) - PAD_X).abs() < 1e-6);
    }

    #[test]
    fn chapter_top_y_increases_per_chapter() {
        let m = sample_menu();
        assert!(m.chapter_top_y(1, 100.0) > m.chapter_top_y(0, 100.0));
    }

    #[test]
    fn chapter_top_y_offsets_by_scroll() {
        let m = sample_menu();
        let a = m.chapter_top_y(1, 100.0);
        let b = m.chapter_top_y(1, 60.0);
        assert!((b - (a - 40.0)).abs() < 1e-3);
    }

    #[test]
    fn total_height_is_positive() {
        assert!(sample_menu().total_height() > 0.0);
    }

    #[test]
    fn first_level_unlocked_by_default() {
        assert!(sample_menu().is_level_unlocked(0));
    }

    #[test]
    fn later_level_locked_until_previous_completed() {
        let mut m = sample_menu();
        assert!(!m.is_level_unlocked(1));
        m.progress.record_win("a", 1200, 1, 0);
        assert!(m.is_level_unlocked(1));
        assert!(!m.is_level_unlocked(2));
    }

    #[test]
    fn chapter_zero_always_unlocked() {
        assert!(sample_menu().is_chapter_unlocked(0));
    }

    #[test]
    fn chapter_one_locked_until_chapter_zero_complete() {
        let mut m = sample_menu();
        assert!(!m.is_chapter_unlocked(1));
        m.progress.record_win("a", 1200, 1, 0);
        assert!(!m.is_chapter_unlocked(1));
        m.progress.record_win("b", 1200, 1, 0);
        assert!(m.is_chapter_unlocked(1));
    }

    #[test]
    fn out_of_range_level_is_locked() {
        assert!(!sample_menu().is_level_unlocked(999));
    }

    #[test]
    fn star_string_reflects_lock_and_stars() {
        assert_eq!(star_string(0, false), "☆☆☆");
        assert_eq!(star_string(0, true), "☆☆☆");
        assert_eq!(star_string(1, true), "★☆☆");
        assert_eq!(star_string(2, true), "★★☆");
        assert_eq!(star_string(3, true), "★★★");
        assert_eq!(star_string(3, false), "☆☆☆");
    }

    #[test]
    fn star_string_caps_above_three() {
        assert_eq!(star_string(5, true), "★★★");
        assert_eq!(star_string(255, true), "★★★");
    }

    #[test]
    fn objective_label_is_english() {
        let l = crate::level::Objective::ClearJade(Jade::Bi, 15);
        assert!(objective_label(l).starts_with("Clear "));
    }

    #[test]
    fn menu_new_preserves_arguments() {
        let m = sample_menu();
        assert_eq!(m.chapters.len(), 2);
        assert_eq!(m.levels.len(), 3);
    }

    // ---------- Footer-overlap invariant ----------

    #[test]
    fn footer_top_is_below_content_area() {
        // Any level card in the content area ends strictly above the
        // footer. The scroll view already clamps via `max_scroll`, so
        // this test guards the constants themselves.
        let vh = 900.0;
        let footer_top = vh - FOOTER_H;
        assert!(footer_top > CONTENT_TOP);
        // Enough vertical space for at least one card between
        // CONTENT_TOP and footer_top.
        assert!(footer_top - CONTENT_TOP > LEVEL_H);
    }
}