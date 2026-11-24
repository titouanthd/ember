//! Simon as a node tree (Phase 7 migration, ADR-033).
//!
//! ```text
//! Simon                    root: owns the GameWorld, input, sounds, best score
//! └── UiRoot
//!     ├── Start            title, prompt, best, hint
//!     ├── Play             header (score / best), 4 ColorPads, status
//!     └── GameOver         centred box: title, score line, hint
//! ```
//!
//! The rules in `systems` are untouched. `Simon::process` ticks them and
//! pushes the result into the UI nodes; a `ColorPad` click arrives as a
//! [`PadClicked`] signal emitted by `UiRoot` (ADR-036).

use crate::GameState;
use crate::components::{SimonColor, button_centers};
use crate::config::GameContext;
use crate::persistence::BestScore;
use crate::systems::{ClickResult, GameWorld, PlayPhase, handle_click, tick_playback};
use ember::prelude::*;
use ember_servers::AudioClipId;

/// Frame time cap, as in the macroquad version.
pub const MAX_FRAME_DT: f32 = 1.0 / 30.0;
/// RNG seed of the macroquad version, kept for identical sequences.
pub const SEED: u32 = 0xDEAD_BEEF;

const SOUNDS: [&[u8]; 4] = [
    include_bytes!("../sounds/green.wav"),
    include_bytes!("../sounds/red.wav"),
    include_bytes!("../sounds/yellow.wav"),
    include_bytes!("../sounds/blue.wav"),
];

/// A colour pad was clicked while the game accepted input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PadClicked {
    pub color: SimonColor,
}

// --- ColorPad ---------------------------------------------------------

/// One of the four round Simon buttons: a custom `ember_ui` control.
/// Hit only inside its circle; clicks on left release (like the old
/// `CircleButton`) while `enabled`.
pub struct ColorPad {
    name: String,
    base: ControlBase,
    pub color: SimonColor,
    pub lit_color: Color,
    pub dim_color: Color,
    /// Set by `Simon` each frame: lit by the sequence or the player.
    pub lit: bool,
    /// Set by `Simon` each frame: the game accepts clicks.
    pub enabled: bool,
}

impl ColorPad {
    pub fn new(color: SimonColor, center: Vec2, radius: f32, lit: Color, dim: Color) -> Self {
        let size = Vec2::splat(radius * 2.0);
        Self {
            name: format!("{color:?}Pad"),
            base: ControlBase::new()
                .with_offset(center - Vec2::splat(radius))
                .with_min_size(size),
            color,
            lit_color: lit,
            dim_color: dim,
            lit: false,
            enabled: false,
        }
    }

    fn center_radius(&self) -> (Vec2, f32) {
        let r = self.rect();
        (Vec2::new(r.x + r.w * 0.5, r.y + r.h * 0.5), r.w * 0.5)
    }

    /// Drawn bright: lit, or hovered while clickable.
    pub fn is_bright(&self) -> bool {
        self.lit || (self.enabled && self.base.hovered)
    }
}

