//! Zhuo Ji — entry point.

use std::time::{SystemTime, UNIX_EPOCH};

use ember_stdlib::input::Input;
use ember_stdlib::graphics::text::draw_right;
use macroquad::prelude::*;
use ember_stdlib::ui::hit;
    
use zhuo_ji::components::{ClaimKind, Meld, Suit, Tile};
use zhuo_ji::hand;
use zhuo_ji::help::{self, HelpAction, HelpState};
use zhuo_ji::layout::{Seat, TableLayout};
use zhuo_ji::menu::{self, MenuAction, MenuState};
use zhuo_ji::scoring;
use zhuo_ji::tiles;
use zhuo_ji::{Game, GameContext, HuMethod, Phase};

fn window_conf() -> Conf {
    Conf {
        window_title: "Zhuo Ji — 贵阳捉鸡麻将".to_owned(),
        window_width: 1440,
        window_height: 900,
        window_resizable: true,
        high_dpi: true,
        ..Default::default()
    }
}

fn fresh_seed() -> u32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| (d.as_nanos() ^ (d.as_nanos() >> 32)) as u32)
        .unwrap_or(0x2E5B_1234)
}

const TRACKED_KEYS: &[KeyCode] = &[
    KeyCode::Space,
    KeyCode::R,
    KeyCode::Escape,
    KeyCode::H,
    KeyCode::Left,
    KeyCode::Right,
    KeyCode::Up,
    KeyCode::Down,
    KeyCode::Enter,
];

