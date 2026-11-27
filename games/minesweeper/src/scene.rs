//! Minesweeper as a node tree (Phase 7 migration).
//!
//! ```text
//! MinesweeperGame         root: owns the untouched `Game`, runs the old
//! │                       input handlers, draws menu, board, header,
//! │                       footer and end banners
//! └── UiRoot
//!     ├── START           ember_ui Buttons (were ember_stdlib Buttons
//!     ├── Restart         stored in `Game`); clicks arrive as
//!     └── Menu            `ButtonClicked` and run on the next process
//! ```

use crate::GameState;
use crate::components::{BoardState, Cell, CellState};
use crate::config::{self, GameContext};
use crate::systems::{self, Game};
use ember::prelude::*;

/// Frame time cap, as in the macroquad version.
pub const MAX_FRAME_DT: f32 = 1.0 / 30.0;
const ENTRY_W: f32 = 360.0;
const ENTRY_H: f32 = 60.0;
const ENTRY_GAP: f32 = 16.0;
const BLACK: Color = Color::new(0.0, 0.0, 0.0, 1.0);
const WIN: Color = Color::new(0.4, 1.0, 0.5, 1.0);
const LOSE: Color = Color::new(1.0, 0.35, 0.4, 1.0);

/// `mm:ss` (or `h:mm:ss`).
pub use ember_stdlib::clock::format_time;

/// Colour of an adjacent-mines number.
pub fn number_color(n: u8) -> Color {
    match n {
        1 => Color::new(0.35, 0.65, 1.00, 1.0),
        2 => Color::new(0.35, 0.85, 0.45, 1.0),
        3 => Color::new(1.00, 0.40, 0.40, 1.0),
        4 => Color::new(0.60, 0.40, 1.00, 1.0),
        5 => Color::new(1.00, 0.60, 0.20, 1.0),
        6 => Color::new(0.30, 0.85, 0.85, 1.0),
        7 => Color::new(0.95, 0.95, 0.95, 1.0),
        8 => Color::new(0.60, 0.60, 0.60, 1.0),
        _ => Color::new(0.5, 0.5, 0.5, 1.0),
    }
}

/// Ids of the three buttons.
#[derive(Debug, Clone, Copy)]
pub struct Buttons {
    pub start: NodeId,
    pub restart: NodeId,
    pub menu: NodeId,
}

pub struct MinesweeperGame {
    name: String,
    pub ctx: GameContext,
    pub game: Game,
    pub buttons: Option<Buttons>,
    ui_root: Option<NodeId>,
    clicked: Vec<NodeId>,
    /// Pointer position at the last `process`, for hover effects in `draw`.
    mouse: Vec2,
}

impl MinesweeperGame {
    pub fn new(ctx: GameContext, game: Game) -> Self {
        Self {
            name: "Minesweeper".into(),
            ctx,
            game,
            buttons: None,
            ui_root: None,
            clicked: Vec::new(),
            mouse: Vec2::ZERO,
        }
    }

    /// Menu entries, one per difficulty (unchanged layout).
    pub fn difficulty_rects(&self) -> Vec<Rect> {
        let cx = self.ctx.window_w * 0.5;
        let first_y = self.first_entry_y();
        (0..self.game.difficulties.len())
            .map(|i| {
                let y = first_y + i as f32 * (ENTRY_H + ENTRY_GAP);
                Rect::new(cx - ENTRY_W * 0.5, y, ENTRY_W, ENTRY_H)
            })
            .collect()
    }

    fn entries_h(&self) -> f32 {
        let n = self.game.difficulties.len() as f32;
        n * ENTRY_H + (n - 1.0).max(0.0) * ENTRY_GAP
    }

    fn first_entry_y(&self) -> f32 {
        self.ctx.window_h * 0.5 - self.entries_h() * 0.5 + 40.0
    }

    /// Top-left of the START button (was `Game::update_menu_layout`).
    pub fn start_button_pos(&self) -> Vec2 {
        Vec2::new(
            self.ctx.window_w * 0.5 - 100.0,
            self.first_entry_y() + self.entries_h() + 30.0,
        )
    }

    /// Screen rect of a board cell.
    pub fn cell_rect(&self, col: usize, row: usize) -> Rect {
        let s = self.game.cell_size;
        Rect::new(
            self.game.grid_origin.x + col as f32 * s,
            self.game.grid_origin.y + row as f32 * s,
            s,
            s,
        )
    }

