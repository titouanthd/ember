//! FreeCell as a node tree (Phase 7 migration).
//!
//! ```text
//! FreeCellGame            root: owns the untouched `Game`, runs the old
//! │                       input handlers (keys, drag and drop), draws
//! │                       menu, board, overlays and win banner
//! └── UiRoot
//!     ├── Hint            ember_ui Buttons (were ember_stdlib Buttons
//!     ├── Restart         stored in `Game`); clicks arrive as
//!     ├── New             `ButtonClicked` and run on the next process
//!     └── Menu
//! ```

use crate::GameState;
use crate::components::{Card, Color as CardColor, Suit, Zone};
use crate::config::{self, GameContext};
use crate::drag::DragState;
use crate::fonts;
use crate::layout;
use crate::systems::Game;
use ember::prelude::*;

/// Frame time cap, as in the macroquad version.
pub const MAX_FRAME_DT: f32 = 1.0 / 30.0;
const BUTTON_W: f32 = 100.0;
const BUTTON_H: f32 = 30.0;
const DROP_OK: Color = Color::new(0.30, 0.90, 0.45, 0.65);
const DROP_BAD: Color = Color::new(0.95, 0.30, 0.35, 0.55);
const SUIT_PLACEHOLDER_BLACK: Color = Color::new(0.55, 0.60, 0.68, 1.0);
const BANNER_BG: Color = Color::new(0.0, 0.0, 0.0, 0.88);

