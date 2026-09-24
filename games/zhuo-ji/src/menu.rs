//! Main menu — title screen with three buttons.

use macroquad::prelude::*;

use crate::components::{Suit, Tile};
use crate::config::GameContext;
use crate::layout::{TableLayout, TileSize};
use crate::tiles;

pub const VERSION: &str = "0.1.0";

const PANEL_W: f32 = 560.0;
const PANEL_H: f32 = 620.0;
const BTN_W: f32 = 300.0;
const BTN_H: f32 = 56.0;
const BTN_GAP: f32 = 16.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    NewGame,
    Rules,
    Quit,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuButton {
    NewGame,
    Rules,
    Quit,
}

impl MenuButton {
    pub const ALL: [MenuButton; 3] = [MenuButton::NewGame, MenuButton::Rules, MenuButton::Quit];

    pub fn label(self) -> &'static str {
        match self {
            MenuButton::NewGame => "New game",
            MenuButton::Rules => "Rules",
            MenuButton::Quit => "Quit",
        }
    }
}

pub struct MenuState {
    pub selected: usize,
}

impl Default for MenuState {
    fn default() -> Self {
        Self::new()
    }
}

impl MenuState {
    pub fn new() -> Self {
        Self { selected: 0 }
    }

    pub fn up(&mut self) {
        self.selected = (self.selected + 2) % 3;
    }

    pub fn down(&mut self) {
        self.selected = (self.selected + 1) % 3;
    }