    fn on_button(&mut self, id: NodeId) {
        let Some(b) = self.buttons else { return };
        let ctx = &self.ctx;
        let game = &mut self.game;
        let playing = game.state == GameState::Playing;
        if (id == b.start && game.state == GameState::Start) || (id == b.restart && playing) {
            game.start_game(ctx);
        } else if id == b.menu && playing {
            game.state = GameState::Start;
        }
    }

    fn handle_menu_input(&mut self, input: &Input) {
        let game = &mut self.game;
        for (i, key) in [Key::Num1, Key::Num2, Key::Num3].into_iter().enumerate() {
            if input.is_key_pressed(key) && game.difficulties.len() > i {
                game.selected_difficulty = i;
            }
        }
        if input.mouse_left_pressed {
            for (i, r) in self.difficulty_rects().iter().enumerate() {
                if r.contains(input.mouse_pos) {
                    self.game.selected_difficulty = i;
                }
            }
        }
        if input.is_key_pressed(Key::Enter) {
            self.game.start_game(&self.ctx);
        }
    }

    fn handle_playing_input(&mut self, input: &Input, dt: f32) {
        let ctx = &self.ctx;
        let game = &mut self.game;
        if game.timer_running && game.board.state == BoardState::Playing {
            game.elapsed += dt;
        }
        if input.is_key_pressed(Key::Escape) {
            game.state = GameState::Start;
            return;
        }
        if input.is_key_pressed(Key::R) {
            game.start_game(ctx);
            return;
        }
        if game.is_playable()
            && let Some((col, row)) = game.cell_at(input.mouse_pos)
        {
            if input.mouse_left_pressed {
                if !game.board.mines_placed {
                    systems::place_mines(&mut game.board, col, row, &mut game.rng);
                    game.timer_running = true;
                    systems::reveal(&mut game.board, col, row);
                } else {
                    let cell = *game.board.cells.get(col, row).expect("in bounds");
                    if cell.is_revealed() {
                        systems::chord(&mut game.board, col, row);
                    } else {
                        systems::reveal(&mut game.board, col, row);
                    }
                }
            }
            if input.mouse_middle_pressed {
                systems::chord(&mut game.board, col, row);
            }
            if input.mouse_right_pressed {
                systems::toggle_flag(&mut game.board, col, row);
            }
        }
        match game.board.state {
            BoardState::Won => {
                game.timer_running = false;
                game.record_best_if_needed();
                game.state = GameState::Win;
            }
            BoardState::Lost => {
                game.timer_running = false;
                game.state = GameState::GameOver;
            }
            _ => {}
        }
    }

    fn handle_end_input(&mut self, input: &Input) {
        if input.is_key_pressed(Key::Escape) || input.is_key_pressed(Key::R) {
            self.game.state = GameState::Start;
        } else if input.is_key_pressed(Key::Enter) {
            self.game.start_game(&self.ctx);
        }
    }

