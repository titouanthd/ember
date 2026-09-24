//! In-game help overlay — full rules for Zhuo Ji.
//!
//! Seven tabs: Goal / Patterns / Dou / Ji / Tiles / Specials / UI.

use std::path::PathBuf;

use macroquad::prelude::*;

use crate::components::{Suit, Tile};
use crate::config::GameContext;
use crate::layout::{TableLayout, TileSize};
use crate::tiles;

const PANEL_W: f32 = 1040.0;
const PANEL_H: f32 = 780.0;
const TAB_H: f32 = 42.0;
const CONTENT_TOP: f32 = 150.0;
const BTN_H: f32 = 46.0;
const BTN_W: f32 = 260.0;

const LINE: f32 = 24.0;

// ─── Tabs ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HelpTab {
    #[default]
    Goal,
    Patterns,
    Dou,
    Ji,
    Tiles,
    Specials,
    Interface,
}

impl HelpTab {
    pub const ALL: [HelpTab; 7] = [
        HelpTab::Goal,
        HelpTab::Patterns,
        HelpTab::Dou,
        HelpTab::Ji,
        HelpTab::Tiles,
        HelpTab::Specials,
        HelpTab::Interface,
    ];

    pub fn label(self) -> &'static str {
        match self {
            HelpTab::Goal => "Goal",
            HelpTab::Patterns => "Patterns",
            HelpTab::Dou => "Dou",
            HelpTab::Ji => "Ji",
            HelpTab::Tiles => "Tiles",
            HelpTab::Specials => "Specials",
            HelpTab::Interface => "UI",
        }
    }

    pub fn idx(self) -> usize {
        self as usize
    }

    pub fn from_idx(i: usize) -> Self {
        match i {
            0 => HelpTab::Goal,
            1 => HelpTab::Patterns,
            2 => HelpTab::Dou,
            3 => HelpTab::Ji,
            4 => HelpTab::Tiles,
            5 => HelpTab::Specials,
            6 => HelpTab::Interface,
            _ => HelpTab::Goal,
        }
    }

    pub fn next(self) -> Self {
        Self::from_idx((self.idx() + 1) % Self::ALL.len())
    }

    pub fn prev(self) -> Self {
        Self::from_idx((self.idx() + Self::ALL.len() - 1) % Self::ALL.len())
    }
}

// ─── State ───────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct HelpState {
    pub visible: bool,
    pub has_seen: bool,
    pub tab: HelpTab,
    pub suit_filter: Option<Suit>,
    pub rank_filter: Option<u8>,
}

impl HelpState {
    pub fn new() -> Self {
        let has_seen = load_seen();
        Self {
            visible: !has_seen,
            has_seen,
            tab: HelpTab::Goal,
            suit_filter: None,
            rank_filter: None,
        }
    }

    pub fn open(&mut self) {
        self.visible = true;
    }

    pub fn close(&mut self) {
        self.visible = false;
        if !self.has_seen {
            self.has_seen = true;
            save_seen();
        }
    }

    pub fn toggle(&mut self) {
        if self.visible {
            self.close();
        } else {
            self.open();
        }
    }

    pub fn next_tab(&mut self) {
        self.tab = self.tab.next();
    }

    pub fn prev_tab(&mut self) {
        self.tab = self.tab.prev();
    }
}

// ─── Persistence ─────────────────────────────────────────────────────────

fn prefs_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    Some(exe.parent()?.join(".zhuoji_prefs"))
}

fn load_seen() -> bool {
    prefs_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|s| s.trim() == "seen")
        .unwrap_or(false)
}

fn save_seen() {
    if let Some(p) = prefs_path() {
        let _ = std::fs::write(p, "seen");
    }
}

// ─── Geometry ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub struct HelpGeom {
    pub px: f32,
    pub py: f32,
    pub pw: f32,
    pub ph: f32,
    pub content_x: f32,
    pub content_y: f32,
    pub content_w: f32,
    pub content_h: f32,
}

impl HelpGeom {
    pub fn new(layout: &TableLayout) -> Self {
        let pw = PANEL_W.min(layout.scr_w - 60.0);
        let ph = PANEL_H.min(layout.scr_h - 60.0);
        let px = (layout.scr_w - pw) * 0.5;
        let py = (layout.scr_h - ph) * 0.5;
        let content_x = px + 44.0;
        let content_y = py + CONTENT_TOP;
        let content_w = pw - 88.0;
        let content_h = ph - CONTENT_TOP - BTN_H - 40.0;
        Self {
            px,
            py,
            pw,
            ph,
            content_x,
            content_y,
            content_w,
            content_h,
        }
    }
}

