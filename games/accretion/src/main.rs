//! Full game loop with five screens and a tabbed right panel.
//!
//! Layout:
//!   Top bar       : Paradigm | [1x][2x][3x][4x] | G
//!   Left          : Disk (centered in the remaining space)
//!   Center-bottom : Advance button (or progress lines)
//!   Right panel   : Tabs (Res / Bldgs / Log / Tree) + Collapse footer

use accretion::components::{Ending, Paradigm, ResourceId};
use accretion::config as cfg;
use accretion::disk::{self, Disk};
use accretion::economy::BUILDINGS;
use accretion::endgame;
use accretion::fonts;
use accretion::format::format_number;
use accretion::paradigm;
use accretion::prestige;
use accretion::save;
use accretion::systems::Game;
use macroquad::prelude::*;

// ─── Screens ────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
enum AppScreen {
    Title,
    Game,
    PrestigeConfirm,
    EndgameChoice,
    Ending(Ending),
}

// ─── UI State ───────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
enum TimeSpeed {
    X1, X2, X3, X4,
}

impl TimeSpeed {
    const ALL: [TimeSpeed; 4] = [Self::X1, Self::X2, Self::X3, Self::X4];

    fn multiplier(self) -> f32 {
        match self {
            Self::X1 => 1.0,
            Self::X2 => 2.0,
            Self::X3 => 3.0,
            Self::X4 => 4.0,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::X1 => "1x",
            Self::X2 => "2x",
            Self::X3 => "3x",
            Self::X4 => "4x",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RightTab {
    Resources,
    Buildings,
    Log,
    Progression,
}

impl RightTab {
    const ALL: [RightTab; 4] = [Self::Resources, Self::Buildings, Self::Log, Self::Progression];

    fn label(self) -> &'static str {
        match self {
            Self::Resources => "Res",
            Self::Buildings => "Bldgs",
            Self::Log => "Log",
            Self::Progression => "Tree",
        }
    }
}

struct UiState {
    speed: TimeSpeed,
    right_tab: RightTab,
    buildings_scroll: f32,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            speed: TimeSpeed::X1,
            right_tab: RightTab::Resources,
            buildings_scroll: 0.0,
        }
    }
}

// ─── Layout constants ───────────────────────────────────────────

const AUTOSAVE_PERIOD: f32 = 30.0;

// Right panel layout
const PANEL_PAD: f32 = 20.0;
const TAB_TOP: f32 = 70.0;
const CONTENT_TOP: f32 = 120.0;
const CONTENT_BOTTOM: f32 = 80.0; // Space reserved for the Collapse button

// Building card layout (inside right panel)
const CARD_W: f32 = cfg::RIGHT_PANEL_W - PANEL_PAD * 2.0; // 280px
const CARD_H: f32 = 76.0;
const CARD_GAP_Y: f32 = 10.0;
const CARD_ROW_H: f32 = CARD_H + CARD_GAP_Y;

// ─── Config ─────────────────────────────────────────────────────

fn window_conf() -> Conf {
    Conf {
        window_title: "Accretion".to_owned(),
        window_width: 1280,
        window_height: 800,
        high_dpi: true,
        ..Default::default()
    }
}

// ─── Loop ───────────────────────────────────────────────────────

#[macroquad::main(window_conf)]
async fn main() {
    fonts::load_default_font().await;

    let mut game = save::load().unwrap_or_else(Game::new);
    let mut disk = Disk::new();
    let mut screen = AppScreen::Title;
    let mut ui = UiState::default();
    let mut autosave_acc = 0.0f32;

    loop {
        let dt = get_frame_time();

        let is_playing = matches!(
            screen,
            AppScreen::Game | AppScreen::PrestigeConfirm | AppScreen::EndgameChoice
        );

        if is_playing {
            let sim_dt = dt * ui.speed.multiplier();
            game.tick(sim_dt);
            disk.tick(sim_dt);
            autosave_acc += dt;
        }

        let should_save = handle_input(&mut game, &mut disk, &mut screen, &mut ui);

        if should_save || (is_playing && autosave_acc >= AUTOSAVE_PERIOD) {
            save::save(&game);
            autosave_acc = 0.0;
        }

        draw(&game, &disk, screen, &ui);
        next_frame().await;
    }
}

// ─── Input ──────────────────────────────────────────────────────