    pub fn action(&self) -> MenuAction {
        match self.selected {
            0 => MenuAction::NewGame,
            1 => MenuAction::Rules,
            2 => MenuAction::Quit,
            _ => MenuAction::None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MenuGeom {
    pub px: f32,
    pub py: f32,
    pub pw: f32,
    pub ph: f32,
}

impl MenuGeom {
    pub fn new(layout: &TableLayout) -> Self {
        let pw = PANEL_W.min(layout.scr_w - 80.0);
        let ph = PANEL_H.min(layout.scr_h - 80.0);
        let px = (layout.scr_w - pw) * 0.5;
        let py = (layout.scr_h - ph) * 0.5;
        Self { px, py, pw, ph }
    }
}

pub fn button_rect(g: &MenuGeom, i: usize) -> (f32, f32, f32, f32) {
    let start_y = g.py + 350.0;
    let x = g.px + (g.pw - BTN_W) * 0.5;
    let y = start_y + i as f32 * (BTN_H + BTN_GAP);
    (x, y, BTN_W, BTN_H)
}

fn rect_contains(x: f32, y: f32, w: f32, h: f32, p: Vec2) -> bool {
    p.x >= x && p.x <= x + w && p.y >= y && p.y <= y + h
}

pub fn hit_button_idx(layout: &TableLayout, mouse: Vec2) -> Option<usize> {
    let g = MenuGeom::new(layout);
    for i in 0..MenuButton::ALL.len() {
        let (x, y, w, h) = button_rect(&g, i);
        if rect_contains(x, y, w, h, mouse) {
            return Some(i);
        }
    }
    None
}

pub fn hit_test(layout: &TableLayout, mouse: Vec2) -> MenuAction {
    match hit_button_idx(layout, mouse) {
        Some(0) => MenuAction::NewGame,
        Some(1) => MenuAction::Rules,
        Some(2) => MenuAction::Quit,
        _ => MenuAction::None,
    }
}

pub fn draw_menu(layout: &TableLayout, ctx: &GameContext, state: &MenuState, mouse: Vec2) {
    // Soft dimmer over the background mountains.
    draw_rectangle(
        0.0, 0.0, layout.scr_w, layout.scr_h,
        Color::new(0.02, 0.05, 0.04, 0.45),
    );

    let g = MenuGeom::new(layout);

    // Panel background.
    draw_rectangle(g.px, g.py, g.pw, g.ph, Color::new(0.10, 0.18, 0.13, 1.0));
    draw_rectangle_lines(g.px, g.py, g.pw, g.ph, 3.0, ctx.colors.gold_dark);
    draw_rectangle_lines(
        g.px + 4.0, g.py + 4.0, g.pw - 8.0, g.ph - 8.0,
        1.5, ctx.colors.gold_light,
    );

    // Corner diamonds.
    for (dx, dy) in [
        (g.px + 12.0, g.py + 12.0),
        (g.px + g.pw - 12.0, g.py + 12.0),
        (g.px + 12.0, g.py + g.ph - 12.0),
        (g.px + g.pw - 12.0, g.py + g.ph - 12.0),
    ] {
        draw_poly(dx, dy, 4, 10.0, 45.0, ctx.colors.gold_light);
        draw_poly_lines(dx, dy, 4, 10.0, 45.0, 1.0, ctx.colors.gold_dark);
    }

    // Pulsing title.
    let cx = g.px + g.pw * 0.5;
    let pulse = 1.0 + 0.03 * (get_time() as f32 * 2.5).sin();
    text_centered("Zhuo Ji", cx, g.py + 110.0, 64.0 * pulse, ctx.colors.highlight);

    // CJK subtitle (uses the TileFont).
    if ctx.font.is_available() {
        ctx.font.draw_centered(
            "贵阳捉鸡麻将",
            cx,
            g.py + 155.0,
            28.0,
            ctx.colors.text,
        );
    }
    text_centered("Guiyang Mahjong", cx, g.py + 190.0, 18.0, ctx.colors.text_dim);

    // Divider.
    draw_line(
        g.px + 80.0, g.py + 215.0,
        g.px + g.pw - 80.0, g.py + 215.0,
        1.0, ctx.colors.gold_dark,
    );

    // Three decorative tiles: 5W, 1T, 8D (two permanent Ji highlighted).
    let deco_size = TileSize::new(52.0, 72.0);
    let deco_gap = 32.0;
    let deco_count = 3.0;
    let deco_total = deco_count * deco_size.w + (deco_count - 1.0) * deco_gap;
    let deco_x0 = cx - deco_total * 0.5;
    let deco_y = g.py + 240.0;
    let deco_tiles = [
        Tile::new(Suit::Wan, 5),
        Tile::new(Suit::Tiao, 1),
        Tile::new(Suit::Tong, 8),
    ];
    for (i, t) in deco_tiles.iter().enumerate() {
        let tx = deco_x0 + i as f32 * (deco_size.w + deco_gap);
        tiles::draw_tile_flat(*t, tx, deco_y, deco_size, &ctx.colors, &ctx.font);
    }

    // Buttons.
    for (i, btn) in MenuButton::ALL.iter().enumerate() {
        let (x, y, w, h) = button_rect(&g, i);
        let hovered = rect_contains(x, y, w, h, mouse);
        let selected = state.selected == i;
        let active = hovered || selected;

        let (bg, txt, border) = if active {
            (
                ctx.colors.gold_light,
                ctx.colors.tile_text,
                ctx.colors.gold_light,
            )
        } else {
            (
                ctx.colors.gold_dark,
                ctx.colors.text,
                Color::new(0.65, 0.55, 0.28, 1.0),
            )
        };
        draw_rectangle(x, y, w, h, bg);
        draw_rectangle_lines(x, y, w, h, 2.5, border);
        if selected && !hovered {
            // Draw a small left-margin indicator for keyboard-selected.
            let mx = x - 22.0;
            let my = y + h * 0.5;
            draw_triangle(
                Vec2::new(mx - 6.0, my - 8.0),
                Vec2::new(mx + 6.0, my),
                Vec2::new(mx - 6.0, my + 8.0),
                ctx.colors.highlight,
            );
        }
        text_centered(btn.label(), x + w * 0.5, y + h * 0.5 + 9.0, 22.0, txt);
    }

    // Footer hints centered.
    text_centered(
        "Up/Down select   Enter confirm   H rules",
        cx,
        g.py + g.ph - 34.0,
        14.0,
        ctx.colors.text_dim,
    );

    // Version stamp.
    text_right(
        &format!("v{VERSION}"),
        g.px + g.pw - 16.0,
        g.py + g.ph - 12.0,
        13.0,
        ctx.colors.text_dim,
    );
}

fn text_centered(text: &str, cx: f32, y: f32, size: f32, color: Color) {
    let dim = measure_text(text, None, size as u16, 1.0);
    draw_text(text, cx - dim.width * 0.5, y, size, color);
}

fn text_right(text: &str, rx: f32, y: f32, size: f32, color: Color) {
    let dim = measure_text(text, None, size as u16, 1.0);
    draw_text(text, rx - dim.width, y, size, color);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_menu_button_labels() {
        assert_eq!(MenuButton::NewGame.label(), "New game");
        assert_eq!(MenuButton::Rules.label(), "Rules");
        assert_eq!(MenuButton::Quit.label(), "Quit");
    }

    #[test]
    fn test_menu_hit_test_new_game() {
        let layout = TableLayout::new(1440.0, 900.0);
        let g = MenuGeom::new(&layout);
        let (x, y, w, h) = button_rect(&g, 0);
        let m = Vec2::new(x + w * 0.5, y + h * 0.5);
        assert_eq!(hit_test(&layout, m), MenuAction::NewGame);
    }

    #[test]
    fn test_menu_hit_test_rules() {
        let layout = TableLayout::new(1440.0, 900.0);
        let g = MenuGeom::new(&layout);
        let (x, y, w, h) = button_rect(&g, 1);
        let m = Vec2::new(x + w * 0.5, y + h * 0.5);
        assert_eq!(hit_test(&layout, m), MenuAction::Rules);
    }

    #[test]
    fn test_menu_hit_test_quit() {
        let layout = TableLayout::new(1440.0, 900.0);
        let g = MenuGeom::new(&layout);
        let (x, y, w, h) = button_rect(&g, 2);
        let m = Vec2::new(x + w * 0.5, y + h * 0.5);
        assert_eq!(hit_test(&layout, m), MenuAction::Quit);
    }

    #[test]
    fn test_menu_hit_test_outside() {
        let layout = TableLayout::new(1440.0, 900.0);
        assert_eq!(hit_test(&layout, Vec2::new(5.0, 5.0)), MenuAction::None);
    }

    #[test]
    fn test_menu_panel_inside_screen() {
        for (w, h) in [(1440.0, 900.0), (1280.0, 800.0), (1000.0, 700.0)] {
            let layout = TableLayout::new(w, h);
            let g = MenuGeom::new(&layout);
            assert!(g.px >= 0.0);
            assert!(g.py >= 0.0);
            assert!(g.px + g.pw <= w);
            assert!(g.py + g.ph <= h);
        }
    }

    #[test]
    fn test_menu_buttons_inside_panel() {
        let layout = TableLayout::new(1440.0, 900.0);
        let g = MenuGeom::new(&layout);
        for i in 0..MenuButton::ALL.len() {
            let (x, y, w, h) = button_rect(&g, i);
            assert!(x >= g.px);
            assert!(x + w <= g.px + g.pw);
            assert!(y >= g.py);
            assert!(y + h <= g.py + g.ph);
        }
    }

    #[test]
    fn test_menu_state_cycles_down() {
        let mut s = MenuState::new();
        assert_eq!(s.selected, 0);
        s.down();
        assert_eq!(s.selected, 1);
        s.down();
        assert_eq!(s.selected, 2);
        s.down();
        assert_eq!(s.selected, 0);
    }

    #[test]
    fn test_menu_state_cycles_up() {
        let mut s = MenuState::new();
        s.up();
        assert_eq!(s.selected, 2);
        s.up();
        assert_eq!(s.selected, 1);
    }

    #[test]
    fn test_menu_state_action_mapping() {
        let mut s = MenuState::new();
        assert_eq!(s.action(), MenuAction::NewGame);
        s.down();
        assert_eq!(s.action(), MenuAction::Rules);
        s.down();
        assert_eq!(s.action(), MenuAction::Quit);
    }
}