fn tab_rect(g: &HelpGeom, i: usize) -> (f32, f32, f32, f32) {
    let n = HelpTab::ALL.len();
    let total_w = g.pw - 80.0;
    let gap = 8.0;
    let each = (total_w - gap * (n as f32 - 1.0)) / n as f32;
    let x = g.px + 40.0 + i as f32 * (each + gap);
    let y = g.py + 74.0;
    (x, y, each, TAB_H)
}

fn close_btn_rect(g: &HelpGeom) -> (f32, f32, f32, f32) {
    let s = 34.0;
    (g.px + g.pw - s - 20.0, g.py + 18.0, s, s)
}

fn got_it_btn_rect(g: &HelpGeom) -> (f32, f32, f32, f32) {
    let x = g.px + (g.pw - BTN_W) * 0.5;
    let y = g.py + g.ph - BTN_H - 22.0;
    (x, y, BTN_W, BTN_H)
}

// Filter geometry for the Tiles tab.

const SUIT_BTN_W: f32 = 70.0;
const SUIT_BTN_H: f32 = 32.0;
const SUIT_BTN_GAP: f32 = 8.0;

const RANK_BTN_W: f32 = 40.0;
const RANK_BTN_H: f32 = 32.0;
const RANK_BTN_GAP: f32 = 6.0;

fn suit_btn_rect(g: &HelpGeom, i: usize) -> (f32, f32, f32, f32) {
    let x = g.content_x + 80.0 + i as f32 * (SUIT_BTN_W + SUIT_BTN_GAP);
    let y = g.content_y + 24.0;
    (x, y, SUIT_BTN_W, SUIT_BTN_H)
}

fn rank_btn_rect(g: &HelpGeom, i: usize) -> (f32, f32, f32, f32) {
    let x = g.content_x + 80.0 + i as f32 * (RANK_BTN_W + RANK_BTN_GAP);
    let y = g.content_y + 68.0;
    (x, y, RANK_BTN_W, RANK_BTN_H)
}

// ─── Hit testing ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelpAction {
    None,
    Close,
    GotIt,
    Tab(HelpTab),
    SuitFilter(Option<Suit>),
    RankFilter(Option<u8>),
}

fn rect_contains(x: f32, y: f32, w: f32, h: f32, p: Vec2) -> bool {
    p.x >= x && p.x <= x + w && p.y >= y && p.y <= y + h
}

pub fn hit_test(layout: &TableLayout, state: &HelpState, mouse: Vec2) -> HelpAction {
    let g = HelpGeom::new(layout);
    let (cx, cy, cw, ch) = close_btn_rect(&g);
    if rect_contains(cx, cy, cw, ch, mouse) {
        return HelpAction::Close;
    }
    let (gx, gy, gw, gh) = got_it_btn_rect(&g);
    if rect_contains(gx, gy, gw, gh, mouse) {
        return HelpAction::GotIt;
    }
    for (i, tab) in HelpTab::ALL.iter().enumerate() {
        let (tx, ty, tw, th) = tab_rect(&g, i);
        if rect_contains(tx, ty, tw, th, mouse) {
            return HelpAction::Tab(*tab);
        }
    }

    if state.tab == HelpTab::Tiles {
        for i in 0..4 {
            let (x, y, w, h) = suit_btn_rect(&g, i);
            if rect_contains(x, y, w, h, mouse) {
                let action = match i {
                    0 => HelpAction::SuitFilter(None),
                    1 => HelpAction::SuitFilter(Some(Suit::Wan)),
                    2 => HelpAction::SuitFilter(Some(Suit::Tiao)),
                    3 => HelpAction::SuitFilter(Some(Suit::Tong)),
                    _ => unreachable!(),
                };
                return action;
            }
        }
        for i in 0..10 {
            let (x, y, w, h) = rank_btn_rect(&g, i);
            if rect_contains(x, y, w, h, mouse) {
                let action = if i == 0 {
                    HelpAction::RankFilter(None)
                } else {
                    HelpAction::RankFilter(Some(i as u8))
                };
                return action;
            }
        }
    }

    HelpAction::None
}

// ─── Drawing ─────────────────────────────────────────────────────────────