const BTN_W: f32 = 120.0;
const BTN_H: f32 = 50.0;
const BTN_GAP: f32 = 10.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppScreen {
    Menu,
    Game,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MatchEndTab {
    Scores,
    History,
}

fn match_end_tab_rects(layout: &TableLayout) -> [(MatchEndTab, (f32, f32, f32, f32)); 2] {
    let pw = 900.0_f32.min(layout.scr_w - 100.0);
    let ph = 620.0_f32.min(layout.scr_h - 80.0);
    let px = (layout.scr_w - pw) * 0.5;
    let py = (layout.scr_h - ph) * 0.5;
    let cx = px + pw * 0.5;

    let tab_y = py + 76.0;
    let tab_w = 220.0;
    let tab_h = 38.0;
    let tab_gap = 14.0;
    let tab_total = tab_w * 2.0 + tab_gap;
    let tab_x0 = cx - tab_total * 0.5;

    [
        (MatchEndTab::Scores, (tab_x0, tab_y, tab_w, tab_h)),
        (
            MatchEndTab::History,
            (tab_x0 + tab_w + tab_gap, tab_y, tab_w, tab_h),
        ),
    ]
}

fn hit_match_end_tab(layout: &TableLayout, mouse: Vec2) -> Option<MatchEndTab> {
    for (tab, (x, y, w, h)) in match_end_tab_rects(layout) {
        if hit::contains_xywh(x, y, w, h, mouse) {
            return Some(tab);
        }
    }
    None
}

#[macroquad::main(window_conf)]
async fn main() {
    let ctx = GameContext::load();
    let mut game = Game::new(fresh_seed());
    let mut help_state = HelpState::new();
    let mut menu_state = MenuState::new();
    let mut screen = AppScreen::Menu;
    let mut match_end_tab = MatchEndTab::Scores;
    let mut match_over_time: f32 = 0.0;
    let mut last_was_match_over = false;
    let mut hand_end_time: f32 = 0.0;
    let mut last_was_hand_end = false;

    loop {
        let input = Input::from_macroquad_with_keys(TRACKED_KEYS);

        let dt = get_frame_time().min(1.0 / 30.0);
        let layout = TableLayout::new(screen_width(), screen_height());

        // ─── Keyboard ─────────────────────────────────────────────────
        if input.is_key_pressed(KeyCode::Escape) {
            if help_state.visible {
                help_state.close();
            } else if screen == AppScreen::Menu {
                break;
            } else {
                screen = AppScreen::Menu;
            }
        }

        if input.is_key_pressed(KeyCode::H) {
            help_state.toggle();
        }

        if help_state.visible {
            if input.is_key_pressed(KeyCode::Left) {
                help_state.prev_tab();
            }
            if input.is_key_pressed(KeyCode::Right) {
                help_state.next_tab();
            }
        } else if screen == AppScreen::Menu {
            if input.is_key_pressed(KeyCode::Up) {
                menu_state.up();
            }
            if input.is_key_pressed(KeyCode::Down) {
                menu_state.down();
            }
            if input.is_key_pressed(KeyCode::Enter) || input.is_key_pressed(KeyCode::Space) {
                match menu_state.action() {
                    MenuAction::NewGame => {
                        game = Game::new(fresh_seed());
                        match_end_tab = MatchEndTab::Scores;
                        match_over_time = 0.0;
                        last_was_match_over = false;
                        hand_end_time = 0.0;
                        last_was_hand_end = false;
                        screen = AppScreen::Game;
                    }
                    MenuAction::Rules => help_state.open(),
                    MenuAction::Quit => break,
                    MenuAction::None => {}
                }
            }
        }

        // ─── Screen-specific timers ───────────────────────────────────
        if screen == AppScreen::Game {
            let is_mo = matches!(game.phase, Phase::MatchOver { .. });
            if is_mo && !last_was_match_over {
                match_over_time = 0.0;
                match_end_tab = MatchEndTab::Scores;
            } else if is_mo {
                match_over_time += dt;
            }
            last_was_match_over = is_mo;

            let is_hand_end = matches!(game.phase, Phase::Hu { .. } | Phase::HuangZhuang);
            if is_hand_end && !last_was_hand_end {
                hand_end_time = 0.0;
            } else if is_hand_end {
                hand_end_time += dt;
            }
            last_was_hand_end = is_hand_end;
        }

        // ─── Mouse ────────────────────────────────────────────────────
        if input.mouse_left_pressed {
            if help_state.visible {
                match help::hit_test(&layout, &help_state, input.mouse_pos) {
                    HelpAction::Close | HelpAction::GotIt => help_state.close(),
                    HelpAction::Tab(t) => help_state.tab = t,
                    HelpAction::SuitFilter(s) => help_state.suit_filter = s,
                    HelpAction::RankFilter(r) => help_state.rank_filter = r,
                    HelpAction::None => {}
                }
            } else if screen == AppScreen::Menu {
                match menu::hit_test(&layout, input.mouse_pos) {
                    MenuAction::NewGame => {
                        game = Game::new(fresh_seed());
                        match_end_tab = MatchEndTab::Scores;
                        match_over_time = 0.0;
                        last_was_match_over = false;
                        hand_end_time = 0.0;
                        last_was_hand_end = false;
                        screen = AppScreen::Game;
                    }
                    MenuAction::Rules => help_state.open(),
                    MenuAction::Quit => break,
                    MenuAction::None => {}
                }
            } else {
                let is_mo = matches!(game.phase, Phase::MatchOver { .. });
                let is_hand_end =
                    matches!(game.phase, Phase::Hu { .. } | Phase::HuangZhuang);
                if is_mo {
                    if let Some(tab) = hit_match_end_tab(&layout, input.mouse_pos) {
                        match_end_tab = tab;
                    }
                } else if !is_hand_end {
                    if let Some(action) = hit_claim_button(&game, &layout, input.mouse_pos) {
                        match action {
                            ClaimAction::Claim(kind) => { game.human_claim(kind); }
                            ClaimAction::Pass => { game.human_pass(&ctx); }
                        }
                    } else if let Some(action) =
                        hit_own_turn_button(&game, &layout, input.mouse_pos)
                    {
                        match action {
                            OwnAction::Zimo => { game.human_zimo(); }
                            OwnAction::AnGang(t) => { game.human_an_gang(t); }
                        }
                    } else if let Some(idx) = hit_hand_tile(&game, &layout, input.mouse_pos) {
                        game.human_discard(idx, &ctx);
                    }
                }
            }
        }

        // Menu hover updates selection (when menu is active, help closed).
        if screen == AppScreen::Menu && !help_state.visible
            && let Some(idx) = menu::hit_button_idx(&layout, input.mouse_pos)
        {
            menu_state.selected = idx;
        }

        // ─── Game update ──────────────────────────────────────────────
        if screen == AppScreen::Game {
            let game_dt = if help_state.visible { 0.0 } else { dt };
            let _ = game.update(&input, &ctx, game_dt);
        }

        // ─── Draw ─────────────────────────────────────────────────────
        match screen {
            AppScreen::Menu => {
                draw_background(&layout, &ctx);
                menu::draw_menu(&layout, &ctx, &menu_state, input.mouse_pos);
            }
            AppScreen::Game => {
                draw_background(&layout, &ctx);

                draw_wall(&game, &layout, &ctx);
                draw_clock(&game, &layout, &ctx);
                draw_under_clock(&game, &layout, &ctx);

                for seat in Seat::ALL {
                    draw_river(&game, &layout, &ctx, seat);
                }

                for seat in [Seat::West, Seat::North, Seat::East] {
                    draw_ai_hand(&game, &layout, &ctx, seat);
                    draw_ai_melds(&game, &layout, &ctx, seat);
                }

                for seat in Seat::ALL {
                    draw_avatar(&game, &layout, &ctx, seat);
                }

                draw_human_melds(&game, &layout, &ctx);
                draw_human_hand(&game, &layout, &ctx, input.mouse_pos);
                draw_human_drawn(&game, &layout, &ctx, input.mouse_pos);

                draw_info_stack(&game, &layout, &ctx);
                draw_claim_buttons(&game, &layout, &ctx, input.mouse_pos);
                draw_own_turn_buttons(&game, &layout, &ctx, input.mouse_pos);

                if !help_state.has_seen && !help_state.visible {
                    text_centered(
                        "Press H to see the rules",
                        layout.cx(),
                        layout.scr_h - 30.0,
                        14.0,
                        ctx.colors.text_dim,
                    );
                }

                let is_mo = matches!(game.phase, Phase::MatchOver { .. });
                let is_hand_end =
                    matches!(game.phase, Phase::Hu { .. } | Phase::HuangZhuang);
                if is_mo {
                    draw_match_end_overlay(
                        &layout, &ctx, &game, match_end_tab, match_over_time, input.mouse_pos,
                    );
                } else if is_hand_end {
                    draw_hand_end_overlay(&layout, &ctx, &game, hand_end_time);
                }
            }
        }

        draw_premium_border(&layout, &ctx);

        if help_state.visible {
            help::draw_help_overlay(&layout, &ctx, &help_state, input.mouse_pos);
        }

        next_frame().await;
    }
}

// ─── Default-font text helpers ────────────────────────────────────────────

fn text_centered(text: &str, cx: f32, y: f32, size: f32, color: Color) {
    let dim = measure_text(text, None, size as u16, 1.0);
    draw_text(text, cx - dim.width * 0.5, y, size, color);
}

fn text_right(text: &str, rx: f32, y: f32, size: f32, color: Color) {
    draw_right(text, rx, y, size as u16, color);
}

fn signed_color(v: i32, ctx: &GameContext) -> Color {
    if v > 0 {
        Color::new(0.40, 0.90, 0.50, 1.0)
    } else if v < 0 {
        Color::new(0.90, 0.40, 0.35, 1.0)
    } else {
        ctx.colors.text_dim
    }
}

// ─── Hit testing ───────────────────────────────────────────────────────────

fn hit_hand_tile(game: &Game, layout: &TableLayout, mouse: Vec2) -> Option<usize> {
    if !matches!(game.phase, Phase::AwaitingDiscard { player: 0 }) {
        return None;
    }
    let p = &game.players[0];
    let n = p.concealed.len();
    let tw = layout.sizes.hand.w;
    let th = layout.sizes.hand.h;

    for i in 0..n {
        let pos = layout.hand_tile_pos(i, n);
        if hit::contains_xywh(pos.x, pos.y, tw, th, mouse) {
            return Some(i);
        }
    }
    if p.drawn.is_some() {
        let pos = layout.drawn_pos(n);
        if hit::contains_xywh(pos.x, pos.y, tw, th, mouse) {
            return Some(n);
        }
    }
    None
}

enum OwnAction {
    Zimo,
    AnGang(Tile),
}

fn own_turn_actions(game: &Game) -> Vec<(OwnAction, &'static str)> {
    let Phase::AwaitingDiscard { player: 0 } = game.phase else {
        return Vec::new();
    };
    let mut out = Vec::new();
    if game.can_zimo(0) {
        out.push((OwnAction::Zimo, "ZIMO!"));
    }
    if let Some(tile) = game.an_gang_tile(0) {
        out.push((OwnAction::AnGang(tile), "An Gang"));
    }
    out
}

fn own_turn_button_rects(game: &Game, layout: &TableLayout)
    -> Vec<(OwnAction, (f32, f32, f32, f32))>
{
    let actions = own_turn_actions(game);
    if actions.is_empty() { return Vec::new(); }
    let n = actions.len();
    let total = n as f32 * BTN_W + (n - 1) as f32 * BTN_GAP;
    let mut x = layout.buttons_center_x() - total * 0.5;
    let y = layout.buttons_y();
    let mut out = Vec::new();
    for (action, _) in actions {
        out.push((action, (x, y, BTN_W, BTN_H)));
        x += BTN_W + BTN_GAP;
    }
    out
}

fn hit_own_turn_button(game: &Game, layout: &TableLayout, mouse: Vec2) -> Option<OwnAction> {
    for (action, (x, y, w, h)) in own_turn_button_rects(game, layout) {
        if hit::contains_xywh(x, y, w, h, mouse) {
            return Some(action);
        }
    }
    None
}

enum ClaimAction {
    Claim(ClaimKind),
    Pass,
}

fn claim_button_rects(game: &Game, layout: &TableLayout)
    -> Vec<(ClaimAction, (f32, f32, f32, f32))>
{
    let Phase::AwaitingClaims { discard, from, .. } = game.phase else {
        return Vec::new();
    };
    let Some(opts) = game.human_claim_options(discard, from) else {
        return Vec::new();
    };
    let n = opts.len() + 1;
    let total = n as f32 * BTN_W + (n - 1) as f32 * BTN_GAP;
    let mut x = layout.buttons_center_x() - total * 0.5;
    let y = layout.buttons_y();
    let mut out = Vec::new();
    for kind in opts {
        out.push((ClaimAction::Claim(kind), (x, y, BTN_W, BTN_H)));
        x += BTN_W + BTN_GAP;
    }
    out.push((ClaimAction::Pass, (x, y, BTN_W, BTN_H)));
    out
}

fn hit_claim_button(game: &Game, layout: &TableLayout, mouse: Vec2) -> Option<ClaimAction> {
    for (action, (x, y, w, h)) in claim_button_rects(game, layout) {
        if hit::contains_xywh(x, y, w, h, mouse) {
            return Some(action);
        }
    }
    None
}

// ─── Background ───────────────────────────────────────────────────────────

fn draw_background(layout: &TableLayout, ctx: &GameContext) {
    clear_background(ctx.colors.table_bg);

    let w = layout.scr_w;
    let h = layout.scr_h;

    draw_mountain_layer(w, h, ctx.colors.mountain_far, 0.38, 7, 0.14, 71.0);
    draw_cloud_sea(w, h, 0.42, ctx.colors.mist, 0.045);

    draw_mountain_layer(w, h, ctx.colors.mountain_mid, 0.54, 6, 0.17, 133.0);
    draw_cloud_sea(w, h, 0.58, ctx.colors.mist, 0.055);

    draw_mountain_layer(w, h, ctx.colors.mountain_near, 0.72, 5, 0.20, 251.0);
    draw_cloud_sea(w, h, 0.76, ctx.colors.mist, 0.065);

    draw_mountain_layer(w, h, ctx.colors.mountain_fore, 0.92, 4, 0.22, 419.0);
}

fn draw_mountain_layer(
    w: f32,
    h: f32,
    color: Color,
    baseline_frac: f32,
    count: usize,
    height_frac: f32,
    seed: f32,
) {
    let baseline = h * baseline_frac;
    let step = w / (count as f32);
    let max_peak = h * height_frac;
    let shade = darken(color, 0.55);
    let rim = Color::new(
        (color.r * 1.6).min(1.0),
        (color.g * 1.6).min(1.0),
        (color.b * 1.4).min(1.0),
        0.55,
    );

    for i in 0..count {
        let cx = step * (i as f32 + 0.5);
        let peak = max_peak * (0.70 + hash1(seed + i as f32) * 0.55);
        let width = step * (1.10 + hash1(seed + i as f32 + 0.5) * 0.25);
        draw_mountain_silhouette(
            Vec2::new(cx, baseline),
            width,
            peak,
            (color, shade, rim),
            (10, seed + i as f32 * 7.0),
        );
    }

    draw_rectangle(0.0, baseline, w, h - baseline, color);
}

fn draw_mountain_silhouette(
    origin: Vec2,
    width: f32,
    height: f32,
    (color, shade, rim): (Color, Color, Color),
    (segments, seed): (usize, f32),
) {
    let (cx, base_y) = (origin.x, origin.y);
    let half_w = width * 0.5;
    let step = width / segments as f32;

    let mut points: Vec<Vec2> = Vec::with_capacity(segments + 1);
    for i in 0..=segments {
        let t = i as f32 / segments as f32;
        let x = cx - half_w + i as f32 * step;
        let profile = (t * std::f32::consts::PI).sin();
        let noise = (hash1(seed + i as f32) - 0.5) * 0.18;
        let h = height * (profile + noise).max(0.0);
        points.push(Vec2::new(x, base_y - h));
    }

    for i in 0..segments {
        let p0 = points[i];
        let p1 = points[i + 1];
        let b0 = Vec2::new(p0.x, base_y);
        let b1 = Vec2::new(p1.x, base_y);

        let mid_x = (p0.x + p1.x) * 0.5;
        let rel_x = (mid_x - cx) / half_w;
        let mid_h = ((base_y - p0.y) + (base_y - p1.y)) * 0.5;
        let rel_h = if height > 0.0 { mid_h / height } else { 0.0 };

        let t = (rel_x + 0.35 * (1.0 - rel_h)).clamp(0.0, 1.0);
        let shade_f = smoothstep(0.0, 0.6, t);
        let tri_color = lerp_color(color, shade, shade_f);

        draw_triangle(p0, p1, b0, tri_color);
        draw_triangle(p1, b1, b0, tri_color);
    }

    for i in 0..segments {
        draw_line(
            points[i].x, points[i].y,
            points[i + 1].x, points[i + 1].y,
            1.1, rim,
        );
    }
}

fn draw_cloud_sea(w: f32, h: f32, y_frac: f32, color: Color, height_frac: f32) {
    let top_y = h * y_frac;
    let band_h = h * height_frac;
    let steps = 32;
    let step_h = band_h / steps as f32;
    let peak_a = (color.a * 2.0).min(0.55);

    for i in 0..steps {
        let t = i as f32 / (steps - 1) as f32;
        let a = (t * std::f32::consts::PI).sin();
        let y = top_y + i as f32 * step_h;
        let strip = Color::new(color.r, color.g, color.b, peak_a * a);
        draw_rectangle(0.0, y, w, step_h + 0.5, strip);
    }
}

// ─── Utilities ────────────────────────────────────────────────────────────

fn hash1(x: f32) -> f32 {
    let v = (x * 12.9898).sin() * 43_758.547;
    v - v.floor()
}

fn darken(c: Color, factor: f32) -> Color {
    Color::new(c.r * factor, c.g * factor, c.b * factor, c.a)
}

fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    Color::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

// ─── Premium border ───────────────────────────────────────────────────────

fn draw_premium_border(layout: &TableLayout, ctx: &GameContext) {
    let w = layout.scr_w;
    let h = layout.scr_h;
    draw_rectangle_lines(0.0, 0.0, w, h, 8.0, ctx.colors.gold_dark);
    draw_rectangle_lines(8.0, 8.0, w - 16.0, h - 16.0, 2.0, ctx.colors.gold_light);
    let d = 12.0;
    for (x, y) in [(0.0, 0.0), (w, 0.0), (0.0, h), (w, h)] {
        draw_poly(x, y, 4, d, 45.0, ctx.colors.gold_light);
        draw_poly_lines(x, y, 4, d, 45.0, 1.5, ctx.colors.gold_dark);
    }
}

// ─── Wall + clock ─────────────────────────────────────────────────────────

fn draw_wall(game: &Game, layout: &TableLayout, ctx: &GameContext) {
    let wp = layout.wall_pos();
    tiles::draw_wall(
        wp.x, wp.y, layout.wall.x, layout.wall.y,
        game.wall.len(), &ctx.colors,
    );
}

fn clock_value(game: &Game, ctx: &GameContext) -> Option<(f32, f32)> {
    match game.phase {
        Phase::AwaitingDiscard { player: 0 } => Some((game.turn_timer, ctx.layout.turn_window)),
        Phase::AwaitingClaims { t, .. } => Some((t, ctx.layout.claim_window)),
        Phase::AiThinking { t, .. } => Some((t, ctx.layout.ai_think_duration)),
        _ => None,
    }
}

fn clock_arc_color(frac: f32, ctx: &GameContext) -> Color {
    if frac > 0.5 {
        Color::new(0.35, 0.80, 0.45, 1.0)
    } else if frac > 0.2 {
        Color::new(0.95, 0.75, 0.30, 1.0)
    } else {
        ctx.colors.danger
    }
}

fn draw_clock(game: &Game, layout: &TableLayout, ctx: &GameContext) {
    let cp = layout.clock_pos();
    let r = layout.clock_r;

    let side = zhuo_ji::layout::FRAME_SIDE;
    draw_rectangle_lines(
        cp.x - side * 0.5,
        cp.y - side * 0.5,
        side, side,
        1.2,
        ctx.colors.gold_light,
    );

    draw_circle(cp.x, cp.y, r + 3.0, Color::new(0.05, 0.08, 0.06, 0.85));
    draw_circle(cp.x, cp.y, r, Color::new(0.14, 0.18, 0.14, 0.95));
    draw_circle_lines(cp.x, cp.y, r, 2.0, ctx.colors.gold_dark);

    if let Some((remaining, limit)) = clock_value(game, ctx) {
        let frac = (remaining / limit.max(1e-3)).clamp(0.0, 1.0);
        let color = clock_arc_color(frac, ctx);
        draw_arc_ring(cp.x, cp.y, r - 3.0, frac, color, 4.0);
        let secs = remaining.max(0.0).ceil() as i32;
        let s = format!("{secs}");
        text_centered(&s, cp.x + 2.0, cp.y + 11.0, 30.0, Color::new(0.0, 0.0, 0.0, 0.80));
        text_centered(&s, cp.x, cp.y + 10.0, 30.0, color);
    } else {
        text_centered("-", cp.x, cp.y + 8.0, 22.0, ctx.colors.text_dim);
    }

    let angle = seat_angle(game.turn);
    draw_pointer(cp.x, cp.y, angle, r + 6.0, r + 16.0, ctx.colors.highlight);
}

fn seat_angle(player: usize) -> f32 {
    match player {
        0 => std::f32::consts::FRAC_PI_2,
        1 => std::f32::consts::PI,
        2 => -std::f32::consts::FRAC_PI_2,
        3 => 0.0,
        _ => 0.0,
    }
}

fn draw_pointer(cx: f32, cy: f32, angle: f32, r_inner: f32, r_outer: f32, color: Color) {
    let half = 0.22;
    let tip_x = cx + r_outer * angle.cos();
    let tip_y = cy + r_outer * angle.sin();
    let lx = cx + r_inner * (angle + half).cos();
    let ly = cy + r_inner * (angle + half).sin();
    let rx = cx + r_inner * (angle - half).cos();
    let ry = cy + r_inner * (angle - half).sin();
    draw_triangle(Vec2::new(tip_x, tip_y), Vec2::new(lx, ly), Vec2::new(rx, ry), color);
}

fn draw_arc_ring(cx: f32, cy: f32, r: f32, frac: f32, color: Color, thickness: f32) {
    if frac <= 0.0 { return; }
    let start = -std::f32::consts::FRAC_PI_2;
    let span = frac * std::f32::consts::TAU;
    let segments = ((span / (std::f32::consts::TAU / 64.0)).ceil() as i32).max(2);
    for i in 0..segments {
        let t0 = start + span * (i as f32 / segments as f32);
        let t1 = start + span * ((i + 1) as f32 / segments as f32);
        let p0 = Vec2::new(cx + r * t0.cos(), cy + r * t0.sin());
        let p1 = Vec2::new(cx + r * t1.cos(), cy + r * t1.sin());
        draw_line(p0.x, p0.y, p1.x, p1.y, thickness, color);
    }
}

fn draw_under_clock(game: &Game, layout: &TableLayout, ctx: &GameContext) {
    let cx = layout.cx();
    let base_y = layout.cy() + layout.clock_r + 28.0;

    if matches!(game.phase, Phase::MatchOver { .. }) {
        text_centered(
            "Space: play more    R: new match    Escape: menu",
            cx, base_y, 16.0, ctx.colors.highlight,
        );
    }
}

// ─── Rivers ────────────────────────────────────────────────────────────────

fn draw_river(game: &Game, layout: &TableLayout, ctx: &GameContext, seat: Seat) {
    let p = &game.players[seat.idx()];
    for (i, &t) in p.discards.iter().enumerate() {
        let pos = layout.river_tile_pos(seat, i);
        tiles::draw_tile_flat(
            t, pos.x, pos.y, layout.sizes.river,
            &ctx.colors, &ctx.font,
        );
    }
}

// ─── AI hands + melds ─────────────────────────────────────────────────────

fn draw_ai_hand(game: &Game, layout: &TableLayout, ctx: &GameContext, seat: Seat) {
    let reveal = matches!(
        game.phase,
        Phase::MatchOver { .. } | Phase::Hu { .. } | Phase::HuangZhuang
    );
    let p = &game.players[seat.idx()];
    let concealed = p.concealed.len();
    for i in 0..concealed {
        let pos = layout.ai_back_pos(seat, i, concealed);
        if reveal {
            tiles::draw_tile_standing(
                p.concealed[i], pos.x, pos.y, layout.sizes.ai_back,
                &ctx.colors, &ctx.font,
            );
        } else {
            tiles::draw_tile_back_standing(pos.x, pos.y, layout.sizes.ai_back);
        }
    }
    if let Some(drawn) = p.drawn {
        let pos = layout.ai_drawn_pos(seat, concealed);
        if reveal {
            tiles::draw_tile_standing(
                drawn, pos.x, pos.y, layout.sizes.ai_back,
                &ctx.colors, &ctx.font,
            );
        } else {
            tiles::draw_tile_back_standing(pos.x, pos.y, layout.sizes.ai_back);
            draw_drawn_marker(seat, pos, layout.sizes.ai_back, &ctx.colors);
        }
    }
}

fn draw_ai_melds(game: &Game, layout: &TableLayout, ctx: &GameContext, seat: Seat) {
    let p = &game.players[seat.idx()];
    if p.melds.is_empty() { return; }
    let concealed = p.concealed.len();
    let t = layout.sizes.ai_meld;

    // 1) Compute the layout of all melds for this seat.
    let is_gang: Vec<bool> = p.melds
        .iter()
        .map(|m| matches!(m, Meld::Gang { .. }))
        .collect();
    let origin = layout.ai_melds_origin(seat, concealed, p.melds.len());
    let layout_positions = layout.melds_row_layout(origin, &is_gang);

    // 2) Pass A: draw all row bases first. This avoids shadows from
    //    adjacent tiles falling on already-drawn faces.
    for slots in &layout_positions.row {
        for &pos in slots {
            tiles::draw_tile_standing_base(pos.x, pos.y, t);
        }
    }

    // 3) Pass B: draw all row faces.
    for (mi, m) in p.melds.iter().enumerate() {
        let tile = match m {
            Meld::Peng { tile, .. } => *tile,
            Meld::Gang { tile, .. } => *tile,
        };
        for &pos in &layout_positions.row[mi] {
            tiles::draw_tile_standing_face(tile, pos.x, pos.y, t, &ctx.colors, &ctx.font);
        }
    }

    // 4) Pass C: for Gangs, draw the stacked 4th tile on top.
    for (mi, m) in p.melds.iter().enumerate() {
        if !matches!(m, Meld::Gang { .. }) { continue; }
        let Some(pos) = layout_positions.stacks[mi] else { continue; };
        let tile = match m {
            Meld::Gang { tile, .. } => *tile,
            _ => unreachable!(),
        };
        tiles::draw_tile_standing_base(pos.x, pos.y, t);
        tiles::draw_tile_standing_face(tile, pos.x, pos.y, t, &ctx.colors, &ctx.font);
    }
}

fn draw_drawn_marker(
    seat: Seat,
    pos: Vec2,
    size: zhuo_ji::layout::TileSize,
    colors: &zhuo_ji::config::Colors,
) {
    let cx = pos.x + size.w * 0.5;
    let cy = pos.y + size.h * 0.5;
    let off = size.h * 0.5 + 10.0;
    let (mx, my) = match seat {
        Seat::North => (cx, cy + off),
        Seat::South => (cx, cy - off),
        Seat::West => (cx + off, cy),
        Seat::East => (cx - off, cy),
    };
    let (ax, ay) = match seat {
        Seat::North => (0.0, -1.0),
        Seat::South => (0.0, 1.0),
        Seat::West => (-1.0, 0.0),
        Seat::East => (1.0, 0.0),
    };
    let px = -ay;
    let py = ax;
    let s = 7.0;
    draw_triangle(
        Vec2::new(mx + ax * s, my + ay * s),
        Vec2::new(mx - ax * s + px * s, my - ay * s + py * s),
        Vec2::new(mx - ax * s - px * s, my - ay * s - py * s),
        colors.highlight,
    );
}

// ─── Human melds, hand, drawn ─────────────────────────────────────────────

fn draw_human_melds(game: &Game, layout: &TableLayout, ctx: &GameContext) {
    let melds = &game.players[0].melds;
    if melds.is_empty() { return; }
    let t = layout.sizes.ai_meld;
    let hand_n = game.players[0].concealed.len();

    let is_gang: Vec<bool> = melds
        .iter()
        .map(|m| matches!(m, Meld::Gang { .. }))
        .collect();
    let origin = layout.human_melds_origin(hand_n, melds.len());
    let layout_positions = layout.melds_row_layout(origin, &is_gang);

    // Row bases.
    for slots in &layout_positions.row {
        for &pos in slots {
            tiles::draw_tile_standing_base(pos.x, pos.y, t);
        }
    }
    // Row faces.
    for (mi, m) in melds.iter().enumerate() {
        let tile = match m {
            Meld::Peng { tile, .. } => *tile,
            Meld::Gang { tile, .. } => *tile,
        };
        for &pos in &layout_positions.row[mi] {
            tiles::draw_tile_standing_face(tile, pos.x, pos.y, t, &ctx.colors, &ctx.font);
        }
    }
    // Stacked gang tiles.
    for (mi, m) in melds.iter().enumerate() {
        if !matches!(m, Meld::Gang { .. }) { continue; }
        let Some(pos) = layout_positions.stacks[mi] else { continue; };
        let tile = match m {
            Meld::Gang { tile, .. } => *tile,
            _ => unreachable!(),
        };
        tiles::draw_tile_standing_base(pos.x, pos.y, t);
        tiles::draw_tile_standing_face(tile, pos.x, pos.y, t, &ctx.colors, &ctx.font);
    }
}

/// `true` if the tile at index `i` has at least one "friend" in the
/// hand — a twin (pair+), or a same-suit neighbour within 2 ranks
/// (partial sequence).
fn tile_is_useful(hand: &[Tile], i: usize) -> bool {
    let t = hand[i];
    for (j, &other) in hand.iter().enumerate() {
        if j == i { continue; }
        if other == t { return true; }
        if other.suit != t.suit { continue; }
        let d = (other.rank as i32 - t.rank as i32).abs();
        if d == 1 || d == 2 { return true; }
    }
    false
}

/// `true` if `tile` has a friend in `hand` (pair or ±2 ranks).
fn tile_has_friend_in(hand: &[Tile], tile: Tile) -> bool {
    for &other in hand {
        if other == tile { return true; }
        if other.suit != tile.suit { continue; }
        let d = (other.rank as i32 - tile.rank as i32).abs();
        if d == 1 || d == 2 { return true; }
    }
    false
}

/// Computes the status glow for a tile in the concealed hand.
/// Priority: Ji > Useful.
fn glow_for_concealed(
    hand: &[Tile],
    i: usize,
    ji: Option<&scoring::JiInfo>,
) -> tiles::TileGlow {
    let t = hand[i];
    if let Some(j) = ji
        && scoring::ji_fan_for_tile(t, j) > 0
    {
        return tiles::TileGlow::Ji;
    }
    if tile_is_useful(hand, i) {
        return tiles::TileGlow::Useful;
    }
    tiles::TileGlow::None
}

/// Computes the status glow for the drawn tile, using the concealed
/// hand as context. Priority: Ji > Useful.
fn glow_for_drawn(
    hand: &[Tile],
    drawn: Tile,
    ji: Option<&scoring::JiInfo>,
) -> tiles::TileGlow {
    if let Some(j) = ji
        && scoring::ji_fan_for_tile(drawn, j) > 0
    {
        return tiles::TileGlow::Ji;
    }
    if tile_has_friend_in(hand, drawn) {
        return tiles::TileGlow::Useful;
    }
    tiles::TileGlow::None
}

fn draw_human_hand(game: &Game, layout: &TableLayout, ctx: &GameContext, mouse: Vec2) {
    let discardable = matches!(game.phase, Phase::AwaitingDiscard { player: 0 });
    let n = game.players[0].concealed.len();
    let tw = layout.sizes.hand.w;
    let th = layout.sizes.hand.h;
    let ji = game.last_ji.as_ref();

    for (i, &tile) in game.players[0].concealed.iter().enumerate() {
        let pos = layout.hand_tile_pos(i, n);
        let hovered = discardable && hit::contains_xywh(pos.x, pos.y, tw, th, mouse);
        if hovered {
            tiles::draw_tile_standing_highlighted(
                tile, pos.x, pos.y, layout.sizes.hand,
                &ctx.colors, &ctx.font,
            );
        } else {
            let glow = glow_for_concealed(&game.players[0].concealed, i, ji);
            tiles::draw_tile_standing_with_glow(
                tile, pos.x, pos.y, layout.sizes.hand, glow,
                &ctx.colors, &ctx.font,
            );
        }
    }
}

fn draw_human_drawn(game: &Game, layout: &TableLayout, ctx: &GameContext, mouse: Vec2) {
    let Some(drawn) = game.players[0].drawn else { return; };
    let discardable = matches!(game.phase, Phase::AwaitingDiscard { player: 0 });
    let n = game.players[0].concealed.len();
    let pos = layout.drawn_pos(n);
    let tw = layout.sizes.hand.w;
    let th = layout.sizes.hand.h;

    let hovered = discardable && hit::contains_xywh(pos.x, pos.y, tw, th, mouse);
    if hovered {
        tiles::draw_tile_standing_highlighted(
            drawn, pos.x, pos.y, layout.sizes.hand,
            &ctx.colors, &ctx.font,
        );
    } else {
        let glow = glow_for_drawn(
            &game.players[0].concealed,
            drawn,
            game.last_ji.as_ref(),
        );
        tiles::draw_tile_standing_with_glow(
            drawn, pos.x, pos.y, layout.sizes.hand, glow,
            &ctx.colors, &ctx.font,
        );
    }

    let mx = pos.x + tw * 0.5;
    let my = pos.y - 8.0;
    draw_triangle(
        Vec2::new(mx - 7.0, my - 7.0),
        Vec2::new(mx + 7.0, my - 7.0),
        Vec2::new(mx, my),
        ctx.colors.highlight,
    );
}

// ─── Avatars ───────────────────────────────────────────────────────────────

fn draw_avatar(game: &Game, layout: &TableLayout, ctx: &GameContext, seat: Seat) {
    let pos = layout.avatar_pos(seat);
    let size = layout.avatar;
    let is_current = game.turn == seat.idx();

    let fill = avatar_color(seat.idx());
    draw_rectangle(pos.x, pos.y, size, size, fill);
    let border_color = if is_current { ctx.colors.highlight } else { Color::new(0.08, 0.08, 0.08, 1.0) };
    let border_w = if is_current { 4.0 } else { 2.0 };
    draw_rectangle_lines(pos.x, pos.y, size, size, border_w, border_color);

    let label = avatar_initial(seat.idx());
    text_centered(label, pos.x + size * 0.5, pos.y + size * 0.62, 28.0, WHITE);

    let p = &game.players[seat.idx()];
    let tile_count = p.concealed.len() + p.drawn.is_some() as usize;
    let name = player_name(seat.idx());
    let score_str = format!("{}", p.score);
    let count_str = format!("{tile_count}");

    match seat {
        Seat::West | Seat::East => {
            let cxi = pos.x + size * 0.5;
            let ny = pos.y + size + 18.0;
            text_centered(name, cxi, ny, 16.0, ctx.colors.text);
            text_centered(&count_str, cxi, ny + 16.0, 14.0, ctx.colors.text_dim);
            text_centered(&score_str, cxi, ny + 36.0, 20.0, ctx.colors.text);
        }
        Seat::North => {
            let nx = pos.x + size + 12.0;
            let ny = pos.y + size * 0.35;
            draw_text(name, nx, ny, 16.0, ctx.colors.text);
            draw_text(&count_str, nx, ny + 18.0, 14.0, ctx.colors.text_dim);
            draw_text(&score_str, nx, ny + 40.0, 20.0, ctx.colors.text);
        }
        Seat::South => {
            let nx = pos.x - 12.0;
            let ny = pos.y + size * 0.35;
            text_right(name, nx, ny, 16.0, ctx.colors.text);
            text_right(&count_str, nx, ny + 18.0, 14.0, ctx.colors.text_dim);
            text_right(&score_str, nx, ny + 40.0, 20.0, ctx.colors.highlight);
        }
    }
}

fn avatar_color(idx: usize) -> Color {
    match idx {
        0 => Color::new(0.30, 0.55, 0.90, 1.0),
        1 => Color::new(0.88, 0.35, 0.35, 1.0),
        2 => Color::new(0.95, 0.75, 0.30, 1.0),
        3 => Color::new(0.35, 0.75, 0.45, 1.0),
        _ => WHITE,
    }
}

fn avatar_initial(idx: usize) -> &'static str {
    match idx {
        0 => "Y", 1 => "1", 2 => "2", 3 => "3", _ => "?",
    }
}