fn handle_input(
    game: &mut Game,
    disk: &mut Disk,
    screen: &mut AppScreen,
    ui: &mut UiState,
) -> bool {
    if is_key_pressed(KeyCode::Escape) {
        match *screen {
            AppScreen::PrestigeConfirm | AppScreen::EndgameChoice => *screen = AppScreen::Game,
            _ => {}
        }
    }
    if is_key_pressed(KeyCode::Enter) && matches!(*screen, AppScreen::Ending(_)) {
        *screen = AppScreen::Title;
    }
    
    if *screen == AppScreen::Game {
        if is_key_pressed(KeyCode::Key1) { ui.speed = TimeSpeed::X1; }
        if is_key_pressed(KeyCode::Key2) { ui.speed = TimeSpeed::X2; }
        if is_key_pressed(KeyCode::Key3) { ui.speed = TimeSpeed::X3; }
        if is_key_pressed(KeyCode::Key4) { ui.speed = TimeSpeed::X4; }
        
        // Tab shortcuts
        if is_key_pressed(KeyCode::R) { ui.right_tab = RightTab::Resources; }
        if is_key_pressed(KeyCode::B) { ui.right_tab = RightTab::Buildings; }
        if is_key_pressed(KeyCode::L) { ui.right_tab = RightTab::Log; }
        if is_key_pressed(KeyCode::T) { ui.right_tab = RightTab::Progression; }
    }

    // Mouse wheel scroll for the Buildings tab
    if *screen == AppScreen::Game && ui.right_tab == RightTab::Buildings {
        let (mx, _) = mouse_position();
        if mx >= screen_width() - cfg::RIGHT_PANEL_W {
            let (_, wheel_y) = mouse_wheel();
            if wheel_y != 0.0 {
                let max_scroll = max_buildings_scroll(game);
                ui.buildings_scroll = (ui.buildings_scroll - wheel_y * 40.0).clamp(0.0, max_scroll);
            }
        }
    }

    if !is_mouse_button_pressed(MouseButton::Left) {
        return false;
    }
    let (mx, my) = mouse_position();
    let pos = vec2(mx, my);

    match *screen {
        AppScreen::Title => handle_title(game, screen, pos),
        AppScreen::Game => handle_game(game, screen, ui, pos),
        AppScreen::PrestigeConfirm => handle_prestige_confirm(game, disk, screen, pos),
        AppScreen::EndgameChoice => handle_endgame_choice(game, disk, screen, pos),
        AppScreen::Ending(_) => false,
    }
}

fn handle_title(game: &mut Game, screen: &mut AppScreen, pos: Vec2) -> bool {
    let has_progress = run_has_progress(game);

    if !has_progress {
        if title_begin_rect().contains(pos) {
            *screen = AppScreen::Game;
        }
        return false;
    }

    if title_continue_rect().contains(pos) {
        *screen = AppScreen::Game;
        return false;
    }
    if prestige::can_prestige(game) && title_prestige_rect().contains(pos) {
        *screen = AppScreen::PrestigeConfirm;
        return false;
    }
    if title_abandon_rect().contains(pos) {
        game.reset_run();
        *screen = AppScreen::Game;
        return true;
    }
    false
}

fn handle_game(
    game: &mut Game,
    screen: &mut AppScreen,
    ui: &mut UiState,
    pos: Vec2,
) -> bool {
    // 1. Right panel
    if pos.x >= screen_width() - cfg::RIGHT_PANEL_W {
        // Tabs
        for (i, tab) in RightTab::ALL.iter().enumerate() {
            if right_tab_rect(i).contains(pos) {
                ui.right_tab = *tab;
                return false;
            }
        }
        // Collapse button
        if prestige::can_prestige(game) && collapse_footer_rect().contains(pos) {
            *screen = AppScreen::PrestigeConfirm;
            return false;
        }
        // Building cards (only if Buildings tab is active)
        if ui.right_tab == RightTab::Buildings {
            let content_y = CONTENT_TOP;
            let local_y = pos.y - content_y + ui.buildings_scroll;
            
            let mut row = 0;
            for def in BUILDINGS {
                if def.paradigm > game.paradigm {
                    continue;
                }
                let card_y = row as f32 * CARD_ROW_H;
                let rect = Rect::new(0.0, card_y, CARD_W, CARD_H);
                
                // pos.x is relative to the panel, so we check local X
                let local_x = pos.x - (screen_width() - cfg::RIGHT_PANEL_W + PANEL_PAD);
                if local_x >= rect.x && local_x <= rect.x + rect.w
                    && local_y >= rect.y && local_y <= rect.y + rect.h
                {
                    if game.can_buy(def.id) {
                        game.buy_building(def.id);
                        return true;
                    }
                    return false;
                }
                row += 1;
            }
        }
        return false;
    }

    // 2. Top bar speed buttons
    for (i, s) in TimeSpeed::ALL.iter().enumerate() {
        if speed_button_rect(i).contains(pos) {
            ui.speed = *s;
            return false;
        }
    }

    // 3. Endgame button
    if endgame::can_endgame(game) && endgame_button_rect().contains(pos) {
        *screen = AppScreen::EndgameChoice;
        return false;
    }

    // 4. Advance button
    if game.can_advance().is_some() && advance_button_rect().contains(pos) {
        game.try_advance();
        return true;
    }

    // 5. Disk click
    if (pos - disk::center()).length() <= disk::radius() {
        game.click();
        return false;
    }

    false
}