pub fn draw_help_overlay(
    layout: &TableLayout,
    ctx: &GameContext,
    state: &HelpState,
    mouse: Vec2,
) {
    draw_rectangle(
        0.0, 0.0, layout.scr_w, layout.scr_h,
        Color::new(0.02, 0.05, 0.04, 0.94),
    );

    let g = HelpGeom::new(layout);

    draw_rectangle(g.px, g.py, g.pw, g.ph, Color::new(0.10, 0.18, 0.13, 1.0));
    draw_rectangle_lines(g.px, g.py, g.pw, g.ph, 3.0, ctx.colors.gold_dark);
    draw_rectangle_lines(
        g.px + 4.0, g.py + 4.0, g.pw - 8.0, g.ph - 8.0,
        1.5, ctx.colors.gold_light,
    );

    let d = 9.0;
    for (cx, cy) in [
        (g.px + 12.0, g.py + 12.0),
        (g.px + g.pw - 12.0, g.py + 12.0),
        (g.px + 12.0, g.py + g.ph - 12.0),
        (g.px + g.pw - 12.0, g.py + g.ph - 12.0),
    ] {
        draw_poly(cx, cy, 4, d, 45.0, ctx.colors.gold_light);
        draw_poly_lines(cx, cy, 4, d, 45.0, 1.0, ctx.colors.gold_dark);
    }

    text_centered(
        "How to play Zhuo Ji",
        g.px + g.pw * 0.5,
        g.py + 46.0,
        30.0,
        ctx.colors.text,
    );

    draw_close_button(&g, ctx, mouse);
    draw_tabs(&g, ctx, state, mouse);

    match state.tab {
        HelpTab::Goal => draw_goal(&g, ctx),
        HelpTab::Patterns => draw_patterns(&g, ctx),
        HelpTab::Dou => draw_dou(&g, ctx),
        HelpTab::Ji => draw_ji(&g, ctx),
        HelpTab::Tiles => draw_tiles(&g, ctx, state, mouse),
        HelpTab::Specials => draw_specials(&g, ctx),
        HelpTab::Interface => draw_interface(&g, ctx),
    }

    draw_got_it(&g, ctx, mouse);
}

fn draw_close_button(g: &HelpGeom, ctx: &GameContext, mouse: Vec2) {
    let (x, y, w, h) = close_btn_rect(g);
    let hovered = rect_contains(x, y, w, h, mouse);
    let bg = if hovered {
        ctx.colors.danger
    } else {
        Color::new(0.20, 0.14, 0.14, 1.0)
    };
    draw_rectangle(x, y, w, h, bg);
    draw_rectangle_lines(x, y, w, h, 2.0, ctx.colors.gold_light);
    let cx = x + w * 0.5;
    let cy = y + h * 0.5;
    let s = 8.0;
    let col = ctx.colors.text;
    draw_line(cx - s, cy - s, cx + s, cy + s, 2.5, col);
    draw_line(cx - s, cy + s, cx + s, cy - s, 2.5, col);
}

fn draw_tabs(g: &HelpGeom, ctx: &GameContext, state: &HelpState, mouse: Vec2) {
    for (i, tab) in HelpTab::ALL.iter().enumerate() {
        let (x, y, w, h) = tab_rect(g, i);
        let hovered = rect_contains(x, y, w, h, mouse);
        let active = *tab == state.tab;

        let bg = if active {
            ctx.colors.gold_dark
        } else if hovered {
            Color::new(0.20, 0.30, 0.24, 1.0)
        } else {
            Color::new(0.13, 0.22, 0.17, 1.0)
        };
        let border = if active {
            ctx.colors.gold_light
        } else {
            Color::new(0.35, 0.45, 0.40, 1.0)
        };

        draw_rectangle(x, y, w, h, bg);
        draw_rectangle_lines(x, y, w, h, 2.0, border);
        let label = tab.label();
        let txt = if active { ctx.colors.tile_text } else { ctx.colors.text };
        text_centered(label, x + w * 0.5, y + h * 0.5 + 7.0, 17.0, txt);
    }
}

fn draw_got_it(g: &HelpGeom, ctx: &GameContext, mouse: Vec2) {
    let (x, y, w, h) = got_it_btn_rect(g);
    let hovered = rect_contains(x, y, w, h, mouse);
    let bg = if hovered {
        ctx.colors.highlight
    } else {
        ctx.colors.gold_dark
    };
    draw_rectangle(x, y, w, h, bg);
    draw_rectangle_lines(x, y, w, h, 2.5, ctx.colors.gold_light);
    text_centered(
        "Got it, let's play!",
        x + w * 0.5,
        y + h * 0.5 + 8.0,
        20.0,
        ctx.colors.tile_text,
    );
}

// ─── Text helpers ────────────────────────────────────────────────────────

fn text_centered(text: &str, cx: f32, y: f32, size: f32, color: Color) {
    let dim = measure_text(text, None, size as u16, 1.0);
    draw_text(text, cx - dim.width * 0.5, y, size, color);
}

fn heading(x: f32, y: f32, text: &str, ctx: &GameContext) {
    draw_text(">", x, y, 20.0, ctx.colors.gold_light);
    draw_text(text, x + 20.0, y, 20.0, ctx.colors.gold_light);
}