// ─── Info stack / buttons ─────────────────────────────────────────────────

fn phase_label(game: &Game) -> String {
    match game.phase {
        Phase::Deal { .. } => "Dealing...".into(),
        Phase::AwaitingDraw { .. } => "Drawing...".into(),
        Phase::DrawAnim { .. } => "Drawing...".into(),
        Phase::AiThinking { .. } => "Thinking...".into(),
        Phase::AwaitingDiscard { player: 0 } => "Your discard".into(),
        Phase::AwaitingDiscard { .. } => "Discarding...".into(),
        Phase::AwaitingClaims { t, .. } => format!("Claims... ({:.1}s)", t.max(0.0)),
        Phase::ClaimAnim { .. } => "Claim...".into(),
        Phase::Hu { winner, .. } => {
            if winner == 0 { "You win! (Space)".into() }
            else { format!("{} wins (Space)", player_name(winner)) }
        }
        Phase::HuangZhuang => "Wall empty (Space)".into(),
        Phase::MatchOver { winner } => {
            if winner == 0 { "MATCH - You win!".into() }
            else { format!("MATCH - {} wins", player_name(winner)) }
        }
    }
}

fn draw_info_stack(game: &Game, layout: &TableLayout, ctx: &GameContext) {
    let info = layout.info_pos();

    if matches!(game.phase, Phase::MatchOver { .. } | Phase::HuangZhuang) {
        let mut cy = info.y;
        draw_text(
            format!("Turn: {}", player_name(game.turn)),
            info.x, cy, 18.0, ctx.colors.text,
        );
        cy += 24.0;
        draw_text(phase_label(game), info.x, cy, 16.0, ctx.colors.text_dim);
        return;
    }

    let concealed = &game.players[0].concealed;
    let melds = &game.players[0].melds;
    let s = hand::shanten(concealed, melds);

    let (status, status_color) = if s < 0 {
        ("Ready".to_string(), Color::new(0.40, 0.90, 0.50, 1.0))
    } else if s == 0 {
        ("TENPAI".to_string(), Color::new(0.40, 0.90, 0.50, 1.0))
    } else if s == 1 {
        ("1-shanten".to_string(), ctx.colors.text)
    } else {
        (format!("{s}-shanten"), ctx.colors.text_dim)
    };

    let mut cy = info.y;
    draw_text(&status, info.x, cy, 22.0, status_color);
    let status_w = measure_text(&status, None, 22, 1.0).width;
    let mut inline_x = info.x + status_w + 14.0;

    if s == 0 {
        let waits = waiting_tiles(concealed, melds);
        if !waits.is_empty() {
            let sep = " | ";
            draw_text(sep, inline_x, cy, 16.0, ctx.colors.text_dim);
            inline_x += measure_text(sep, None, 16, 1.0).width;
            let lbl = "Waits:";
            draw_text(lbl, inline_x, cy, 16.0, ctx.colors.text_dim);
            inline_x += measure_text(lbl, None, 16, 1.0).width + 6.0;
            for t in waits {
                let tl = t.label();
                draw_text(&tl, inline_x, cy, 17.0, ctx.colors.highlight);
                inline_x += measure_text(&tl, None, 17, 1.0).width + 6.0;
            }
        }
    }
    cy += 26.0;

    let hand_str = format!(
        "Hand {} / {}",
        (game.hands_played + 1).min(game.match_length),
        game.match_length
    );
    draw_text(&hand_str, info.x, cy, 15.0, ctx.colors.text_dim);
    let hand_w = measure_text(&hand_str, None, 15, 1.0).width;

    let sep = " | ";
    let sep_x = info.x + hand_w + 2.0;
    draw_text(sep, sep_x, cy, 15.0, ctx.colors.text_dim);
    let sep_w = measure_text(sep, None, 15, 1.0).width;

    let turn_str = format!("Turn: {}", player_name(game.turn));
    draw_text(&turn_str, sep_x + sep_w, cy, 15.0, ctx.colors.text);
    cy += 20.0;

    draw_text(phase_label(game), info.x, cy, 14.0, ctx.colors.text_dim);
    cy += 18.0;

    if let Some(ji) = game.last_ji
        && ji.primary.rank > 0
    {
        draw_text("Ji:", info.x, cy, 14.0, ctx.colors.text_dim);
        let jw = measure_text("Ji:", None, 14, 1.0).width;
        let label = ji.primary.label();
        draw_text(&label, info.x + jw + 6.0, cy, 14.0, ctx.colors.highlight);
    }
}

