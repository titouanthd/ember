//! Memory as a node tree (Phase 7 migration).
//!
//! ```text
//! MemoryGame              root: owns the untouched `Game`, reads input,
//! │                       draws menu entries, board, header, banner
//! └── UiRoot
//!     ├── START           ember_ui Buttons (were ember_stdlib Buttons
//!     ├── Restart         stored in `Game`); clicks arrive as
//!     └── Menu            `ButtonClicked` and run on the next process
//! ```

use crate::GameState;
use crate::components::{Card, CardState};
use crate::config::{self, GameContext};
use crate::systems::Game;
use ember::prelude::*;

/// Frame time cap, as in the macroquad version.
pub const MAX_FRAME_DT: f32 = 1.0 / 30.0;
const ENTRY_W: f32 = 360.0;
const ENTRY_H: f32 = 60.0;
const ENTRY_GAP: f32 = 16.0;
const MISMATCH: Color = Color::new(1.0, 0.30, 0.35, 1.0);
const BUTTON_TEXT: f32 = 20.0;

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

/// Ids of the three buttons.
#[derive(Debug, Clone, Copy)]
pub struct Buttons {
    pub start: NodeId,
    pub restart: NodeId,
    pub menu: NodeId,
}

pub struct MemoryGame {
    name: String,
    pub ctx: GameContext,
    pub game: Game,
    pub buttons: Option<Buttons>,
    ui_root: Option<NodeId>,
    clicked: Vec<NodeId>,
    /// Pointer position at the last `process`, for hover effects in `draw`.
    mouse: Vec2,
}