fn body(x: f32, y: f32, text: &str, ctx: &GameContext) {
    draw_text(text, x, y, 16.0, ctx.colors.text);
}

fn dim(x: f32, y: f32, text: &str, ctx: &GameContext) {
    draw_text(text, x, y, 15.0, ctx.colors.text_dim);
}

fn term_line(x: f32, y: f32, term: &str, desc: &str, ctx: &GameContext) {
    draw_text(term, x, y, 16.0, ctx.colors.highlight);
    let tw = measure_text(term, None, 16, 1.0).width;
    draw_text(desc, x + tw + 10.0, y, 16.0, ctx.colors.text);
}

// ─── Tab 1: Goal ─────────────────────────────────────────────────────────

fn draw_goal(g: &HelpGeom, ctx: &GameContext) {
    let x = g.content_x;
    let mut y = g.content_y + 20.0;

    heading(x, y, "Goal", ctx);
    y += LINE + 6.0;
    body(x + 20.0, y, "Form 4 sets (triplets or sequences) + 1 pair,", ctx);
    y += LINE;
    body(x + 20.0, y, "and do it before the other three players.", ctx);
    y += LINE;
    dim(x + 20.0, y, "No Chi (eating) in Guiyang: only Peng, Gang, Hu.", ctx);
    y += LINE + 18.0;

    heading(x, y, "The tiles", ctx);
    y += LINE + 6.0;
    dim(x + 20.0, y, "3 suits x 9 ranks x 4 copies = 108 tiles.", ctx);
    y += LINE + 6.0;

    let preview_y = y;
    let preview_size = TileSize::new(52.0, 78.0);
    let labels = ["Wan (characters)", "Tiao (bamboo)", "Tong (dots)"];
    let sample = [
        Tile::new(Suit::Wan, 5),
        Tile::new(Suit::Tiao, 5),
        Tile::new(Suit::Tong, 5),
    ];
    let mut tx = x + 20.0;
    for (i, t) in sample.iter().enumerate() {
        tiles::draw_tile_flat(*t, tx, preview_y, preview_size, &ctx.colors, &ctx.font);
        let cx = tx + preview_size.w * 0.5;
        text_centered(labels[i], cx, preview_y + preview_size.h + 20.0, 14.0, ctx.colors.text_dim);
        tx += preview_size.w + 40.0;
    }
    y = preview_y + preview_size.h + 48.0;

    heading(x, y, "Your turn, in 3 steps", ctx);
    y += LINE + 6.0;
    body(x + 20.0, y, "1. Draw a tile (it appears on the right)", ctx);
    y += LINE;
    body(x + 20.0, y, "2. Discard a tile (click on it)", ctx);
    y += LINE;
    body(x + 20.0, y, "3. Others may Peng / Gang / Hu", ctx);
    y += LINE + 18.0;

    heading(x, y, "The 3 actions", ctx);
    y += LINE + 6.0;
    term_line(x + 20.0, y, "PENG", "- Take a discarded tile to form a triplet (3 identical)", ctx);
    y += LINE;
    term_line(x + 20.0, y, "GANG", "- Take a discarded tile to form a quad (4 identical)", ctx);
    y += LINE;
    term_line(x + 20.0, y, "HU", "- Win: your hand is complete", ctx);
}

// ─── Tab 2: Patterns ─────────────────────────────────────────────────────

fn draw_patterns(g: &HelpGeom, ctx: &GameContext) {
    let x = g.content_x;
    let mut y = g.content_y + 20.0;

    heading(x, y, "Hand patterns and their fan value", ctx);
    y += LINE + 10.0;

    let c_name = x + 20.0;
    let c_fan = x + 320.0;
    let c_desc = x + 420.0;
    draw_text("Name", c_name, y, 15.0, ctx.colors.text_dim);
    draw_text("Fan", c_fan, y, 15.0, ctx.colors.text_dim);
    draw_text("Description", c_desc, y, 15.0, ctx.colors.text_dim);
    y += 8.0;
    draw_line(x + 20.0, y, x + g.content_w - 20.0, y, 1.0, ctx.colors.gold_dark);
    y += 20.0;

    let rows: &[(&str, &str, &str)] = &[
        ("Pinghu",       "1",  "Standard hand (4 sets + 1 pair)"),
        ("Da dui zi",    "5",  "All triplets (4 triplets + 1 pair)"),
        ("Qi dui",       "10", "Seven pairs"),
        ("Qing yi se",   "10", "Single suit (Wan / Tiao / Tong)"),
        ("Bao ting",     "10", "Declared ready from the start"),
        ("Qing da dui",  "15", "Da dui zi + Qing yi se"),
        ("Long qi dui",  "20", "5 pairs + 1 concealed quad"),
        ("Qing qi dui",  "20", "Qi dui + Qing yi se"),
        ("Qing long bei","30", "Long qi dui + Qing yi se"),
    ];
    for (name, fan, desc) in rows {
        draw_text(name, c_name, y, 16.0, ctx.colors.highlight);
        draw_text(fan, c_fan, y, 16.0, ctx.colors.text);
        draw_text(desc, c_desc, y, 15.0, ctx.colors.text_dim);
        y += LINE;
    }

    y += 14.0;
    draw_line(x + 20.0, y, x + g.content_w - 20.0, y, 1.0, ctx.colors.gold_dark);
    y += 24.0;
    dim(x + 20.0, y, "Formula: Total = Pattern + Dou + Ji (in fan).", ctx);
}