fn handle_prestige_confirm(
    game: &mut Game,
    disk: &mut Disk,
    screen: &mut AppScreen,
    pos: Vec2,
) -> bool {
    if prestige_confirm_rect().contains(pos) {
        prestige::prestige(game);
        *disk = Disk::new();
        *screen = AppScreen::Title;
        return true;
    }
    if prestige_cancel_rect().contains(pos) {
        *screen = AppScreen::Game;
    }
    false
}

fn handle_endgame_choice(
    game: &mut Game,
    disk: &mut Disk,
    screen: &mut AppScreen,
    pos: Vec2,
) -> bool {
    if endgame_neutron_rect().contains(pos) {
        endgame::choose_ending(game, Ending::NeutronStar);
        *disk = Disk::new();
        *screen = AppScreen::Ending(Ending::NeutronStar);
        return true;
    }
    if endgame_blackhole_rect().contains(pos) {
        endgame::choose_ending(game, Ending::BlackHole);
        *disk = Disk::new();
        *screen = AppScreen::Ending(Ending::BlackHole);
        return true;
    }
    false
}

// ─── Layout helpers ─────────────────────────────────────────────

fn run_has_progress(game: &Game) -> bool {
    game.paradigm != Paradigm::Dust
        || game.resources.values().any(|v| *v > 0.0)
        || game.buildings.iter().any(|b| b.count > 0)
}

fn max_buildings_scroll(game: &Game) -> f32 {
    let visible = BUILDINGS.iter().filter(|d| d.paradigm <= game.paradigm).count();
    let total_h = visible as f32 * CARD_ROW_H;
    let viewport_h = screen_height() - CONTENT_TOP - CONTENT_BOTTOM;
    (total_h - viewport_h).max(0.0)
}

fn advance_button_rect() -> Rect {
    let w = 320.0;
    let h = 48.0;
    let game_w = screen_width() - cfg::RIGHT_PANEL_W;
    Rect::new((game_w - w) / 2.0, screen_height() - 80.0, w, h)
}

fn endgame_button_rect() -> Rect {
    let w = 260.0;
    let h = 48.0;
    let game_w = screen_width() - cfg::RIGHT_PANEL_W;
    Rect::new((game_w - w) / 2.0, screen_height() - 140.0, w, h)
}

fn speed_button_rect(i: usize) -> Rect {
    Rect::new(320.0 + i as f32 * 50.0, 15.0, 40.0, 30.0)
}

fn right_tab_rect(i: usize) -> Rect {
    let panel_x = screen_width() - cfg::RIGHT_PANEL_W;
    // 4 tabs: 72px wide each, 4px gap
    Rect::new(panel_x + 8.0 + i as f32 * 76.0, TAB_TOP, 72.0, 30.0)
}

fn collapse_footer_rect() -> Rect {
    let panel_x = screen_width() - cfg::RIGHT_PANEL_W;
    Rect::new(panel_x + 20.0, screen_height() - 64.0, cfg::RIGHT_PANEL_W - 40.0, 44.0)
}

fn prestige_confirm_rect() -> Rect {
    Rect::new(screen_width() / 2.0 - 100.0, screen_height() / 2.0 + 60.0, 80.0, 40.0)
}
fn prestige_cancel_rect() -> Rect {
    Rect::new(screen_width() / 2.0 + 20.0, screen_height() / 2.0 + 60.0, 80.0, 40.0)
}
fn endgame_neutron_rect() -> Rect {
    Rect::new(screen_width() / 2.0 - 240.0, screen_height() / 2.0 + 60.0, 200.0, 60.0)
}
fn endgame_blackhole_rect() -> Rect {
    Rect::new(screen_width() / 2.0 + 40.0, screen_height() / 2.0 + 60.0, 200.0, 60.0)
}
fn title_begin_rect() -> Rect {
    Rect::new(screen_width() / 2.0 - 120.0, screen_height() / 2.0 + 100.0, 240.0, 56.0)
}
fn title_continue_rect() -> Rect {
    Rect::new(screen_width() / 2.0 - 120.0, screen_height() / 2.0 + 60.0, 240.0, 56.0)
}
fn title_prestige_rect() -> Rect {
    Rect::new(screen_width() / 2.0 - 120.0, screen_height() / 2.0 + 130.0, 240.0, 48.0)
}
fn title_abandon_rect() -> Rect {
    Rect::new(screen_width() / 2.0 - 120.0, screen_height() / 2.0 + 190.0, 240.0, 40.0)
}