    /// START on the menu; Restart / Menu on the board (drawn on the end
    /// screens too, as before, but only active while playing).
    fn sync_buttons(&self, tree: &mut NodeTree) {
        let Some(b) = self.buttons else { return };
        let state = self.game.state;
        let on_board = matches!(
            state,
            GameState::Playing | GameState::Win | GameState::GameOver
        );
        for (id, show, active) in [
            (b.start, state == GameState::Start, true),
            (b.restart, on_board, state == GameState::Playing),
            (b.menu, on_board, state == GameState::Playing),
        ] {
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

    fn draw_menu(&self, r: &mut dyn RenderingServer) {
        let ctx = &self.ctx;
        centered(
            r,
            "MINESWEEPER",
            ctx.window_w * 0.5,
            100.0,
            56.0,
            ctx.color_text,
        );
        for (i, rect) in self.difficulty_rects().into_iter().enumerate() {
            let d = &self.game.difficulties[i];
            let bg = if i == self.game.selected_difficulty {
                ctx.color_menu_selected
            } else if rect.contains(self.mouse) {
                ctx.color_menu_hover
            } else {
                ctx.color_btn_idle
            };
            r.draw_rect(rect, bg);
            r.draw_rect_outline(rect, 1.5, ctx.color_grid_line);
            let best = match self.game.best_times.get(&d.name) {
                Some(&t) => format!("Best: {}", format_time(t)),
                None => "Best: --:--".to_owned(),
            };
            r.draw_text(
                &d.name,
                Vec2::new(rect.x + 20.0, rect.y + 22.0),
                22.0,
                ctx.color_text,
            );
            r.draw_text(
                &format!("{}×{} · {} mines", d.width, d.height, d.mines),
                Vec2::new(rect.x + 20.0, rect.y + 42.0),
                16.0,
                ctx.color_text,
            );
            right(
                r,
                &best,
                rect.x + rect.w - 20.0,
                rect.y + 42.0,
                16.0,
                ctx.color_text,
            );
        }
    }

    fn draw_game(&self, r: &mut dyn RenderingServer) {
        let ctx = &self.ctx;
        let game = &self.game;
        let w = ctx.window_w;
        let y = ctx.hud_h * 0.5 + 6.0;
        r.draw_rect(Rect::new(0.0, 0.0, w, ctx.hud_h), ctx.color_header_bg);
        let d = game.current_difficulty();
        r.draw_text(
            &format!("{} · {}×{} · {} mines", d.name, d.width, d.height, d.mines),
            Vec2::new(12.0, y),
            20.0,
            ctx.color_text,
        );
        r.draw_text(
            &format!("Mines: {}", game.board.mines_remaining()),
            Vec2::new(380.0, y),
            20.0,
            ctx.color_text,
        );
        right(
            r,
            &format_time(game.elapsed),
            600.0,
            y,
            20.0,
            ctx.color_text,
        );

        let hovered = game.cell_at(self.mouse);
        let s = game.cell_size;
        for row in 0..game.board.height {
            for col in 0..game.board.width {
                let cell = *game.board.cells.get(col, row).expect("in bounds");
                let rect = self.cell_rect(col, row);
                let hover = hovered == Some((col, row)) && game.is_playable();
                r.draw_rect(rect, self.cell_background(&cell, hover));
                r.draw_rect_outline(rect, 1.0, ctx.color_grid_line);
                if cell.is_revealed() && !cell.is_mine && cell.adjacent > 0 {
                    let txt = cell.adjacent.to_string();
                    let size = (s * 0.6).clamp(12.0, 26.0);
                    let m = r.measure_text(&txt, size);
                    let pos = Vec2::new(
                        rect.x + (s - m.width) * 0.5,
                        rect.y + (s + size) * 0.5 - 4.0,
                    );
                    r.draw_text(&txt, pos, size, number_color(cell.adjacent));
                }
                if cell.is_mine && game.board.state == BoardState::Lost && !cell.is_flagged() {
                    r.draw_circle(
                        Vec2::new(rect.x + s * 0.5, rect.y + s * 0.5),
                        s * 0.2,
                        BLACK,
                    );
                }
            }
        }

        let footer = Rect::new(0.0, ctx.window_h - ctx.footer_h, w, ctx.footer_h);
        r.draw_rect(footer, ctx.color_header_bg);
        centered(
            r,
            "R restart · Esc menu · left-click reveal · right-click flag · middle-click chord",
            w * 0.5,
            ctx.window_h - ctx.footer_h * 0.5 + 5.0,
            14.0,
            ctx.color_text,
        );

        match game.state {
            GameState::Win => {
                let line1 = format!("YOU WIN — {}", format_time(game.elapsed));
                let line2 = if game.was_new_best {
                    "NEW BEST!".to_string()
                } else {
                    match game.best_times.get(&d.name) {
                        Some(&t) => format!("Best: {}", format_time(t)),
                        None => String::new(),
                    }
                };
                self.draw_end_banner(r, &line1, &line2, WIN);
            }
            GameState::GameOver => {
                self.draw_end_banner(r, "GAME OVER", "Press R for menu, Enter to retry", LOSE);
            }
            _ => {}
        }
    }

    fn cell_background(&self, cell: &Cell, hovered: bool) -> Color {
        let ctx = &self.ctx;
        match cell.state {
            CellState::Hidden if hovered => ctx.color_cell_hover,
            CellState::Hidden => ctx.color_cell_hidden,
            CellState::Revealed if cell.is_mine && self.game.board.state == BoardState::Lost => {
                ctx.color_cell_mine
            }
            CellState::Revealed => ctx.color_cell_revealed,
            CellState::Flagged => ctx.color_flag,
        }
    }

    fn draw_end_banner(&self, r: &mut dyn RenderingServer, line1: &str, line2: &str, color: Color) {
        let ctx = &self.ctx;
        let d1 = r.measure_text(line1, 48.0);
        let d2 = r.measure_text(line2, 20.0);
        let cx = ctx.window_w * 0.5;
        let cy = ctx.hud_h + ctx.playfield_h() * 0.5;
        let (pad_x, pad_y) = (30.0, 25.0);
        let box_w = d1.width.max(d2.width) + pad_x * 2.0;
        let box_h = d1.height + d2.height + pad_y * 2.0 + 10.0;
        let rect = Rect::new(cx - box_w * 0.5, cy - box_h * 0.5, box_w, box_h);
        r.draw_rect(rect, Color::new(0.0, 0.0, 0.0, 0.82));
        r.draw_text(
            line1,
            Vec2::new(cx - d1.width * 0.5, rect.y + pad_y + d1.height),
            48.0,
            color,
        );
        r.draw_text(
            line2,
            Vec2::new(
                cx - d2.width * 0.5,
                rect.y + pad_y + d1.height + 10.0 + d2.height,
            ),
            20.0,
            ctx.color_text,
        );
    }
}

/// Text centred on `x`; `y` is the baseline.
fn centered(r: &mut dyn RenderingServer, text: &str, x: f32, y: f32, size: f32, color: Color) {
    let w = r.measure_text(text, size).width;
    r.draw_text(text, Vec2::new(x - w * 0.5, y), size, color);
}

/// Text ending at `x`; `y` is the baseline.
fn right(r: &mut dyn RenderingServer, text: &str, x: f32, y: f32, size: f32, color: Color) {
    let w = r.measure_text(text, size).width;
    r.draw_text(text, Vec2::new(x - w, y), size, color);
}

impl Object for MinesweeperGame {
    fn class_name() -> &'static str {
        "MinesweeperGame"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl NodeName for MinesweeperGame {
    fn name(&self) -> &str {
        &self.name
    }
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

impl Node for MinesweeperGame {
    fn ready(&mut self, ctx: &mut NodeContext) {
        ctx.render().set_clear_color(self.ctx.color_bg);
        let me = ctx.current_id();
        ctx.connect::<ButtonClicked>(me, |ctx, ev| {
            let me = ctx.current_id();
            if let Some(m) = ctx
                .tree()
                .get_mut(me)
                .and_then(|n| n.as_any_mut().downcast_mut::<MinesweeperGame>())
            {
                m.clicked.push(ev.button);
            }
        });
        self.sync_buttons(ctx.tree());
    }

    fn process(&mut self, dt: f32, ctx: &mut NodeContext) {
        let input = ctx.input().clone();
        self.mouse = input.mouse_pos;
        // Clicks from the previous frame's UI pass. Focus is dropped so a
        // clicked button does not also react to Enter later.
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
        match self.game.state {
            GameState::Start => self.handle_menu_input(&input),
            GameState::Playing => self.handle_playing_input(&input, dt.min(MAX_FRAME_DT)),
            GameState::Win | GameState::GameOver => self.handle_end_input(&input),
            _ => {}
        }
        self.sync_buttons(ctx.tree());
    }

    fn draw(&self, ctx: &mut DrawContext) {
        let r = ctx.render();
        match self.game.state {
            GameState::Start => self.draw_menu(r),
            GameState::Playing | GameState::Win | GameState::GameOver => self.draw_game(r),
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
    let w = ctx.window_w;
    let app = MinesweeperGame::new(ctx, game);
    let start_pos = app.start_button_pos();
    let mut tree = NodeTree::with_root(app);
    let root = tree.root().expect("root");
    let ui = tree.insert(UiRoot::new(theme));
    tree.attach(ui, root).expect("attach");
    let mut button = |text: &str, pos: Vec2, size: Vec2| {
        let id = tree.insert(
            Button::new(text)
                .with_size(20.0)
                .with_offset(pos)
                .with_min_size(size),
        );
        tree.attach(id, ui).expect("attach");
        id
    };
    let buttons = Buttons {
        start: button("START", start_pos, Vec2::new(200.0, 50.0)),
        restart: button(
            "Restart",
            Vec2::new(w - 220.0, 10.0),
            Vec2::new(100.0, 30.0),
        ),
        menu: button("Menu", Vec2::new(w - 110.0, 10.0), Vec2::new(100.0, 30.0)),
    };
    if let Some(app) = tree
        .get_mut(root)
        .and_then(|n| n.as_any_mut().downcast_mut::<MinesweeperGame>())
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
    run_window("Minesweeper", w as i32, h as i32, game);
}