impl Object for ColorPad {
    fn class_name() -> &'static str {
        "ColorPad"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl NodeName for ColorPad {
    fn name(&self) -> &str {
        &self.name
    }
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

impl Control for ColorPad {
    fn base(&self) -> &ControlBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ControlBase {
        &mut self.base
    }

    fn hit_test(&self, pos: Vec2) -> bool {
        let (c, r) = self.center_radius();
        let d = pos - c;
        d.x * d.x + d.y * d.y <= r * r
    }

    fn on_mouse_up(&mut self, button: MouseButton, _pos: Vec2) {
        if self.enabled && button == MouseButton::Left {
            let color = self.color;
            self.base.emit(move |_| PadClicked { color });
        }
    }

    fn draw_control(&self, ui: &mut UiDrawContext) {
        let (c, r) = self.center_radius();
        let fill = if self.is_bright() {
            self.lit_color
        } else {
            self.dim_color
        };
        let render = ui.render();
        render.draw_circle(c, r, fill);
        render.draw_circle_outline(c, r, 1.5, Color::new(1.0, 1.0, 1.0, 0.15));
    }
}

control_node!(ColorPad);

// --- Simon (root) ------------------------------------------------------

/// Ids of the UI nodes `Simon` updates every frame.
#[derive(Debug, Clone, Copy)]
pub struct Ui {
    pub start: NodeId,
    pub start_best: NodeId,
    pub play: NodeId,
    pub score: NodeId,
    pub best: NodeId,
    pub status: NodeId,
    pub pads: [NodeId; 4],
    pub game_over: NodeId,
    pub game_over_line: NodeId,
}

pub struct Simon {
    name: String,
    cfg: GameContext,
    pub state: GameState,
    pub world: GameWorld,
    store: Option<BestScore>,
    sounds: [Option<AudioClipId>; 4],
    /// Lit colour at the last sound, so playback sounds once per light.
    last_lit: Option<SimonColor>,
    ui: Option<Ui>,
}

impl Simon {
    /// `store` = `None` keeps the best score in memory only (tests).
    pub fn new(cfg: GameContext, store: Option<BestScore>) -> Self {
        let best = store.as_ref().map_or(0, |s| s.load_or(0));
        Self {
            name: "Simon".into(),
            cfg,
            state: GameState::Start,
            world: GameWorld::new(SEED, best),
            store,
            sounds: [None; 4],
            last_lit: None,
            ui: None,
        }
    }

    /// Ids of the UI nodes (set by [`build_tree`]).
    pub fn ui(&self) -> Option<Ui> {
        self.ui
    }

    fn restart(&mut self) {
        self.world.restart();
        self.state = GameState::Playing;
        self.last_lit = None;
    }

    /// Save the best score if the current run beats it.
    fn record_best_if_needed(&mut self) {
        let prev = self
            .store
            .as_ref()
            .map_or(self.world.best, |s| s.load_or(0));
        let new_best = self.world.best.max(self.world.score);
        if new_best > prev {
            if let Some(store) = &self.store {
                store.save(&new_best);
            }
            self.world.best = new_best;
        }
    }

    fn clip(&self, color: SimonColor) -> Option<AudioClipId> {
        self.sounds[color.index()]
    }

    /// A pad click (from `PadClicked`), as in the old main loop. Returns
    /// the sound to play.
    pub fn click(&mut self, color: SimonColor) -> Option<AudioClipId> {
        if self.state != GameState::Playing || !self.world.phase.is_accepting_input() {
            return None;
        }
        match handle_click(&mut self.world, color) {
            ClickResult::RoundComplete => {
                self.record_best_if_needed();
                self.world.next_round();
                self.last_lit = None;
            }
            ClickResult::Wrong => {
                self.record_best_if_needed();
                self.state = GameState::GameOver;
                self.last_lit = None;
            }
            ClickResult::Correct | ClickResult::Ignored => {}
        }
        self.clip(color)
    }

    /// The sound to play when the lit colour changes to a new one.
    fn sync_playback_sound(&mut self) -> Option<AudioClipId> {
        let current = self.world.lit_color();
        if current == self.last_lit {
            return None;
        }
        self.last_lit = current;
        current.and_then(|c| self.clip(c))
    }

    /// Push the game state into the UI nodes.
    fn sync_ui(&self, ctx: &mut NodeContext) {
        let Some(ui) = self.ui else { return };
        let playing = matches!(self.state, GameState::Playing | GameState::GameOver);
        let tree = ctx.tree();
        set_visible(tree, ui.start, self.state == GameState::Start);
        set_visible(tree, ui.play, playing);
        set_visible(tree, ui.game_over, self.state == GameState::GameOver);
        set_text(tree, ui.start_best, format!("Best: {}", self.world.best));
        set_text(tree, ui.score, format!("Score: {}", self.world.score));
        set_text(tree, ui.best, format!("Best: {}", self.world.best));
        let status = match self.world.phase {
            PlayPhase::PreRound { .. } => "Get ready...",
            PlayPhase::Showing { .. } => "Watch...",
            PlayPhase::Waiting { .. } => "Your turn",
        };
        set_text(tree, ui.status, status);
        set_text(
            tree,
            ui.game_over_line,
            format!("Score: {}  ·  Best: {}", self.world.score, self.world.best),
        );
        let lit = self.world.lit_color();
        let accepting = self.state == GameState::Playing && self.world.phase.is_accepting_input();
        for (i, &id) in ui.pads.iter().enumerate() {
            if let Some(pad) = node_mut::<ColorPad>(tree, id) {
                pad.lit = lit == SimonColor::from_index(i);
                pad.enabled = accepting;
            }
        }
    }
}

impl Object for Simon {
    fn class_name() -> &'static str {
        "Simon"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl NodeName for Simon {
    fn name(&self) -> &str {
        &self.name
    }
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

impl Node for Simon {
    fn ready(&mut self, ctx: &mut NodeContext) {
        ctx.render().set_clear_color(self.cfg.color_bg);
        for (slot, bytes) in self.sounds.iter_mut().zip(SOUNDS) {
            match ctx.audio().load_clip(bytes) {
                Ok(clip) => *slot = Some(clip),
                Err(e) => eprintln!("simon: cannot load a sound: {e:?}"),
            }
        }
        let me = ctx.current_id();
        ctx.connect::<PadClicked>(me, |ctx, ev| {
            let me = ctx.current_id();
            let clip = node_mut::<Simon>(ctx.tree(), me).and_then(|s| s.click(ev.color));
            if let Some(clip) = clip {
                ctx.audio().play(clip, 1.0, 1.0);
            }
        });
        self.sync_ui(ctx);
    }

    fn process(&mut self, dt: f32, ctx: &mut NodeContext) {
        let dt = dt.min(MAX_FRAME_DT);
        let input = ctx.input();
        let space = input.is_key_pressed(Key::Space);
        let r = input.is_key_pressed(Key::R);
        match self.state {
            GameState::Start if space => self.restart(),
            GameState::Playing => {
                tick_playback(&mut self.world, dt);
                if let Some(clip) = self.sync_playback_sound() {
                    ctx.audio().play(clip, 1.0, 1.0);
                }
            }
            GameState::GameOver if r || space => self.state = GameState::Start,
            _ => {}
        }
        self.sync_ui(ctx);
    }
}

// --- Tree helpers ------------------------------------------------------

fn node_mut<T: 'static>(tree: &mut NodeTree, id: NodeId) -> Option<&mut T> {
    tree.get_mut(id)?.as_any_mut().downcast_mut::<T>()
}

fn set_visible(tree: &mut NodeTree, id: NodeId, visible: bool) {
    if let Some(n) = tree.get_mut(id)
        && let Some(p) = n.as_any_mut().downcast_mut::<Panel>()
    {
        p.base_mut().visible = visible;
    }
}

fn set_text(tree: &mut NodeTree, id: NodeId, text: impl Into<String>) {
    if let Some(label) = node_mut::<Label>(tree, id) {
        let text = text.into();
        if label.text != text {
            label.set_text(text);
        }
    }
}

// --- Building the tree -------------------------------------------------

fn add(tree: &mut NodeTree, parent: NodeId, node: impl Node) -> NodeId {
    let id = tree.insert(node);
    tree.attach(id, parent).expect("attach");
    id
}

/// A full-window container that draws nothing.
fn group() -> Panel {
    Panel::new()
        .with_background(Color::TRANSPARENT)
        .with_border(0.0, None)
        .with_anchors(Anchors::FULL_RECT)
}

/// A label horizontally centred across its parent, its text centred on
/// `y` (pixels from the parent's top).
fn centered(text: impl Into<String>, y: f32, size: f32, color: Color) -> Label {
    Label::new(text)
        .with_size(size)
        .with_color(color)
        .with_align(Alignment::Center)
        .with_anchors(Anchors::TOP_WIDE)
        .with_offset(Vec2::new(0.0, y - size * 0.5))
        .with_min_size(Vec2::new(0.0, size))
}

/// Build the Simon tree. `store` = `None` for tests (no file writes).
pub fn build_tree(cfg: GameContext, store: Option<BestScore>) -> NodeTree {
    let (w, h) = (cfg.window_w, cfg.window_h);
    let text = cfg.color_text;
    let mut tree = NodeTree::with_root(Simon::new(cfg.clone(), store));
    let root = tree.root().expect("root");
    let mut theme = Theme::dark();
    theme.text = text;
    let ui_root = add(&mut tree, root, UiRoot::new(theme).register::<ColorPad>());

    // Start screen
    let start = add(&mut tree, ui_root, group());
    add(&mut tree, start, centered("SIMON", h * 0.35, 72.0, text));
    add(
        &mut tree,
        start,
        centered("PRESS SPACE TO START", h * 0.5, 28.0, text),
    );
    let start_best = add(&mut tree, start, centered("Best: 0", h * 0.6, 22.0, text));
    add(
        &mut tree,
        start,
        centered(
            "Watch the sequence, then repeat it by clicking.",
            h * 0.7,
            16.0,
            text,
        ),
    );

    // Playfield
    let play = add(&mut tree, ui_root, group().hidden());
    let header = add(
        &mut tree,
        play,
        Panel::new()
            .with_background(cfg.color_header_bg)
            .with_border(0.0, None)
            .with_padding(Margins::symmetric(20.0, 0.0))
            .with_anchors(Anchors::TOP_WIDE)
            .with_min_size(Vec2::new(0.0, cfg.hud_h)),
    );
    let header_label = |s: &str, align| {
        Label::new(s)
            .with_size(24.0)
            .with_color(text)
            .with_align(align)
            .with_anchors(Anchors::FULL_RECT)
    };
    let score = add(&mut tree, header, header_label("Score: 0", Alignment::Left));
    let best = add(&mut tree, header, header_label("Best: 0", Alignment::Right));
    let centers = button_centers(&cfg);
    let pads = std::array::from_fn(|i| {
        let color = SimonColor::from_index(i).expect("4 colours");
        let (lit, dim) = colors_for(color, &cfg);
        add(
            &mut tree,
            play,
            ColorPad::new(color, centers[i], cfg.button_radius, lit, dim),
        )
    });
    let status = add(&mut tree, play, centered("", h - 30.0, 20.0, text));

    // Game over box (sized for the longest line, like the old renderer).
    let box_size = Vec2::new(w.min(420.0), 130.0);
    let game_over = add(
        &mut tree,
        ui_root,
        Panel::new()
            .with_background(Color::new(0.0, 0.0, 0.0, 0.85))
            .with_border(0.0, None)
            .with_anchors(Anchors::CENTER)
            .with_offset(-box_size * 0.5)
            .with_min_size(box_size)
            .hidden(),
    );
    add(
        &mut tree,
        game_over,
        centered("GAME OVER", 36.0, 56.0, Color::new(1.0, 0.35, 0.4, 1.0)),
    );
    let game_over_line = add(&mut tree, game_over, centered("", 87.0, 22.0, text));
    add(
        &mut tree,
        game_over,
        centered("Press R or Space to continue", 117.0, 16.0, text),
    );

    let ui = Ui {
        start,
        start_best,
        play,
        score,
        best,
        status,
        pads,
        game_over,
        game_over_line,
    };
    if let Some(simon) = node_mut::<Simon>(&mut tree, root) {
        simon.ui = Some(ui);
    }
    tree
}

fn colors_for(c: SimonColor, cfg: &GameContext) -> (Color, Color) {
    match c {
        SimonColor::Green => (cfg.color_green_lit, cfg.color_green_dim),
        SimonColor::Red => (cfg.color_red_lit, cfg.color_red_dim),
        SimonColor::Yellow => (cfg.color_yellow_lit, cfg.color_yellow_dim),
        SimonColor::Blue => (cfg.color_blue_lit, cfg.color_blue_dim),
    }
}

/// Open the window and play.
pub fn run() {
    let cfg = crate::config::load_config();
    let (w, h) = (cfg.window_w, cfg.window_h);
    let tree = build_tree(cfg, Some(crate::persistence::default()));
    let game = SceneLoop::new(
        tree,
        MacroquadRenderer::new(),
        MacroquadAudio::new(),
        Rect::new(0.0, 0.0, w, h),
    );
    run_window("Simon", w as i32, h as i32, game);
}