fn waiting_tiles(concealed: &[Tile], melds: &[Meld]) -> Vec<Tile> {
    let mut waits = Vec::new();
    for suit in Suit::ALL {
        for rank in 1..=9 {
            let t = Tile::new(suit, rank);
            let mut test = concealed.to_vec();
            test.push(t);
            test.sort();
            if hand::is_winning_hand(&test, melds) {
                waits.push(t);
            }
        }
    }
    waits
}

fn draw_claim_buttons(game: &Game, layout: &TableLayout, ctx: &GameContext, mouse: Vec2) {
    if let Phase::AwaitingClaims { t, .. } = game.phase {
        let frac = (t / ctx.layout.claim_window).clamp(0.0, 1.0);
        let bar_w = 330.0;
        let bar_h = 6.0;
        let bar_x = layout.cx() - bar_w * 0.5;
        let bar_y = layout.buttons_y() - 16.0;
        draw_rectangle(bar_x, bar_y, bar_w, bar_h, Color::new(0.10, 0.10, 0.10, 0.85));
        let fill_color = if frac > 0.5 {
            Color::new(0.35, 0.80, 0.45, 1.0)
        } else if frac > 0.2 {
            Color::new(0.95, 0.75, 0.30, 1.0)
        } else {
            ctx.colors.danger
        };
        draw_rectangle(bar_x, bar_y, bar_w * frac, bar_h, fill_color);
        draw_rectangle_lines(bar_x, bar_y, bar_w, bar_h, 1.0, ctx.colors.gold_dark);
    }

    for (action, (x, y, w, h)) in claim_button_rects(game, layout) {
        let hovered = hit::contains_xywh(x, y, w, h, mouse);
        let is_pass = matches!(action, ClaimAction::Pass);
        let bg = if hovered {
            if is_pass { ctx.colors.danger } else { ctx.colors.highlight }
        } else if is_pass {
            Color::new(0.38, 0.22, 0.22, 1.0)
        } else {
            ctx.colors.tile_face
        };
        draw_rectangle(x, y, w, h, bg);
        draw_rectangle_lines(x, y, w, h, 3.0, ctx.colors.tile_text);
        let label = match action {
            ClaimAction::Claim(ClaimKind::Peng) => "PENG",
            ClaimAction::Claim(ClaimKind::Gang) => "GANG",
            ClaimAction::Claim(ClaimKind::Hu) => "HU!",
            ClaimAction::Pass => "PASS",
        };
        text_centered(label, x + w * 0.5, y + h * 0.5 + 10.0, 28.0, ctx.colors.tile_text);
    }
}