// ─── Tab 3: Dou ──────────────────────────────────────────────────────────

fn draw_dou(g: &HelpGeom, ctx: &GameContext) {
    let x = g.content_x;
    let mut y = g.content_y + 20.0;

    heading(x, y, "The Dou are the Gangs", ctx);
    y += LINE + 6.0;
    dim(x + 20.0, y, "Three types, three payment schemes.", ctx);
    y += LINE + 18.0;

    term_line(x + 20.0, y, "Men dou", "- Concealed kong (4 in hand)", ctx);
    y += LINE;
    dim(x + 40.0, y, "Each other player pays you 2 fan.", ctx);
    y += LINE + 8.0;

    term_line(x + 20.0, y, "Pa po dou", "- Added kong (upgrade a Peng)", ctx);
    y += LINE;
    dim(x + 40.0, y, "Each other player pays you 3 fan.", ctx);
    y += LINE + 8.0;

    term_line(x + 20.0, y, "Dian dou", "- Exposed kong (from a discard)", ctx);
    y += LINE;
    dim(x + 40.0, y, "Only the discarder pays you 1 fan.", ctx);
    y += LINE + 22.0;

    let box_y = y - 8.0;
    let box_h = LINE * 4.0 + 24.0;
    draw_rectangle(
        x + 20.0, box_y, g.content_w - 40.0, box_h,
        Color::new(0.22, 0.16, 0.08, 1.0),
    );
    draw_rectangle_lines(
        x + 20.0, box_y, g.content_w - 40.0, box_h,
        2.0, ctx.colors.highlight,
    );
    let mut by = box_y + 28.0;
    draw_text("IMPORTANT RULE: the Dou passport", x + 40.0, by, 18.0, ctx.colors.highlight);
    by += LINE + 4.0;
    body(x + 40.0, by, "Without a Dou, you can only win by self-draw (Zimo),", ctx);
    by += LINE;
    body(x + 40.0, by, "or with a Da dui zi or better.", ctx);
    by += LINE;
    body(x + 40.0, by, "No Hu on a discard with only a Pinghu!", ctx);
}

// ─── Tab 4: Ji ───────────────────────────────────────────────────────────

fn draw_ji(g: &HelpGeom, ctx: &GameContext) {
    let x = g.content_x;
    let mut y = g.content_y + 20.0;

    heading(x, y, "The Ji are the chickens", ctx);
    y += LINE + 6.0;
    dim(x + 20.0, y, "1 Ji = 1 fan from each other player, at hand end.", ctx);
    y += LINE + 18.0;

    body(x + 20.0, y, "Permanent Ji (always counted):", ctx);
    y += LINE;
    term_line(x + 40.0, y, "1 Tiao", "(Yao ji)   always a Ji", ctx);
    y += LINE;
    term_line(x + 40.0, y, "8 Tong", "(Ba tong)  always a Ji", ctx);
    y += LINE + 16.0;

    body(x + 20.0, y, "After a win, one tile is flipped:", ctx);
    y += LINE;
    dim(x + 40.0, y, "the next one also becomes a Ji.", ctx);
    y += LINE;
    dim(x + 40.0, y, "Example: flip 5 Wan -> 6 Wan becomes Ji.", ctx);
    y += LINE + 18.0;

    body(x + 20.0, y, "Special variants:", ctx);
    y += LINE;
    term_line(x + 40.0, y, "Jin ji", "(2 fan)  - If the flip is 9 Tiao, 1 Tiao becomes golden", ctx);
    y += LINE;
    term_line(x + 40.0, y, "Chong feng ji", "- The first discarded 1 Tiao counts double", ctx);
    y += LINE;
    term_line(x + 40.0, y, "Ze ren ji", "- If that Ji is Peng'd, the discarder pays +1 fan", ctx);
}

// ─── Tab 5: Tiles ────────────────────────────────────────────────────────