// ─── Draw ───────────────────────────────────────────────────────

fn draw(game: &Game, disk: &Disk, screen: AppScreen, ui: &UiState) {
    clear_background(cfg::BG);
    let accent = cfg::paradigm_color(game.paradigm);

    match screen {
        AppScreen::Title => draw_title(game, accent),
        AppScreen::Ending(ending) => draw_ending(ending, accent),
        AppScreen::Game => draw_game(game, disk, accent, ui),
        AppScreen::PrestigeConfirm => {
            draw_game(game, disk, accent, ui);
            draw_prestige_confirm(game, accent);
        }
        AppScreen::EndgameChoice => {
            draw_game(game, disk, accent, ui);
            draw_endgame_choice(accent);
        }
    }
}

fn draw_game(game: &Game, disk: &Disk, accent: Color, ui: &UiState) {
    draw_disk(disk, game, accent);
    draw_right_panel(game, accent, ui.right_tab, ui.buildings_scroll);
    draw_top_bar(game, accent, ui.speed);
    draw_advance_button(game, accent);
    if endgame::can_endgame(game) {
        draw_endgame_button(accent);
    }
}

fn draw_title(game: &Game, accent: Color) {
    let cx = screen_width() / 2.0;
    let cy = screen_height() / 2.0;

    let title = "ACCRETION";
    let dims = measure_text(title, None, 64, 1.0);
    draw_text(title, cx - dims.width / 2.0, cy - 120.0, 64.0, accent);

    let endings = format!("Endings: {}/2", game.endings_seen.len());
    let g = format!("G: {}", game.g_total);
    let meta = format!("{endings}    {g}");
    let m = measure_text(&meta, None, 20, 1.0);
    draw_text(&meta, cx - m.width / 2.0, cy - 40.0, 20.0, cfg::TEXT);

    let has_progress = run_has_progress(game);

    let button = |rect: Rect, label: &str, color: Color| {
        let hover = rect.contains(vec2(mouse_position().0, mouse_position().1));
        draw_rectangle(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            if hover { cfg::BUTTON_HOVER } else { cfg::BUTTON },
        );
        draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, color);
        let d = measure_text(label, None, 20, 1.0);
        draw_text(
            label,
            rect.x + (rect.w - d.width) / 2.0,
            rect.y + (rect.h + d.height) / 2.0,
            20.0,
            cfg::TEXT,
        );
    };

    if !has_progress {
        button(title_begin_rect(), "Begin", accent);
    } else {
        button(title_continue_rect(), "Continue", accent);
        if prestige::can_prestige(game) {
            let gain = prestige::compute_g_gain(game);
            let label = format!("Prestige (+{gain} G)");
            button(title_prestige_rect(), &label, accent);
        }
        button(title_abandon_rect(), "Abandon Run", cfg::DIM);
    }
}

fn draw_ending(ending: Ending, accent: Color) {
    let cx = screen_width() / 2.0;
    let cy = screen_height() / 2.0;

    let title = ending.name().to_uppercase();
    let dims = measure_text(&title, None, 48, 1.0);
    draw_text(&title, cx - dims.width / 2.0, cy - 80.0, 48.0, accent);

    let mut y = cy - 10.0;
    for line in ending.description() {
        let d = measure_text(line, None, 24, 1.0);
        draw_text(line, cx - d.width / 2.0, y, 24.0, cfg::TEXT);
        y += 36.0;
    }

    let hint = "Press Enter";
    let d = measure_text(hint, None, 18, 1.0);
    draw_text(hint, cx - d.width / 2.0, screen_height() - 60.0, 18.0, cfg::DIM);
}