fn draw_own_turn_buttons(game: &Game, layout: &TableLayout, ctx: &GameContext, mouse: Vec2) {
    for (action, (x, y, w, h)) in own_turn_button_rects(game, layout) {
        let hovered = hit::contains_xywh(x, y, w, h, mouse);
        let bg = if hovered { ctx.colors.highlight } else { ctx.colors.tile_face };
        draw_rectangle(x, y, w, h, bg);
        draw_rectangle_lines(x, y, w, h, 3.0, ctx.colors.tile_text);
        let label = match action {
            OwnAction::Zimo => "ZIMO!",
            OwnAction::AnGang(_) => "An Gang",
        };
        text_centered(label, x + w * 0.5, y + h * 0.5 + 8.0, 24.0, ctx.colors.tile_text);
    }
}

// ─── Hand-end modal (Hu / HuangZhuang) ────────────────────────────────────

fn draw_hand_end_overlay(layout: &TableLayout, ctx: &GameContext, game: &Game, t: f32) {
    let alpha = (t / 0.25).clamp(0.0, 1.0);

    draw_rectangle(
        0.0, 0.0, layout.scr_w, layout.scr_h,
        Color::new(0.02, 0.05, 0.04, 0.80 * alpha),
    );

    let pw = 800.0_f32.min(layout.scr_w - 80.0);
    let ph = 480.0_f32.min(layout.scr_h - 80.0);
    let px = (layout.scr_w - pw) * 0.5;
    let py = (layout.scr_h - ph) * 0.5;
    let cx = px + pw * 0.5;

    draw_rectangle(px, py, pw, ph, Color::new(0.10, 0.18, 0.13, alpha));
    draw_rectangle_lines(
        px, py, pw, ph, 3.0,
        Color::new(
            ctx.colors.gold_dark.r, ctx.colors.gold_dark.g,
            ctx.colors.gold_dark.b, alpha,
        ),
    );
    draw_rectangle_lines(
        px + 4.0, py + 4.0, pw - 8.0, ph - 8.0, 1.5,
        Color::new(
            ctx.colors.gold_light.r, ctx.colors.gold_light.g,
            ctx.colors.gold_light.b, alpha,
        ),
    );

    for (dx, dy) in [
        (px + 12.0, py + 12.0),
        (px + pw - 12.0, py + 12.0),
        (px + 12.0, py + ph - 12.0),
        (px + pw - 12.0, py + ph - 12.0),
    ] {
        draw_poly(dx, dy, 4, 8.0, 45.0, ctx.colors.gold_light);
    }

    let title = match game.phase {
        Phase::Hu { winner, method } => {
            let who = if winner == 0 {
                "YOU WIN".to_string()
            } else {
                format!("{} WINS", player_name(winner).to_uppercase())
            };
            let how = match method {
                HuMethod::Zimo => "self-draw",
                HuMethod::Hu { .. } => "on discard",
            };
            format!("{who}  ({how})")
        }
        Phase::HuangZhuang => "DRAW — no winner".to_string(),
        _ => "HAND OVER".to_string(),
    };
    text_centered(&title, cx, py + 50.0, 26.0, ctx.colors.highlight);

    draw_line(
        px + 40.0, py + 70.0,
        px + pw - 40.0, py + 70.0,
        1.0, ctx.colors.gold_dark,
    );

    let row_x = px + 40.0;
    let row_w = pw - 80.0;
    let header_y = py + 96.0;

    // Column right-edges.
    let col_total = row_x + row_w - 20.0;
    let col_delta = col_total - 100.0;
    let col_ji = col_delta - 90.0;
    let col_dou = col_ji - 70.0;
    let col_hu = col_dou - 70.0;

    // Headers.
    draw_text("Player", row_x + 50.0, header_y, 13.0, ctx.colors.text_dim);
    draw_text("Status", row_x + 140.0, header_y, 13.0, ctx.colors.text_dim);
    text_right("Hu", col_hu, header_y, 13.0, ctx.colors.text_dim);
    text_right("Dou", col_dou, header_y, 13.0, ctx.colors.text_dim);
    text_right("Ji", col_ji, header_y, 13.0, ctx.colors.text_dim);
    text_right("Delta", col_delta, header_y, 13.0, ctx.colors.text_dim);
    text_right("Total", col_total, header_y, 13.0, ctx.colors.text_dim);

    let deltas = game.last_hand_deltas().unwrap_or([0; 4]);
    let bd_opt = game.last_breakdown;
    let row_h = 56.0;
    let row_y0 = py + 112.0;

    for (row_i, seat_idx) in [0usize, 1, 2, 3].into_iter().enumerate() {
        let y = row_y0 + row_i as f32 * row_h;
        let is_winner = matches!(game.phase, Phase::Hu { winner, .. } if winner == seat_idx);
        let p = &game.players[seat_idx];

        let bg = if is_winner {
            Color::new(0.25, 0.20, 0.08, alpha)
        } else {
            Color::new(0.06, 0.12, 0.09, alpha * 0.7)
        };
        draw_rectangle(row_x, y, row_w, row_h - 8.0, bg);
        if is_winner {
            draw_rectangle_lines(row_x, y, row_w, row_h - 8.0, 2.0, ctx.colors.gold_light);
        }

        draw_circle(
            row_x + 26.0,
            y + (row_h - 8.0) * 0.5,
            11.0,
            avatar_color(seat_idx),
        );

        let name_color = if seat_idx == 0 {
            ctx.colors.highlight
        } else {
            ctx.colors.text
        };
        draw_text(player_name(seat_idx), row_x + 50.0, y + 30.0, 20.0, name_color);

        if is_winner {
            let pattern = scoring::detect_pattern(&p.concealed, &p.melds);
            let label = format!("{}  ({} fan)", pattern.label(), pattern.fan());
            draw_text(&label, row_x + 140.0, y + 30.0, 16.0, ctx.colors.highlight);
        } else {
            let all = p.all_tiles();
            let s = hand::shanten(&all, &p.melds);
            let status = if s < 0 {
                "Ready".to_string()
            } else if s == 0 {
                "Tenpai".to_string()
            } else {
                format!("{s}-shanten")
            };
            draw_text(&status, row_x + 140.0, y + 30.0, 15.0, ctx.colors.text_dim);
        }

        // Hu / Dou / Ji columns.
        if let Some(bd) = bd_opt {
            let hu = bd.hu[seat_idx];
            let dou = bd.dou[seat_idx];
            let ji = bd.ji[seat_idx];
            let hu_col = signed_color(hu, ctx);
            let dou_col = signed_color(dou, ctx);
            let ji_col = signed_color(ji, ctx);
            text_right(&format!("{hu:+}"), col_hu, y + 30.0, 16.0, hu_col);
            text_right(&format!("{dou:+}"), col_dou, y + 30.0, 16.0, dou_col);
            text_right(&format!("{ji:+}"), col_ji, y + 30.0, 16.0, ji_col);
        } else {
            text_right("0", col_hu, y + 30.0, 14.0, ctx.colors.text_dim);
            text_right("0", col_dou, y + 30.0, 14.0, ctx.colors.text_dim);
            text_right("0", col_ji, y + 30.0, 14.0, ctx.colors.text_dim);
        }

        let d = deltas[seat_idx];
        let d_col = if d > 0 {
            Color::new(0.40, 0.90, 0.50, alpha)
        } else if d < 0 {
            Color::new(0.90, 0.40, 0.35, alpha)
        } else {
            Color::new(0.70, 0.70, 0.70, alpha)
        };
        text_right(&format!("{d:+}"), col_delta, y + 30.0, 18.0, d_col);

        let total = p.score;
        let t_col = if total > 0 {
            Color::new(0.60, 0.95, 0.70, alpha)
        } else if total < 0 {
            Color::new(0.95, 0.60, 0.55, alpha)
        } else {
            Color::new(0.85, 0.85, 0.85, alpha)
        };
        text_right(&format!("{total:+}"), col_total, y + 30.0, 22.0, t_col);
    }

    // Explanatory note.
    let note_y = py + 380.0;
    draw_text(
        "> Ji (chickens) are counted separately for every player.",
        row_x, note_y, 14.0, ctx.colors.text_dim,
    );
    draw_text(
        "  A player with many Ji can earn more than the winner.",
        row_x, note_y + 20.0, 14.0, ctx.colors.text_dim,
    );

    text_centered(
        "Space: next hand    H: rules    Escape: menu",
        cx,
        py + ph - 24.0,
        15.0,
        ctx.colors.text_dim,
    );
}