/// `mm:ss` (or `h:mm:ss`), like `ember_stdlib::ui::timer::TimerDisplay`.
pub fn format_time(seconds: f32) -> String {
    let total = seconds.max(0.0) as u64;
    let (h, m, s) = (total / 3600, (total % 3600) / 60, total % 60);
    if h > 0 {
        format!("{h:02}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}

/// A seed from the clock, as before.
pub fn random_seed() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u32)
        .unwrap_or(1)
}

/// Ids of the four header buttons.
#[derive(Debug, Clone, Copy)]
pub struct Buttons {
    pub hint: NodeId,
    pub restart: NodeId,
    pub new_game: NodeId,
    pub menu: NodeId,
}

/// Left edge of header button `slot` (0 = Hint … 3 = Menu), as before.
pub fn button_x(ctx: &GameContext, slot: usize) -> f32 {
    ctx.window_w - 480.0 + slot as f32 * 120.0
}

pub struct FreeCellGame {
    name: String,
    pub ctx: GameContext,
    pub game: Game,
    pub buttons: Option<Buttons>,
    /// Whether DejaVu Sans Mono loaded (set in `ready`).
    pub font_loaded: bool,
    ui_root: Option<NodeId>,
    clicked: Vec<NodeId>,
    /// Pointer position at the last `process`, for the drag overlay and
    /// the drop-target highlight in `draw`.
    mouse: Vec2,
}

impl FreeCellGame {
    pub fn new(ctx: GameContext, game: Game) -> Self {
        Self {
            name: "FreeCell".into(),
            ctx,
            game,
            buttons: None,
            font_loaded: false,
            ui_root: None,
            clicked: Vec::new(),
            mouse: Vec2::ZERO,
        }
    }

    fn back_to_menu(&mut self) {
        self.game.state = GameState::Start;
        self.game.hint = None;
    }

    fn on_button(&mut self, id: NodeId) {
        let Some(b) = self.buttons else { return };
        if self.game.state != GameState::Playing {
            return;
        }
        let ctx = &self.ctx;
        let game = &mut self.game;
        if id == b.hint {
            game.hint = game.find_hint();
        } else if id == b.restart {
            game.start_game(game.seed, ctx);
        } else if id == b.new_game {
            game.start_game(random_seed(), ctx);
        } else if id == b.menu {
            self.back_to_menu();
        }
    }

    fn handle_start_input(&mut self, input: &Input) {
        if input.is_key_pressed(Key::N) {
            self.game.start_game(random_seed(), &self.ctx);
        }
        if input.is_key_pressed(Key::Enter) || input.is_key_pressed(Key::Space) {
            self.game.start_game(self.game.seed, &self.ctx);
        }
    }

    fn handle_playing_input(&mut self, input: &Input) {
        let ctx = &self.ctx;
        let game = &mut self.game;
        if input.is_key_pressed(Key::Z) || input.is_key_pressed(Key::W) {
            game.undo();
            return;
        }
        if input.is_key_pressed(Key::Y) {
            game.redo();
            return;
        }
        if input.is_key_pressed(Key::N) {
            game.start_game(random_seed(), ctx);
            return;
        }
        if input.is_key_pressed(Key::R) {
            game.start_game(game.seed, ctx);
            return;
        }
        if input.is_key_pressed(Key::Tab) {
            self.back_to_menu();
            return;
        }

        if input.mouse_left_pressed {
            game.start_drag(input.mouse_pos, ctx);
        }
        if input.mouse_left_down {
            game.update_drag(input.mouse_pos, ctx);
        }
        if input.mouse_left_released {
            game.end_drag(input.mouse_pos, ctx);
        }
        if !input.mouse_left_down && game.drag.is_dragging() {
            game.cancel_drag();
        }
    }

    fn handle_win_input(&mut self, input: &Input) {
        if input.is_key_pressed(Key::R) || input.is_key_pressed(Key::Space) {
            self.game.start_game(self.game.seed, &self.ctx);
            return;
        }
        if input.is_key_pressed(Key::N) {
            self.game.start_game(random_seed(), &self.ctx);
            return;
        }
        if input.is_key_pressed(Key::Tab)
            || input.is_key_pressed(Key::M)
            || input.is_key_pressed(Key::Semicolon)
        {
            self.back_to_menu();
        }
    }

    /// Buttons show on the board (playing and win banner, as before) and
    /// are only active while playing.
    fn sync_buttons(&self, tree: &mut NodeTree) {
        let Some(b) = self.buttons else { return };
        let state = self.game.state;
        let show = matches!(state, GameState::Playing | GameState::Win);
        let active = state == GameState::Playing;
        for id in [b.hint, b.restart, b.new_game, b.menu] {
            if let Some(button) = tree
                .get_mut(id)
                .and_then(|n| n.as_any_mut().downcast_mut::<Button>())
            {
                button.base_mut().visible = show;
                if button.is_disabled() == active {
                    button.set_disabled(!active);
                }
            }
        }
    }

    // --- Drawing (as the old render_* functions) ------------------------

    fn draw_start(&self, r: &mut dyn RenderingServer) {
        let ctx = &self.ctx;
        let game = &self.game;
        let cx = ctx.window_w * 0.5;
        centered(r, "FREECELL", cx, 120.0, 72.0, ctx.color_accent);
        centered(
            r,
            "// all cards visible",
            cx,
            160.0,
            20.0,
            ctx.color_text_dim,
        );
        centered(
            r,
            &format!("Seed: {}", game.seed),
            cx,
            260.0,
            32.0,
            ctx.color_text,
        );
        let best = match game.best_times.get(&game.seed.to_string()) {
            Some(&t) => format!("Best: {}", format_time(t)),
            None => "Best: --:--".to_owned(),
        };
        centered(r, &best, cx, 310.0, 24.0, ctx.color_accent);
        centered(
            r,
            "N: new game (random seed)",
            cx,
            400.0,
            24.0,
            ctx.color_text,
        );
        centered(r, "Enter / Space: start", cx, 440.0, 24.0, ctx.color_text);
        centered(
            r,
            "Drag cards to move them. Z / Y for undo / redo.",
            cx,
            520.0,
            18.0,
            ctx.color_text_dim,
        );
    }

    fn draw_game(&self, r: &mut dyn RenderingServer) {
        self.draw_header(r);
        for i in 0..4 {
            self.draw_free_cell(r, i);
        }
        for i in 0..4 {
            self.draw_foundation(r, i);
        }
        for col in 0..8 {
            self.draw_column(r, col);
        }
        self.draw_drop_target(r);
        self.draw_hint_overlay(r);
        self.draw_drag_overlay(r);
        self.draw_footer(r);
    }

    fn draw_header(&self, r: &mut dyn RenderingServer) {
        let ctx = &self.ctx;
        let game = &self.game;
        r.draw_rect(
            Rect::new(0.0, 0.0, ctx.window_w, ctx.hud_h),
            ctx.color_header_bg,
        );
        let cy = ctx.hud_h * 0.5 + 7.0;

        let seed = format!("Seed: {}", game.seed);
        r.draw_text(&seed, Vec2::new(12.0, cy), 20.0, ctx.color_text);
        let seed_w = r.measure_text(&seed, 20.0).width;
        r.draw_text(
            &format!("Moves: {}", game.moves),
            Vec2::new(12.0 + seed_w + 30.0, cy),
            20.0,
            ctx.color_text,
        );

        // Timer: just left of the Hint button.
        let time = format!("Time: {}", format_time(game.elapsed));
        let color = if game.timer_running {
            ctx.color_text
        } else {
            ctx.color_text_dim
        };
        let time_w = r.measure_text(&time, 20.0).width;
        let x = (button_x(ctx, 0) - time_w - 20.0).max(0.0);
        r.draw_text(&time, Vec2::new(x, cy), 20.0, color);
    }

    fn draw_footer(&self, r: &mut dyn RenderingServer) {
        let ctx = &self.ctx;
        r.draw_rect(
            Rect::new(0.0, ctx.window_h - ctx.footer_h, ctx.window_w, ctx.footer_h),
            ctx.color_header_bg,
        );
        centered(
            r,
            "Drag cards · Hint shows a move · N new · R restart · Tab menu · Z undo · Y redo · Esc quit",
            ctx.window_w * 0.5,
            ctx.window_h - ctx.footer_h * 0.5 + 5.0,
            14.0,
            ctx.color_text_dim,
        );
    }

    fn draw_free_cell(&self, r: &mut dyn RenderingServer, i: usize) {
        let rect = layout::free_cell_rect(i, &self.ctx);
        match self.game.free_cells[i] {
            Some(card) => self.draw_card(r, card, rect),
            None => self.draw_empty_slot(r, rect),
        }
    }

    fn draw_foundation(&self, r: &mut dyn RenderingServer, i: usize) {
        let ctx = &self.ctx;
        let rect = layout::foundation_rect(i, ctx);
        if let Some(card) = self.game.foundations[i].last() {
            self.draw_card(r, *card, rect);
            return;
        }
        self.draw_empty_slot(r, rect);
        // Suit placeholder: light colours on the dark slot, alpha 0.45.
        let suit = Suit::ALL[i];
        let symbol = suit.symbol().to_string();
        let base = match suit.color() {
            CardColor::Red => ctx.color_card_red,
            CardColor::Black => SUIT_PLACEHOLDER_BLACK,
        };
        let color = Color::new(base.r, base.g, base.b, 0.45);
        let size = (rect.w * 0.55).floor();
        let m = r.measure_text(&symbol, size);
        // Baseline: centre of the rect plus ~35 % of the glyph height.
        let pos = Vec2::new(
            rect.x + (rect.w - m.width) * 0.5,
            rect.y + rect.h * 0.5 + m.height * 0.35,
        );
        r.draw_text(&symbol, pos, size, color);
    }

    fn draw_empty_slot(&self, r: &mut dyn RenderingServer, rect: Rect) {
        r.draw_rect(rect, self.ctx.color_slot_empty);
        r.draw_rect_outline(rect, 2.0, self.ctx.color_slot_border);
    }

    fn draw_column(&self, r: &mut dyn RenderingServer, col: usize) {
        let ctx = &self.ctx;
        let cards = &self.game.columns[col];
        if cards.is_empty() {
            let top = layout::column_top(col, ctx);
            self.draw_empty_slot(r, Rect::new(top.x, top.y, ctx.card_w, ctx.card_h));
            return;
        }
        for (card_index, card) in cards.iter().enumerate() {
            if self.is_being_dragged(col, card_index) {
                continue;
            }
            let rect = layout::card_rect_in_column(col, card_index, ctx);
            self.draw_card(r, *card, rect);
        }
    }

    fn is_being_dragged(&self, col: usize, card_index: usize) -> bool {
        if let DragState::Dragging {
            origin,
            start_index,
            cards,
            ..
        } = &self.game.drag
            && let Zone::Column(c) = origin
            && *c == col
        {
            return card_index >= *start_index && card_index < *start_index + cards.len();
        }
        false
    }

    fn draw_card(&self, r: &mut dyn RenderingServer, card: Card, rect: Rect) {
        let ctx = &self.ctx;
        r.draw_rect(rect, ctx.color_card_bg);
        r.draw_rect_outline(rect, 2.0, ctx.color_card_border);
        let fg = match card.color() {
            CardColor::Red => ctx.color_card_red,
            CardColor::Black => ctx.color_card_black,
        };
        let y = rect.y + 28.0;
        r.draw_text(
            &card.rank_label().to_string(),
            Vec2::new(rect.x + 6.0, y),
            26.0,
            fg,
        );
        let suit = card.symbol().to_string();
        let w = r.measure_text(&suit, 26.0).width;
        r.draw_text(&suit, Vec2::new(rect.x + rect.w - 6.0 - w, y), 26.0, fg);
    }

    fn draw_drop_target(&self, r: &mut dyn RenderingServer) {
        let ctx = &self.ctx;
        let game = &self.game;
        if !game.drag.is_dragging() {
            return;
        }
        let Some(target) = game.zone_at(self.mouse, ctx) else {
            return;
        };
        let rect = match target {
            Zone::FreeCell(i) => layout::free_cell_rect(i, ctx),
            Zone::Foundation(i) => layout::foundation_rect(i, ctx),
            // The whole column, not just the top card.
            Zone::Column(i) => layout::column_bounds(i, ctx),
        };
        let color = if game.can_drop_at(target) {
            DROP_OK
        } else {
            DROP_BAD
        };
        let grown = Rect::new(rect.x - 2.0, rect.y - 2.0, rect.w + 4.0, rect.h + 4.0);
        r.draw_rect_outline(grown, 3.0, color);
    }

    fn draw_hint_overlay(&self, r: &mut dyn RenderingServer) {
        let Some(hint) = self.game.hint else { return };
        self.draw_hint_zone(r, hint.from);
        self.draw_hint_zone(r, hint.to);
        let text = format!(
            "Hint: {}{} → {}",
            hint.card.rank_label(),
            hint.card.symbol(),
            zone_display(hint.to)
        );
        centered(
            r,
            &text,
            self.ctx.window_w * 0.5,
            self.ctx.hud_h + 28.0,
            18.0,
            self.ctx.color_accent,
        );
    }

    fn draw_hint_zone(&self, r: &mut dyn RenderingServer, zone: Zone) {
        let ctx = &self.ctx;
        let rect = match zone {
            Zone::FreeCell(i) => layout::free_cell_rect(i, ctx),
            Zone::Foundation(i) => layout::foundation_rect(i, ctx),
            Zone::Column(i) => match self.game.columns[i].len().checked_sub(1) {
                Some(card_index) => layout::card_rect_in_column(i, card_index, ctx),
                None => {
                    let top = layout::column_top(i, ctx);
                    Rect::new(top.x, top.y, ctx.card_w, ctx.card_h)
                }
            },
        };
        r.draw_rect_outline(rect, 4.0, ctx.color_accent);
    }

    fn draw_drag_overlay(&self, r: &mut dyn RenderingServer) {
        let ctx = &self.ctx;
        let DragState::Dragging { cards, offset, .. } = &self.game.drag else {
            return;
        };
        let base = self.mouse - *offset;
        for (i, card) in cards.iter().enumerate() {
            let rect = Rect::new(base.x, base.y + i as f32 * 20.0, ctx.card_w, ctx.card_h);
            r.draw_rect(
                Rect::new(rect.x + 4.0, rect.y + 4.0, rect.w, rect.h),
                ctx.color_drag_shadow,
            );
            self.draw_card(r, *card, rect);
        }
    }

    fn draw_win_banner(&self, r: &mut dyn RenderingServer) {
        let ctx = &self.ctx;
        let game = &self.game;
        let line1 = format!("SOLVED in {}", format_time(game.elapsed));
        let line2 = format!("Moves: {}", game.moves);
        let line3 = if game.was_new_best {
            "NEW BEST!".to_string()
        } else {
            match game.best_times.get(&game.seed.to_string()) {
                Some(&t) => format!("Best: {}", format_time(t)),
                None => String::new(),
            }
        };
        let d1 = r.measure_text(&line1, 40.0);
        let d2 = r.measure_text(&line2, 24.0);
        let d3 = r.measure_text(&line3, 24.0);
        let cx = ctx.window_w * 0.5;
        let cy = ctx.hud_h + ctx.playfield_h() * 0.5;
        let (pad_x, pad_y) = (40.0, 30.0);
        let box_w = d1.width.max(d2.width).max(d3.width) + pad_x * 2.0;
        let box_h = d1.height + d2.height + d3.height + pad_y * 2.0 + 30.0;
        let rect = Rect::new(cx - box_w * 0.5, cy - box_h * 0.5, box_w, box_h);
        r.draw_rect(rect, BANNER_BG);
        r.draw_rect_outline(rect, 2.0, ctx.color_accent);

        let mut y = rect.y + pad_y + d1.height;
        r.draw_text(
            &line1,
            Vec2::new(cx - d1.width * 0.5, y),
            40.0,
            ctx.color_accent,
        );
        y += d2.height + 15.0;
        r.draw_text(
            &line2,
            Vec2::new(cx - d2.width * 0.5, y),
            24.0,
            ctx.color_text,
        );
        y += d3.height + 10.0;
        if !line3.is_empty() {
            r.draw_text(
                &line3,
                Vec2::new(cx - d3.width * 0.5, y),
                24.0,
                ctx.color_text,
            );
        }
        centered(
            r,
            "R / Space: play again · N: new seed · Tab: menu",
            cx,
            rect.y + rect.h + 30.0,
            16.0,
            ctx.color_text_dim,
        );
    }
}

fn zone_display(zone: Zone) -> String {
    match zone {
        Zone::Column(i) => format!("column {}", i + 1),
        Zone::FreeCell(i) => format!("free cell {}", i + 1),
        Zone::Foundation(i) => format!("foundation {}", i + 1),
    }
}

/// Text centred on `x`; `y` is the baseline.
fn centered(r: &mut dyn RenderingServer, text: &str, x: f32, y: f32, size: f32, color: Color) {
    let w = r.measure_text(text, size).width;
    r.draw_text(text, Vec2::new(x - w * 0.5, y), size, color);
}

impl Object for FreeCellGame {
    fn class_name() -> &'static str {
        "FreeCellGame"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl NodeName for FreeCellGame {
    fn name(&self) -> &str {
        &self.name
    }
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

impl Node for FreeCellGame {
    fn ready(&mut self, ctx: &mut NodeContext) {
        ctx.render().set_clear_color(self.ctx.color_bg);
        self.font_loaded = fonts::load_default_font(ctx.render());
        let me = ctx.current_id();
        ctx.connect::<ButtonClicked>(me, |ctx, ev| {
            let me = ctx.current_id();
            if let Some(m) = ctx
                .tree()
                .get_mut(me)
                .and_then(|n| n.as_any_mut().downcast_mut::<FreeCellGame>())
            {
                m.clicked.push(ev.button);
            }
        });
        self.sync_buttons(ctx.tree());
    }

    fn process(&mut self, dt: f32, ctx: &mut NodeContext) {
        let input = ctx.input().clone();
        self.mouse = input.mouse_pos;
        if input.is_key_pressed(Key::Escape) {
            ctx.quit();
            return;
        }
        // Clicks from the previous frame's UI pass. Focus is dropped so a
        // clicked button does not also react to Enter / Space later.
        let clicked = std::mem::take(&mut self.clicked);
        let any_click = !clicked.is_empty();
        for id in clicked {
            self.on_button(id);
        }
        if any_click
            && let Some(ui) = self.ui_root
            && let Some(root) = ctx
                .tree()
                .get_mut(ui)
                .and_then(|n| n.as_any_mut().downcast_mut::<UiRoot>())
        {
            root.request_focus(None);
        }
        if self.game.state == GameState::Playing {
            self.game.tick(dt.min(MAX_FRAME_DT));
        }
        match self.game.state {
            GameState::Start => self.handle_start_input(&input),
            GameState::Playing if !any_click => self.handle_playing_input(&input),
            GameState::Win => self.handle_win_input(&input),
            _ => {}
        }
        self.sync_buttons(ctx.tree());
    }

    fn draw(&self, ctx: &mut DrawContext) {
        let r = ctx.render();
        match self.game.state {
            GameState::Start => self.draw_start(r),
            GameState::Playing => self.draw_game(r),
            GameState::Win => {
                self.draw_game(r);
                self.draw_win_banner(r);
            }
            _ => {}
        }
    }
}

/// The game's tree for `game` (built with `Game::new`, or with a test
/// best-times handle).
pub fn build_tree(ctx: GameContext, game: Game) -> NodeTree {
    let mut theme = Theme::dark();
    theme.button_bg_normal = ctx.color_btn_idle;
    theme.button_bg_hover = ctx.color_btn_hover;
    theme.button_bg_pressed = ctx.color_btn_pressed;
    theme.button_text_normal = ctx.color_text;
    theme.panel_border = Color::new(1.0, 1.0, 1.0, 0.3);
    let xs: Vec<f32> = (0..4).map(|slot| button_x(&ctx, slot)).collect();
    let mut tree = NodeTree::with_root(FreeCellGame::new(ctx, game));
    let root = tree.root().expect("root");
    let ui = tree.insert(UiRoot::new(theme));
    tree.attach(ui, root).expect("attach");
    let mut button = |text: &str, x: f32| {
        let id = tree.insert(
            Button::new(text)
                .with_size(20.0)
                .with_offset(Vec2::new(x, 10.0))
                .with_min_size(Vec2::new(BUTTON_W, BUTTON_H)),
        );
        tree.attach(id, ui).expect("attach");
        id
    };
    let buttons = Buttons {
        hint: button("Hint", xs[0]),
        restart: button("Restart", xs[1]),
        new_game: button("New", xs[2]),
        menu: button("Menu", xs[3]),
    };
    if let Some(app) = tree
        .get_mut(root)
        .and_then(|n| n.as_any_mut().downcast_mut::<FreeCellGame>())
    {
        app.buttons = Some(buttons);
        app.ui_root = Some(ui);
    }
    tree
}

/// Open the window and play.
pub fn run() {
    let ctx = config::load_config();
    let (w, h) = (ctx.window_w, ctx.window_h);
    let tree = build_tree(ctx, Game::new());
    let game = SceneLoop::new(
        tree,
        MacroquadRenderer::new(),
        MacroquadAudio::new(),
        Rect::new(0.0, 0.0, w, h),
    );
    run_window("FreeCell", w as i32, h as i32, game);
}