fn draw_disk(disk: &Disk, game: &Game, accent: Color) {
    let center = disk::center();
    let radius = disk::radius();

    let (mx, my) = mouse_position();
    let hover = (vec2(mx, my) - center).length() <= radius;
    if hover {
        draw_circle_lines(center.x, center.y, radius, 1.5, accent.with_alpha(0.25));
    }

    disk.draw(game, center, radius);

    if game.paradigm == Paradigm::Dust {
        let label = "+1 Dust (click the disk)";
        let dims = measure_text(label, None, 16, 1.0);
        draw_text(
            label,
            center.x - dims.width / 2.0,
            center.y + radius + 24.0,
            16.0,
            cfg::DIM,
        );
    }
}

fn draw_top_bar(game: &Game, accent: Color, speed: TimeSpeed) {
    draw_rectangle(0.0, 0.0, screen_width(), 60.0, cfg::PANEL);
    draw_rectangle(0.0, 60.0, screen_width(), 1.0, cfg::PANEL_BORDER);

    let title = format!("Paradigm: {}", game.paradigm.name());
    draw_text(&title, 20.0, 38.0, 26.0, accent);

    let mouse = vec2(mouse_position().0, mouse_position().1);

    for (i, s) in TimeSpeed::ALL.iter().enumerate() {
        let rect = speed_button_rect(i);
        let active = speed == *s;
        let hover = rect.contains(mouse);
        let bg = if active {
            accent.with_alpha(0.35)
        } else if hover {
            cfg::BUTTON_HOVER
        } else {
            cfg::BUTTON
        };
        draw_rectangle(rect.x, rect.y, rect.w, rect.h, bg);
        draw_rectangle_lines(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            1.0,
            if active { accent } else { cfg::PANEL_BORDER },
        );
        let d = measure_text(s.label(), None, 16, 1.0);
        draw_text(
            s.label(),
            rect.x + (rect.w - d.width) / 2.0,
            rect.y + (rect.h + d.height) / 2.0,
            16.0,
            if active { cfg::TEXT } else { cfg::DIM },
        );
    }

    if game.g_multiplier > 1.0 {
        let label = format!("G: +{:.0}%", (game.g_multiplier - 1.0) * 100.0);
        let d = measure_text(&label, None, 20, 1.0);
        draw_text(
            &label,
            screen_width() - cfg::RIGHT_PANEL_W - d.width - 20.0,
            38.0,
            20.0,
            cfg::DIM,
        );
    }
}