// ─── Match-end modal (2 tabs) ─────────────────────────────────────────────

fn draw_match_end_overlay(
    layout: &TableLayout,
    ctx: &GameContext,
    game: &Game,
    tab: MatchEndTab,
    t: f32,
    mouse: Vec2,
) {
    let alpha = (t / 0.3).clamp(0.0, 1.0);
    draw_rectangle(
        0.0, 0.0, layout.scr_w, layout.scr_h,
        Color::new(0.02, 0.05, 0.04, 0.90 * alpha),
    );

    let pw = 900.0_f32.min(layout.scr_w - 100.0);
    let ph = 620.0_f32.min(layout.scr_h - 80.0);
    let px = (layout.scr_w - pw) * 0.5;
    let py = (layout.scr_h - ph) * 0.5;
    let cx = px + pw * 0.5;

    draw_rectangle(px, py, pw, ph, Color::new(0.10, 0.18, 0.13, alpha));
    draw_rectangle_lines(
        px, py, pw, ph, 3.0,
        Color::new(
            ctx.colors.gold_dark.r, ctx.colors.gold_dark.g,
            ctx.colors.gold_dark.b, alpha,
        ),
    );
    draw_rectangle_lines(
        px + 4.0, py + 4.0, pw - 8.0, ph - 8.0, 1.5,
        Color::new(
            ctx.colors.gold_light.r, ctx.colors.gold_light.g,
            ctx.colors.gold_light.b, alpha,
        ),
    );

    for (dx, dy) in [
        (px + 12.0, py + 12.0),
        (px + pw - 12.0, py + 12.0),
        (px + 12.0, py + ph - 12.0),
        (px + pw - 12.0, py + ph - 12.0),
    ] {
        draw_poly(dx, dy, 4, 10.0, 45.0, ctx.colors.gold_light);
    }

    let title = if let Phase::MatchOver { winner } = game.phase {
        if winner == 0 {
            "YOU WIN THE MATCH".to_string()
        } else {
            format!("{} WINS THE MATCH", player_name(winner).to_uppercase())
        }
    } else {
        "MATCH OVER".to_string()
    };
    let pulse = 1.0 + 0.05 * (t * 3.0).sin();
    text_centered(&title, cx, py + 50.0, 30.0 * pulse, ctx.colors.highlight);

    for (tab_kind, (tx, ty, tw, th)) in match_end_tab_rects(layout) {
        let active = tab_kind == tab;
        let hovered = hit::contains_xywh(tx, ty, tw, th, mouse);
        let bg = if active {
            ctx.colors.gold_dark
        } else if hovered {
            Color::new(0.20, 0.30, 0.24, 1.0)
        } else {
            Color::new(0.13, 0.22, 0.17, 1.0)
        };
        draw_rectangle(tx, ty, tw, th, bg);
        let border = if active {
            ctx.colors.gold_light
        } else {
            Color::new(0.35, 0.45, 0.40, 1.0)
        };
        draw_rectangle_lines(tx, ty, tw, th, 2.0, border);
        let label = match tab_kind {
            MatchEndTab::Scores => "Scores",
            MatchEndTab::History => "Hand history",
        };
        let txt = if active { ctx.colors.tile_text } else { ctx.colors.text };
        text_centered(label, tx + tw * 0.5, ty + th * 0.5 + 7.0, 18.0, txt);
    }

    let content_y = py + 76.0 + 38.0 + 30.0;
    match tab {
        MatchEndTab::Scores => draw_match_scores(ctx, game, px, content_y, pw, t),
        MatchEndTab::History => draw_match_history(ctx, game, px, content_y, pw),
    }

    text_centered(
        "Space: play 16 more    R: new match    Escape: menu",
        cx,
        py + ph - 28.0,
        15.0,
        ctx.colors.text_dim,
    );
}