fn draw_tiles(g: &HelpGeom, ctx: &GameContext, state: &HelpState, mouse: Vec2) {
    // ─── Suit filter row ─────────────────────────────────────────
    draw_text("Suit:", g.content_x + 20.0, g.content_y + 45.0, 16.0, ctx.colors.text_dim);
    let suit_labels = ["All", "Wan", "Tiao", "Tong"];
    let suit_values: [Option<Suit>; 4] = [
        None,
        Some(Suit::Wan),
        Some(Suit::Tiao),
        Some(Suit::Tong),
    ];
    for (i, label) in suit_labels.iter().enumerate() {
        let rect = suit_btn_rect(g, i);
        let active = state.suit_filter == suit_values[i];
        let hovered = rect_contains(rect.0, rect.1, rect.2, rect.3, mouse);
        draw_filter_btn(rect, label, active, hovered, ctx);
    }

    // ─── Rank filter row ─────────────────────────────────────────
    draw_text("Rank:", g.content_x + 20.0, g.content_y + 89.0, 16.0, ctx.colors.text_dim);
    for i in 0..10 {
        let rect = rank_btn_rect(g, i);
        let label = if i == 0 { "All".to_string() } else { i.to_string() };
        let active = if i == 0 {
            state.rank_filter.is_none()
        } else {
            state.rank_filter == Some(i as u8)
        };
        let hovered = rect_contains(rect.0, rect.1, rect.2, rect.3, mouse);
        draw_filter_btn(rect, &label, active, hovered, ctx);
    }

    // ─── Tile grid ───────────────────────────────────────────────
    let grid_y = g.content_y + 120.0;
    let tile_w = 56.0;
    let tile_h = 78.0;
    let gap_x = 8.0;
    let gap_y = 8.0;
    let row_h = tile_h + gap_y;
    let grid_x = g.content_x + 100.0;
    let size = TileSize::new(tile_w, tile_h);

    let suits = [Suit::Wan, Suit::Tiao, Suit::Tong];
    let suit_names = ["Wan", "Tiao", "Tong"];

    for (row, suit) in suits.iter().enumerate() {
        let row_y = grid_y + row as f32 * row_h;
        let label_cy = row_y + tile_h * 0.5 + 6.0;
        draw_text(
            suit_names[row],
            g.content_x + 20.0,
            label_cy,
            18.0,
            ctx.colors.text,
        );

        for rank in 1..=9u8 {
            let x = grid_x + (rank as f32 - 1.0) * (tile_w + gap_x);
            let tile = Tile::new(*suit, rank);

            let matches_suit = state.suit_filter.is_none()
                || state.suit_filter == Some(*suit);
            let matches_rank = state.rank_filter.is_none()
                || state.rank_filter == Some(rank);

            tiles::draw_tile_flat(tile, x, row_y, size, &ctx.colors, &ctx.font);

            if !(matches_suit && matches_rank) {
                draw_rectangle(
                    x, row_y, size.w, size.h,
                    Color::new(0.08, 0.16, 0.11, 0.75),
                );
            }
        }
    }

    // ─── Note ─────────────────────────────────────────────────────
    let note_y = grid_y + 3.0 * row_h + 14.0;
    dim(
        g.content_x + 20.0,
        note_y,
        "Note: 1 Tiao and 8 Tong are permanent Ji (chickens).",
        ctx,
    );
}

fn draw_filter_btn(
    rect: (f32, f32, f32, f32),
    label: &str,
    active: bool,
    hovered: bool,
    ctx: &GameContext,
) {
    let (x, y, w, h) = rect;
    let (bg, border, txt) = if active {
        (ctx.colors.gold_dark, ctx.colors.gold_light, ctx.colors.tile_text)
    } else if hovered {
        (
            Color::new(0.20, 0.30, 0.24, 1.0),
            ctx.colors.gold_light,
            ctx.colors.text,
        )
    } else {
        (
            Color::new(0.13, 0.22, 0.17, 1.0),
            Color::new(0.35, 0.45, 0.40, 1.0),
            ctx.colors.text_dim,
        )
    };
    draw_rectangle(x, y, w, h, bg);
    draw_rectangle_lines(x, y, w, h, 1.5, border);
    text_centered(label, x + w * 0.5, y + h * 0.5 + 5.0, 14.0, txt);
}

// ─── Tab 6: Specials + scoring ───────────────────────────────────────────