fn draw_right_panel(game: &Game, accent: Color, right_tab: RightTab, buildings_scroll: f32) {
    let panel_x = screen_width() - cfg::RIGHT_PANEL_W;
    draw_rectangle(panel_x, 60.0, cfg::RIGHT_PANEL_W, screen_height() - 60.0, cfg::PANEL);
    draw_rectangle(panel_x, 60.0, 1.0, screen_height() - 60.0, cfg::PANEL_BORDER);

    // Tab strip
    let mouse = vec2(mouse_position().0, mouse_position().1);
    for (i, tab) in RightTab::ALL.iter().enumerate() {
        let rect = right_tab_rect(i);
        let active = right_tab == *tab;
        let hover = rect.contains(mouse);
        let bg = if active {
            accent.with_alpha(0.30)
        } else if hover {
            cfg::BUTTON_HOVER
        } else {
            cfg::BUTTON
        };
        draw_rectangle(rect.x, rect.y, rect.w, rect.h, bg);
        draw_rectangle_lines(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            1.0,
            if active { accent } else { cfg::PANEL_BORDER },
        );
        let d = measure_text(tab.label(), None, 14, 1.0);
        draw_text(
            tab.label(),
            rect.x + (rect.w - d.width) / 2.0,
            rect.y + (rect.h + d.height) / 2.0,
            14.0,
            if active { cfg::TEXT } else { cfg::DIM },
        );
    }

    // Content
    let content_x = panel_x + PANEL_PAD;
    let content_y = CONTENT_TOP;
    let viewport_h = screen_height() - CONTENT_TOP - CONTENT_BOTTOM;

    match right_tab {
        RightTab::Resources => draw_resources_tab(game, content_x, content_y),
        RightTab::Log => draw_log_tab(game, content_x, content_y),
        RightTab::Progression => draw_progression_tab(game, content_x, content_y, accent),
        RightTab::Buildings => {
            // Scissor test for the scrollable area
            let dpi = screen_dpi_scale();
            let gl = unsafe { get_internal_gl() };
            unsafe {
                gl.quad_gl.scissor(Some((
                    (content_x * dpi) as i32,
                    (content_y * dpi) as i32,
                    (CARD_W * dpi) as i32,
                    (viewport_h * dpi) as i32,
                )));
            }

            let mut row = 0;
            for def in BUILDINGS {
                if def.paradigm > game.paradigm {
                    continue;
                }
                let y = content_y + row as f32 * CARD_ROW_H - buildings_scroll;
                let rect = Rect::new(content_x, y, CARD_W, CARD_H);

                let affordable = game.can_buy(def.id);
                let hovered = rect.contains(mouse);
                let bg = if affordable && hovered {
                    cfg::BUTTON_HOVER
                } else if affordable {
                    cfg::BUTTON
                } else {
                    cfg::BUTTON_DISABLED
                };

                draw_rectangle(rect.x, rect.y, rect.w, rect.h, bg);
                draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, cfg::PANEL_BORDER);

                draw_text(
                    &format!("{}  \u{00d7}{}", def.name, game.building_count(def.id)),
                    rect.x + 12.0,
                    rect.y + 24.0,
                    16.0,
                    if affordable { cfg::TEXT } else { cfg::DIM },
                );

                let cost_str: String = game.cost_of_next(def.id)
                    .iter()
                    .map(|(r, v)| format!("{} {}", format_number(v.ceil()), r.name()))
                    .collect::<Vec<_>>()
                    .join(" + ");
                draw_text(&cost_str, rect.x + 12.0, rect.y + 46.0, 13.0, cfg::DIM);

                let desc = truncate_to_width(def.description, rect.w - 24.0, 11);
                draw_text(
                    &desc,
                    rect.x + 12.0,
                    rect.y + 66.0,
                    11.0,
                    accent.with_alpha(if affordable { 0.75 } else { 0.35 }),
                );

                row += 1;
            }

            // Restore scissor
            unsafe { gl.quad_gl.scissor(None) };

            // Scrollbar
            let max_scroll = max_buildings_scroll(game);
            if max_scroll > 0.0 {
                let bar_x = panel_x + cfg::RIGHT_PANEL_W - 8.0;
                let bar_h = viewport_h;
                draw_rectangle(bar_x, content_y, 4.0, bar_h, cfg::PANEL_BORDER);

                let handle_h = (viewport_h / (viewport_h + max_scroll)) * bar_h;
                let handle_y = content_y + (buildings_scroll / max_scroll) * (bar_h - handle_h);
                draw_rectangle(bar_x, handle_y, 4.0, handle_h, accent);
            }
        }
    }

    // Footer: Collapse button
    if game.paradigm >= Paradigm::Planets {
        let enabled = prestige::can_prestige(game);
        let rect = collapse_footer_rect();
        let hover = rect.contains(mouse);
        let bg = if !enabled {
            cfg::BUTTON_DISABLED
        } else if hover {
            cfg::BUTTON_HOVER
        } else {
            cfg::BUTTON
        };
        draw_rectangle(rect.x, rect.y, rect.w, rect.h, bg);
        draw_rectangle_lines(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            1.0,
            if enabled { accent } else { cfg::PANEL_BORDER },
        );
        let label = if enabled { "Collapse" } else { "Reach Gas Giant" };
        let d = measure_text(label, None, 16, 1.0);
        draw_text(
            label,
            rect.x + (rect.w - d.width) / 2.0,
            rect.y + (rect.h + d.height) / 2.0,
            16.0,
            if enabled { cfg::TEXT } else { cfg::DIM },
        );
    }
}

fn truncate_to_width(s: &str, max_width: f32, font_size: u16) -> String {
    if measure_text(s, None, font_size, 1.0).width <= max_width {
        return s.to_string();
    }
    let ellipsis = "...";
    let ellipsis_w = measure_text(ellipsis, None, font_size, 1.0).width;
    let available = max_width - ellipsis_w;

    let mut byte_end = 0usize;
    let mut last_space = 0usize;
    for (i, ch) in s.char_indices() {
        let end = i + ch.len_utf8();
        if measure_text(&s[..end], None, font_size, 1.0).width > available {
            break;
        }
        byte_end = end;
        if ch == ' ' {
            last_space = end;
        }
    }
    let cut = if last_space > 0 && last_space < byte_end {
        last_space
    } else {
        byte_end
    };
    format!("{}{}", s[..cut].trim_end(), ellipsis)
}

fn draw_resources_tab(game: &Game, x: f32, y: f32) {
    let value_x = screen_width() - 20.0;
    let mut cy = y;
    for r in ResourceId::ALL {
        let amount = game.resources[&r];
        if r != ResourceId::Dust && amount <= 0.0 {
            continue;
        }
        draw_text(r.name(), x, cy, 16.0, cfg::DIM);
        let value = format_number(amount);
        let vd = measure_text(&value, None, 18, 1.0);
        draw_text(&value, value_x - vd.width, cy, 18.0, cfg::TEXT);
        cy += 30.0;
    }
}