fn draw_match_scores(
    ctx: &GameContext,
    game: &Game,
    panel_x: f32,
    top_y: f32,
    panel_w: f32,
    t: f32,
) {
    let mut ranked: Vec<(usize, i32)> = (0..4).map(|i| (i, game.players[i].score)).collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1));

    let cx = panel_x + panel_w * 0.5;
    let row_h = 64.0;
    let row_w = panel_w - 140.0;
    let row_x = cx - row_w * 0.5;

    for (rank, &(player, score)) in ranked.iter().enumerate() {
        let y = top_y + 20.0 + rank as f32 * row_h;
        let alpha = ((t - rank as f32 * 0.18) / 0.4).clamp(0.0, 1.0);

        let bg = if rank == 0 {
            Color::new(0.25, 0.20, 0.08, alpha)
        } else {
            Color::new(0.08, 0.14, 0.10, alpha)
        };
        draw_rectangle(row_x, y, row_w, row_h - 8.0, bg);

        let rank_str = format!("#{}", rank + 1);
        let rank_color = if rank == 0 {
            ctx.colors.gold_light
        } else {
            ctx.colors.text_dim
        };
        draw_text(&rank_str, row_x + 20.0, y + 36.0, 22.0,
            Color::new(rank_color.r, rank_color.g, rank_color.b, alpha));

        let name_color = if player == 0 {
            ctx.colors.highlight
        } else {
            ctx.colors.text
        };
        draw_text(player_name(player), row_x + 90.0, y + 36.0, 22.0,
            Color::new(name_color.r, name_color.g, name_color.b, alpha));

        let score_color = if score > 0 {
            Color::new(0.40, 0.90, 0.50, alpha)
        } else if score < 0 {
            Color::new(0.90, 0.40, 0.35, alpha)
        } else {
            Color::new(0.80, 0.80, 0.80, alpha)
        };
        let score_str = format!("{score:+}");
        text_right(&score_str, row_x + row_w - 20.0, y + 36.0, 26.0, score_color);
    }
}