fn draw_specials(g: &HelpGeom, ctx: &GameContext) {
    let x = g.content_x;
    let mut y = g.content_y + 20.0;

    heading(x, y, "Special situations", ctx);
    y += LINE + 6.0;
    term_line(x + 20.0, y, "Gang shang pao", "- Win on the tile discarded after a Gang", ctx);
    y += LINE;
    term_line(x + 20.0, y, "Qiang gang", "- Steal a Bu Gang tile to win", ctx);
    y += LINE;
    term_line(x + 20.0, y, "Huang zhuang", "- Empty wall: hands are revealed.", ctx);
    y += LINE;
    dim(x + 60.0, y, "Non-tenpai players pay tenpai players (hand value).", ctx);
    y += LINE;
    dim(x + 60.0, y, "Dou and Ji don't count during Huang zhuang.", ctx);
    y += LINE;
    term_line(x + 20.0, y, "Bao ting", "- Declared ready from the start:", ctx);
    y += LINE;
    dim(x + 60.0, y, "if you win, your fan = that of a Qing yi se (10).", ctx);
    y += LINE + 22.0;

    heading(x, y, "Scoring", ctx);
    y += LINE + 6.0;
    body(x + 20.0, y, "Total = Pattern + Dou + Ji  (in fan)", ctx);
    y += LINE;
    term_line(x + 20.0, y, "Zimo", "- all 3 other players pay", ctx);
    y += LINE;
    term_line(x + 20.0, y, "Hu", "- only the discarder pays", ctx);
    y += LINE;
    term_line(x + 20.0, y, "Dealer", "- wins/loses one extra fan", ctx);
    y += LINE + 22.0;

    heading(x, y, "Keyboard shortcuts", ctx);
    y += LINE + 6.0;
    dim(x + 20.0, y, "H  : this help     Space : next hand", ctx);
    y += LINE;
    dim(x + 20.0, y, "R  : new match     Escape : back to menu", ctx);
}

// ─── Tab 7: Interface ────────────────────────────────────────────────────

fn draw_interface(g: &HelpGeom, ctx: &GameContext) {
    let x = g.content_x;
    let mut y = g.content_y + 20.0;

    heading(x, y, "Reading the screen", ctx);
    y += LINE + 10.0;

    term_line(x + 20.0, y, "Shanten", "- how many tiles you are from a winning hand.", ctx);
    y += LINE;
    dim(x + 60.0, y, "0 = tenpai (1 tile away).", ctx);
    y += LINE;
    dim(x + 60.0, y, "1 = 1-shanten (2 tiles away), and so on.", ctx);
    y += LINE;
    dim(x + 60.0, y, "Shown in the top-left corner of the screen.", ctx);
    y += LINE + 20.0;

    heading(x, y, "Indicators on the table", ctx);
    y += LINE + 10.0;

    term_line(x + 20.0, y, "Gold dot", "- a tile that is useful for your hand", ctx);
    y += LINE;
    dim(x + 60.0, y, "(it forms a pair or a partial sequence).", ctx);
    y += LINE + 8.0;

    term_line(x + 20.0, y, "Clock arc", "- time left for your decision.", ctx);
    y += LINE;
    dim(x + 60.0, y, "Green -> yellow -> red as the timer runs out.", ctx);
    y += LINE + 8.0;

    term_line(x + 20.0, y, "Center arrow", "- points to the current player.", ctx);
    y += LINE + 8.0;

    term_line(x + 20.0, y, "Avatar frame", "- highlighted gold for the active seat.", ctx);
    y += LINE + 8.0;

    term_line(x + 20.0, y, "Drawn marker", "- a small triangle marks the tile", ctx);
    y += LINE;
    dim(x + 60.0, y, "just drawn this turn, shown apart from the hand.", ctx);
}