fn draw_log_tab(game: &Game, x: f32, y: f32) {
    let mut cy = y;
    for line in game.log.iter().rev().take(15) {
        draw_text(line, x, cy, 14.0, cfg::DIM);
        cy += 22.0;
    }
}

fn draw_progression_tab(game: &Game, x: f32, y: f32, accent: Color) {
    let mut cy = y;
    for p in Paradigm::ALL {
        let reached = game.paradigm >= p;
        let current = game.paradigm == p;
        let prefix = if current {
            "\u{25b6} "
        } else if reached {
            "\u{2713} "
        } else {
            "\u{25cb} "
        };
        let label = format!("{}{}", prefix, p.name());
        let color = if current {
            accent
        } else if reached {
            cfg::TEXT
        } else {
            cfg::DIM
        };
        draw_text(&label, x, cy, 16.0, color);
        cy += 26.0;
    }

    if let Some(cond) = paradigm::next_condition(game.paradigm) {
        cy += 16.0;
        draw_text("Requirements:", x, cy, 14.0, cfg::TEXT);
        cy += 20.0;
        let lines = paradigm::progress_lines(cond, &game.buildings, &game.resources);
        for line in lines {
            draw_text(&line, x + 10.0, cy, 12.0, cfg::DIM);
            cy += 18.0;
        }
    }
}

fn draw_advance_button(game: &Game, accent: Color) {
    let next = game.can_advance();
    let cond = paradigm::next_condition(game.paradigm);

    if let Some(target) = next {
        let pulse = 0.5 + 0.5 * (get_time() as f32 * 3.0).sin();
        let rect = advance_button_rect();
        let hover = rect.contains(vec2(mouse_position().0, mouse_position().1));
        let bg = if hover {
            accent.with_alpha(0.55)
        } else {
            accent.with_alpha(0.20 + 0.25 * pulse)
        };
        draw_rectangle(rect.x, rect.y, rect.w, rect.h, bg);
        draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2.0, accent);
        let label = format!("Advance \u{2192} {}", target.name());
        let dims = measure_text(&label, None, 20, 1.0);
        draw_text(
            &label,
            rect.x + (rect.w - dims.width) / 2.0,
            rect.y + (rect.h + dims.height) / 2.0,
            20.0,
            cfg::TEXT,
        );
    } else if let Some(c) = cond {
        let lines = paradigm::progress_lines(c, &game.buildings, &game.resources);
        let title = format!("Next: {}", c.target.name());
        let cx = (screen_width() - cfg::RIGHT_PANEL_W) / 2.0;
        let base_y = screen_height() - 160.0;
        let td = measure_text(&title, None, 18, 1.0);
        draw_text(&title, cx - td.width / 2.0, base_y, 18.0, accent);
        for (i, line) in lines.iter().enumerate() {
            let d = measure_text(line, None, 14, 1.0);
            draw_text(
                line,
                cx - d.width / 2.0,
                base_y + 24.0 + i as f32 * 18.0,
                14.0,
                cfg::DIM,
            );
        }
    }
}

fn draw_endgame_button(accent: Color) {
    let rect = endgame_button_rect();
    let hover = rect.contains(vec2(mouse_position().0, mouse_position().1));
    let pulse = 0.5 + 0.5 * (get_time() as f32 * 2.0).sin();
    let bg = if hover {
        accent.with_alpha(0.7)
    } else {
        accent.with_alpha(0.35 + 0.25 * pulse)
    };
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, bg);
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2.0, accent);
    let label = "The End of the Fall";
    let d = measure_text(label, None, 18, 1.0);
    draw_text(
        label,
        rect.x + (rect.w - d.width) / 2.0,
        rect.y + (rect.h + d.height) / 2.0,
        18.0,
        cfg::TEXT,
    );
}