impl MemoryGame {
    pub fn new(ctx: GameContext, game: Game) -> Self {
        Self {
            name: "Memory".into(),
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
        let n = self.game.difficulties.len() as f32;
        let total_h = n * ENTRY_H + (n - 1.0).max(0.0) * ENTRY_GAP;
        let first_y = self.ctx.window_h * 0.5 - total_h * 0.5 + 40.0;
        (0..self.game.difficulties.len())
            .map(|i| {
                let y = first_y + i as f32 * (ENTRY_H + ENTRY_GAP);
                Rect::new(cx - ENTRY_W * 0.5, y, ENTRY_W, ENTRY_H)
            })
            .collect()
    }

    /// Top-left of the START button (was `Game::update_menu_layout`).
    pub fn start_button_pos(&self) -> Vec2 {
        let n = self.game.difficulties.len() as f32;
        let total_h = n * ENTRY_H + (n - 1.0).max(0.0) * ENTRY_GAP;
        let first_y = self.ctx.window_h * 0.5 - total_h * 0.5 + 40.0;
        Vec2::new(self.ctx.window_w * 0.5 - 100.0, first_y + total_h + 30.0)
    }

    fn on_button(&mut self, id: NodeId) {
        let Some(b) = self.buttons else { return };
        let in_menu = self.game.state == GameState::Start;
        if (id == b.start && in_menu) || (id == b.restart && !in_menu) {
            self.game.start_game(&self.ctx);
        } else if id == b.menu && !in_menu {
            self.game.state = GameState::Start;
        }
    }

    fn handle_keys(&mut self, input: &Input) {
        let ctx = &self.ctx;
        let game = &mut self.game;
        match game.state {
            GameState::Start => {
                for (i, key) in [Key::Num1, Key::Num2, Key::Num3].into_iter().enumerate() {
                    if input.is_key_pressed(key) && game.difficulties.len() > i {
                        game.selected_difficulty = i;
                    }
                }
                if input.is_key_pressed(Key::Enter) || input.is_key_pressed(Key::Space) {
                    game.start_game(ctx);
                }
            }
            GameState::Playing => {
                if input.is_key_pressed(Key::R) {
                    game.start_game(ctx);
                } else if input.is_key_pressed(Key::M) {
                    game.state = GameState::Start;
                }
            }
            GameState::Win => {
                if input.is_key_pressed(Key::R) || input.is_key_pressed(Key::Space) {
                    game.start_game(ctx);
                } else if input.is_key_pressed(Key::M) {
                    game.state = GameState::Start;
                }
            }
            _ => {}
        }
    }

    fn handle_mouse(&mut self, input: &Input) {
        if !input.mouse_left_pressed {
            return;
        }
        match self.game.state {
            GameState::Start => {
                for (i, r) in self.difficulty_rects().iter().enumerate() {
                    if r.contains(input.mouse_pos) {
                        self.game.selected_difficulty = i;
                    }
                }
            }
            GameState::Playing if self.game.is_playable() => {
                if let Some((col, row)) = self.game.cell_at(input.mouse_pos, &self.ctx) {
                    self.game.reveal(col, row, &self.ctx);
                }
            }
            _ => {}
        }
    }

    /// Buttons shown for the current screen.
    fn sync_buttons(&self, tree: &mut NodeTree) {
        let Some(b) = self.buttons else { return };
        let menu = self.game.state == GameState::Start;
        let playing = matches!(self.game.state, GameState::Playing | GameState::Win);
        for (id, show) in [(b.start, menu), (b.restart, playing), (b.menu, playing)] {
            if let Some(button) = tree
                .get_mut(id)
                .and_then(|n| n.as_any_mut().downcast_mut::<Button>())
            {
                button.base_mut().visible = show;
            }
        }
    }

    // --- Drawing (as the old render_* functions) ------------------------

    fn draw_menu(&self, r: &mut dyn RenderingServer, mouse: Vec2) {
        let ctx = &self.ctx;
        centered(
            r,
            "MEMORY",
            ctx.window_w * 0.5,
            100.0,
            56.0,
            ctx.color_accent,
        );
        centered(
            r,
            "// find the pairs",
            ctx.window_w * 0.5,
            135.0,
            18.0,
            ctx.color_text_dim,
        );
        for (i, rect) in self.difficulty_rects().into_iter().enumerate() {
            let d = &self.game.difficulties[i];
            let bg = if i == self.game.selected_difficulty {
                ctx.color_menu_selected
            } else if rect.contains(mouse) {
                ctx.color_menu_hover
            } else {
                ctx.color_btn_idle
            };
            r.draw_rect(rect, bg);
            r.draw_rect_outline(rect, 1.5, ctx.color_hidden_border);
            let best = match self.game.best_times.get(&d.name) {
                Some(&t) => format!("best: {}", format_time(t)),
                None => "best: --:--".to_owned(),
            };
            r.draw_text(
                &format!("[{}] {}", i + 1, d.name),
                Vec2::new(rect.x + 20.0, rect.y + 26.0),
                24.0,
                ctx.color_text,
            );
            r.draw_text(
                &format!("{}x{} · {} pairs", d.cols, d.rows, d.pairs),
                Vec2::new(rect.x + 20.0, rect.y + 46.0),
                16.0,
                ctx.color_text_dim,
            );
            right(
                r,
                &best,
                rect.x + rect.w - 20.0,
                rect.y + 46.0,
                16.0,
                ctx.color_accent,
            );
        }
    }

    fn draw_game(&self, r: &mut dyn RenderingServer, mouse: Vec2) {
        let ctx = &self.ctx;
        let game = &self.game;
        let w = ctx.window_w;
        r.draw_rect(Rect::new(0.0, 0.0, w, ctx.hud_h), ctx.color_header_bg);
        let d = game.current_difficulty();
        let y = ctx.hud_h * 0.5 + 6.0;
        r.draw_text(
            &format!("{}  ·  {}x{}", d.name, d.cols, d.rows),
            Vec2::new(12.0, y),
            20.0,
            ctx.color_text,
        );
        centered(
            r,
            &format!("pairs: {}/{}", game.pairs_found, game.total_pairs),
            w * 0.5,
            y,
            20.0,
            ctx.color_accent,
        );
        // Timer: right-aligned, ending 20 px before the Restart button.
        let time = format_time(game.elapsed);
        let time_right = w - 260.0;
        let ty = ctx.hud_h * 0.5 + 7.0;
        let label = "time: ";
        let tw = r.measure_text(&time, 20.0).width;
        let lw = r.measure_text(label, 20.0).width;
        r.draw_text(
            label,
            Vec2::new(time_right - tw - lw, ty),
            20.0,
            ctx.color_text_dim,
        );
        let time_color = if game.timer_running {
            ctx.color_text
        } else {
            ctx.color_text_dim
        };
        right(r, &time, time_right, ty, 20.0, time_color);

        let hovered = if game.is_playable() {
            game.cell_at(mouse, ctx)
        } else {
            None
        };
        for row in 0..game.board.height() {
            for col in 0..game.board.width() {
                let card = *game.board.get(col, row).expect("in bounds");
                let rect = game.card_rect(col, row, ctx);
                self.draw_card(
                    r,
                    &card,
                    rect,
                    hovered == Some((col, row)),
                    game.is_resolving_at(col, row),
                );
            }
        }

        let footer = Rect::new(0.0, ctx.window_h - ctx.footer_h, w, ctx.footer_h);
        r.draw_rect(footer, ctx.color_header_bg);
        centered(
            r,
            "R restart · M menu · Esc quit · click cards to flip",
            w * 0.5,
            ctx.window_h - ctx.footer_h * 0.5 + 5.0,
            14.0,
            ctx.color_text_dim,
        );
    }

    fn draw_card(
        &self,
        r: &mut dyn RenderingServer,
        card: &Card,
        rect: Rect,
        hovered: bool,
        error: bool,
    ) {
        let ctx = &self.ctx;
        let (bg, border) = match card.state {
            CardState::Hidden if hovered => (ctx.color_hidden_hover, ctx.color_hidden_border),
            CardState::Hidden => (ctx.color_hidden, ctx.color_hidden_border),
            CardState::Flipped if error => (ctx.color_flipped, MISMATCH),
            CardState::Flipped => (ctx.color_flipped, ctx.color_flipped_border),
            CardState::Matched => (ctx.color_matched, ctx.color_matched_border),
        };
        r.draw_rect(rect, bg);
        r.draw_rect_outline(rect, 2.0, border);
        let (text, color) = match card.state {
            CardState::Hidden => ("?".to_string(), ctx.color_hidden_symbol),
            CardState::Flipped => (card.symbol.as_char().to_string(), ctx.color_symbol),
            CardState::Matched => (card.symbol.as_char().to_string(), ctx.color_matched_border),
        };
        let size = (rect.w * 0.55).clamp(16.0, 48.0).floor();
        let m = r.measure_text(&text, size);
        let x = rect.x + (rect.w - m.width) * 0.5;
        let y = rect.y + (rect.h + m.height) * 0.5 - size * 0.1;
        r.draw_text(&text, Vec2::new(x, y), size, color);
    }

    fn draw_win_banner(&self, r: &mut dyn RenderingServer) {
        let ctx = &self.ctx;
        let game = &self.game;
        let line1 = format!("SOLVED in {}", format_time(game.elapsed));
        let line2 = if game.was_new_best {
            "NEW BEST!".to_string()
        } else {
            match game.best_times.get(&game.current_difficulty().name) {
                Some(&t) => format!("best: {}", format_time(t)),
                None => String::new(),
            }
        };
        let d1 = r.measure_text(&line1, 40.0);
        let d2 = r.measure_text(&line2, 20.0);
        let cx = ctx.window_w * 0.5;
        let cy = ctx.hud_h + ctx.playfield_h() * 0.5;
        let (pad_x, pad_y) = (30.0, 25.0);
        let box_w = d1.width.max(d2.width) + pad_x * 2.0;
        let box_h = d1.height + d2.height + pad_y * 2.0 + 10.0;
        let rect = Rect::new(cx - box_w * 0.5, cy - box_h * 0.5, box_w, box_h);
        r.draw_rect(rect, Color::new(0.0, 0.0, 0.0, 0.85));
        r.draw_rect_outline(rect, 2.0, ctx.color_matched_border);
        r.draw_text(
            &line1,
            Vec2::new(cx - d1.width * 0.5, rect.y + pad_y + d1.height),
            40.0,
            ctx.color_matched_border,
        );
        r.draw_text(
            &line2,
            Vec2::new(
                cx - d2.width * 0.5,
                rect.y + pad_y + d1.height + 10.0 + d2.height,
            ),
            20.0,
            ctx.color_text,
        );
        centered(
            r,
            "R or Space to play again",
            cx,
            rect.y + box_h + 30.0,
            16.0,
            ctx.color_text_dim,
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

impl Object for MemoryGame {
    fn class_name() -> &'static str {
        "MemoryGame"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl NodeName for MemoryGame {
    fn name(&self) -> &str {
        &self.name
    }
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

impl Node for MemoryGame {
    fn ready(&mut self, ctx: &mut NodeContext) {
        ctx.render().set_clear_color(self.ctx.color_bg);
        let me = ctx.current_id();
        ctx.connect::<ButtonClicked>(me, |ctx, ev| {
            let me = ctx.current_id();
            if let Some(m) = ctx
                .tree()
                .get_mut(me)
                .and_then(|n| n.as_any_mut().downcast_mut::<MemoryGame>())
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
        // clicked button does not also react to Space / Enter later.
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
        self.handle_keys(&input);
        self.handle_mouse(&input);
        self.sync_buttons(ctx.tree());
    }

    fn draw(&self, ctx: &mut DrawContext) {
        let mouse = self.mouse;
        let r = ctx.render();
        match self.game.state {
            GameState::Start => self.draw_menu(r, mouse),
            GameState::Playing | GameState::Win => {
                self.draw_game(r, mouse);
                if self.game.state == GameState::Win {
                    self.draw_win_banner(r);
                }
            }
            _ => {}
        }
    }
}

/// The game's tree for `game` (built with `Game::new` or
/// `Game::with_best_handle`).
pub fn build_tree(ctx: GameContext, game: Game) -> NodeTree {
    let mut theme = Theme::dark();
    theme.button_bg_normal = ctx.color_btn_idle;
    theme.button_bg_hover = ctx.color_btn_hover;
    theme.button_bg_pressed = ctx.color_btn_pressed;
    theme.button_text_normal = ctx.color_text;
    theme.panel_border = Color::new(1.0, 1.0, 1.0, 0.3);
    let w = ctx.window_w;
    let app = MemoryGame::new(ctx, game);
    let start_pos = app.start_button_pos();
    let mut tree = NodeTree::with_root(app);
    let root = tree.root().expect("root");
    let ui = tree.insert(UiRoot::new(theme));
    tree.attach(ui, root).expect("attach");
    let mut button = |text: &str, pos: Vec2, size: Vec2| {
        let id = tree.insert(
            Button::new(text)
                .with_size(BUTTON_TEXT)
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
            Vec2::new(w - 240.0, 10.0),
            Vec2::new(100.0, 30.0),
        ),
        menu: button("Menu", Vec2::new(w - 120.0, 10.0), Vec2::new(100.0, 30.0)),
    };
    if let Some(app) = tree
        .get_mut(root)
        .and_then(|n| n.as_any_mut().downcast_mut::<MemoryGame>())
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
    run_window("Memory", w as i32, h as i32, game);
}