fn draw_match_history(
    ctx: &GameContext,
    game: &Game,
    panel_x: f32,
    top_y: f32,
    panel_w: f32,
) {
    let cx = panel_x + panel_w * 0.5;
    let col_name_w = 100.0;
    let col_w = (panel_w - col_name_w - 80.0) / 4.0;
    let header_y = top_y + 20.0;
    let header_x = cx - (col_name_w + col_w * 4.0) * 0.5;

    draw_text("Hand", header_x + 20.0, header_y, 15.0, ctx.colors.text_dim);
    for i in 0..4 {
        let hx = header_x + col_name_w + i as f32 * col_w + col_w * 0.5;
        let name = if i == 0 { "You" } else { player_name(i) };
        text_centered(name, hx, header_y, 15.0, ctx.colors.text_dim);
    }
    draw_line(
        header_x, header_y + 6.0,
        header_x + col_name_w + col_w * 4.0, header_y + 6.0,
        1.0, ctx.colors.gold_dark,
    );

    let row_h = 22.0;
    let max_rows = 16usize;
    let start_y = header_y + 30.0;
    let history = &game.hand_history;
    let skip = history.len().saturating_sub(max_rows);
    let display = history.len().min(max_rows);

    for row in 0..display {
        let idx = skip + row;
        let y = start_y + row as f32 * row_h;
        let entry = history[idx];

        let num = format!("{}", idx + 1);
        draw_text(&num, header_x + 30.0, y, 15.0, ctx.colors.text_dim);

        for (i, &score) in entry.iter().enumerate() {
            let hx = header_x + col_name_w + i as f32 * col_w + col_w * 0.5;
            let color = if score > 0 {
                Color::new(0.40, 0.90, 0.50, 1.0)
            } else if score < 0 {
                Color::new(0.90, 0.40, 0.35, 1.0)
            } else {
                ctx.colors.text_dim
            };
            let s = format!("{score:+}");
            text_centered(&s, hx, y, 15.0, color);
        }
    }

    let total_y = start_y + display as f32 * row_h + 22.0;
    draw_line(
        header_x, total_y - 10.0,
        header_x + col_name_w + col_w * 4.0, total_y - 10.0,
        1.0, ctx.colors.gold_dark,
    );
    draw_text("Total", header_x + 20.0, total_y + 12.0, 17.0, ctx.colors.highlight);
    for i in 0..4 {
        let hx = header_x + col_name_w + i as f32 * col_w + col_w * 0.5;
        let score = game.players[i].score;
        let color = if score > 0 {
            Color::new(0.40, 0.90, 0.50, 1.0)
        } else if score < 0 {
            Color::new(0.90, 0.40, 0.35, 1.0)
        } else {
            ctx.colors.text
        };
        let s = format!("{score:+}");
        text_centered(&s, hx, total_y + 12.0, 20.0, color);
    }
}

fn player_name(idx: usize) -> &'static str {
    match idx {
        0 => "You", 1 => "AI-1", 2 => "AI-2", 3 => "AI-3", _ => "?",
    }
}