fn draw_prestige_confirm(game: &Game, accent: Color) {
    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        screen_height(),
        Color::new(0.0, 0.0, 0.0, 0.72),
    );

    let w = 440.0;
    let h = 260.0;
    let x = (screen_width() - w) / 2.0;
    let y = (screen_height() - h) / 2.0;
    draw_rectangle(x, y, w, h, cfg::PANEL);
    draw_rectangle_lines(x, y, w, h, 2.0, accent);

    let gain = prestige::compute_g_gain(game);
    let new_total = game.g_total + gain;
    let new_mult = 1.0 + 0.05 * new_total as f64;

    let cx = x + w / 2.0;
    let centered = |text: &str, dy: f32, size: u16, color: Color| {
        let dims = measure_text(text, None, size, 1.0);
        draw_text(text, cx - dims.width / 2.0, y + dy, size as f32, color);
    };
    centered("COLLAPSE", 40.0, 26, accent);
    centered("Reset your run.", 90.0, 16, cfg::TEXT);
    centered(&format!("G gain:  +{gain}"), 130.0, 18, cfg::TEXT);
    centered(&format!("New G:   {new_total}"), 160.0, 18, cfg::TEXT);
    centered(&format!("New mult:  \u{00d7}{new_mult:.2}"), 190.0, 18, cfg::TEXT);

    let mouse = vec2(mouse_position().0, mouse_position().1);

    let cr = prestige_confirm_rect();
    let hover_c = cr.contains(mouse);
    draw_rectangle(
        cr.x,
        cr.y,
        cr.w,
        cr.h,
        if hover_c { cfg::BUTTON_HOVER } else { cfg::BUTTON },
    );
    draw_rectangle_lines(cr.x, cr.y, cr.w, cr.h, 1.0, accent);
    let d = measure_text("Confirm", None, 16, 1.0);
    draw_text(
        "Confirm",
        cr.x + (cr.w - d.width) / 2.0,
        cr.y + (cr.h + d.height) / 2.0,
        16.0,
        cfg::TEXT,
    );

    let xr = prestige_cancel_rect();
    let hover_x = xr.contains(mouse);
    draw_rectangle(
        xr.x,
        xr.y,
        xr.w,
        xr.h,
        if hover_x { cfg::BUTTON_HOVER } else { cfg::BUTTON },
    );
    draw_rectangle_lines(xr.x, xr.y, xr.w, xr.h, 1.0, cfg::PANEL_BORDER);
    let d = measure_text("Cancel", None, 16, 1.0);
    draw_text(
        "Cancel",
        xr.x + (xr.w - d.width) / 2.0,
        xr.y + (xr.h + d.height) / 2.0,
        16.0,
        cfg::DIM,
    );
}

fn draw_endgame_choice(accent: Color) {
    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        screen_height(),
        Color::new(0.0, 0.0, 0.0, 0.82),
    );

    let w = 600.0;
    let h = 320.0;
    let x = (screen_width() - w) / 2.0;
    let y = (screen_height() - h) / 2.0;
    draw_rectangle(x, y, w, h, cfg::PANEL);
    draw_rectangle_lines(x, y, w, h, 2.0, accent);

    let cx = x + w / 2.0;
    let centered = |text: &str, dy: f32, size: u16, color: Color| {
        let dims = measure_text(text, None, size, 1.0);
        draw_text(text, cx - dims.width / 2.0, y + dy, size as f32, color);
    };
    centered("THE END OF THE FALL", 50.0, 28, accent);
    centered("Choose.", 100.0, 18, cfg::TEXT);

    let mouse = vec2(mouse_position().0, mouse_position().1);

    // Neutron Star button.
    let nr = endgame_neutron_rect();
    let hover_n = nr.contains(mouse);
    draw_rectangle(
        nr.x,
        nr.y,
        nr.w,
        nr.h,
        if hover_n { cfg::BUTTON_HOVER } else { cfg::BUTTON },
    );
    draw_rectangle_lines(
        nr.x,
        nr.y,
        nr.w,
        nr.h,
        2.0,
        Color::new(0.85, 0.95, 1.0, 1.0),
    );
    let d = measure_text("Neutron Star", None, 22, 1.0);
    draw_text(
        "Neutron Star",
        nr.x + (nr.w - d.width) / 2.0,
        nr.y + (nr.h + d.height) / 2.0,
        22.0,
        Color::new(0.85, 0.95, 1.0, 1.0),
    );

    // Black Hole button.
    let br = endgame_blackhole_rect();
    let hover_b = br.contains(mouse);
    draw_rectangle(
        br.x,
        br.y,
        br.w,
        br.h,
        if hover_b { cfg::BUTTON_HOVER } else { cfg::BUTTON },
    );
    draw_rectangle_lines(br.x, br.y, br.w, br.h, 2.0, Color::new(0.6, 0.4, 0.8, 1.0));
    let d = measure_text("Black Hole", None, 22, 1.0);
    draw_text(
        "Black Hole",
        br.x + (br.w - d.width) / 2.0,
        br.y + (br.h + d.height) / 2.0,
        22.0,
        Color::new(0.6, 0.4, 0.8, 1.0),
    );
}