// ─── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn st(tab: HelpTab) -> HelpState {
        HelpState {
            visible: true,
            has_seen: true,
            tab,
            suit_filter: None,
            rank_filter: None,
        }
    }

    #[test]
    fn test_help_tab_cycle_forward() {
        assert_eq!(HelpTab::Goal.next(), HelpTab::Patterns);
        assert_eq!(HelpTab::Interface.next(), HelpTab::Goal);
    }

    #[test]
    fn test_help_tab_cycle_backward() {
        assert_eq!(HelpTab::Goal.prev(), HelpTab::Interface);
        assert_eq!(HelpTab::Ji.prev(), HelpTab::Dou);
    }

    #[test]
    fn test_help_tab_idx_roundtrip() {
        for t in HelpTab::ALL {
            assert_eq!(HelpTab::from_idx(t.idx()), t);
        }
    }

    #[test]
    fn test_help_state_toggle_opens_and_closes() {
        let mut s = st(HelpTab::Goal);
        s.visible = false;
        s.toggle();
        assert!(s.visible);
        s.toggle();
        assert!(!s.visible);
    }

    #[test]
    fn test_help_state_close_marks_seen() {
        let mut s = st(HelpTab::Goal);
        s.has_seen = false;
        s.close();
        assert!(!s.visible);
        assert!(s.has_seen);
    }

    #[test]
    fn test_help_tab_rects_are_inside_panel() {
        let layout = TableLayout::new(1440.0, 900.0);
        let g = HelpGeom::new(&layout);
        for i in 0..HelpTab::ALL.len() {
            let (x, y, w, h) = tab_rect(&g, i);
            assert!(x >= g.px);
            assert!(x + w <= g.px + g.pw);
            assert!(y >= g.py);
            assert!(y + h <= g.py + g.ph);
        }
    }

    #[test]
    fn test_close_button_inside_panel() {
        let layout = TableLayout::new(1440.0, 900.0);
        let g = HelpGeom::new(&layout);
        let (x, y, w, h) = close_btn_rect(&g);
        assert!(x + w <= g.px + g.pw);
        assert!(y + h <= g.py + g.ph);
    }

    #[test]
    fn test_got_it_button_centered() {
        let layout = TableLayout::new(1440.0, 900.0);
        let g = HelpGeom::new(&layout);
        let (x, _y, w, _h) = got_it_btn_rect(&g);
        let mid = x + w * 0.5;
        let panel_mid = g.px + g.pw * 0.5;
        assert!((mid - panel_mid).abs() < 0.5);
    }

    #[test]
    fn test_hit_test_close_button() {
        let layout = TableLayout::new(1440.0, 900.0);
        let g = HelpGeom::new(&layout);
        let (x, y, w, h) = close_btn_rect(&g);
        let m = Vec2::new(x + w * 0.5, y + h * 0.5);
        assert_eq!(hit_test(&layout, &st(HelpTab::Goal), m), HelpAction::Close);
    }

    #[test]
    fn test_hit_test_tab() {
        let layout = TableLayout::new(1440.0, 900.0);
        let g = HelpGeom::new(&layout);
        let (x, y, w, h) = tab_rect(&g, 2);
        let m = Vec2::new(x + w * 0.5, y + h * 0.5);
        assert_eq!(
            hit_test(&layout, &st(HelpTab::Goal), m),
            HelpAction::Tab(HelpTab::Dou)
        );
    }

    #[test]
    fn test_hit_test_nothing_outside() {
        let layout = TableLayout::new(1440.0, 900.0);
        assert_eq!(
            hit_test(&layout, &st(HelpTab::Goal), Vec2::new(5.0, 5.0)),
            HelpAction::None
        );
    }

    #[test]
    fn test_interface_tab_exists() {
        assert_eq!(HelpTab::Interface.label(), "UI");
        assert_eq!(HelpTab::from_idx(6), HelpTab::Interface);
    }

    #[test]
    fn test_tiles_tab_exists() {
        assert_eq!(HelpTab::Tiles.label(), "Tiles");
        assert_eq!(HelpTab::from_idx(4), HelpTab::Tiles);
    }

    #[test]
    fn test_hit_test_suit_filter_when_tiles_tab() {
        let layout = TableLayout::new(1440.0, 900.0);
        let g = HelpGeom::new(&layout);
        // Click "Wan" (index 1).
        let (x, y, w, h) = suit_btn_rect(&g, 1);
        let m = Vec2::new(x + w * 0.5, y + h * 0.5);
        assert_eq!(
            hit_test(&layout, &st(HelpTab::Tiles), m),
            HelpAction::SuitFilter(Some(Suit::Wan))
        );
    }

    #[test]
    fn test_hit_test_suit_filter_all() {
        let layout = TableLayout::new(1440.0, 900.0);
        let g = HelpGeom::new(&layout);
        let (x, y, w, h) = suit_btn_rect(&g, 0);
        let m = Vec2::new(x + w * 0.5, y + h * 0.5);
        assert_eq!(
            hit_test(&layout, &st(HelpTab::Tiles), m),
            HelpAction::SuitFilter(None)
        );
    }

    #[test]
    fn test_hit_test_rank_filter() {
        let layout = TableLayout::new(1440.0, 900.0);
        let g = HelpGeom::new(&layout);
        // Rank button index 5 = rank 5.
        let (x, y, w, h) = rank_btn_rect(&g, 5);
        let m = Vec2::new(x + w * 0.5, y + h * 0.5);
        assert_eq!(
            hit_test(&layout, &st(HelpTab::Tiles), m),
            HelpAction::RankFilter(Some(5))
        );
    }

    #[test]
    fn test_hit_test_filter_inactive_on_other_tabs() {
        let layout = TableLayout::new(1440.0, 900.0);
        let g = HelpGeom::new(&layout);
        let (x, y, w, h) = suit_btn_rect(&g, 1);
        let m = Vec2::new(x + w * 0.5, y + h * 0.5);
        // On the Goal tab, filter buttons are not active → no action.
        assert_eq!(hit_test(&layout, &st(HelpTab::Goal), m), HelpAction::None);
    }
}