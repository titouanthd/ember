// games/pong/src/scene.rs
//! Pong as a node tree, loaded from `scenes/pong.escene`:
//!
//! ```text
//! Pong            match rules; owns the match (`systems`)
//! ├─ PlayerPaddle Paddle: position, size, speed, color, ai = false
//! ├─ AiPaddle     Paddle: same, ai = true
//! ├─ Ball         Ball: size, color
//! └─ Scoreboard   `PointScored` (scene connection) / `MatchReset`
//! ```
//!
//! `PointScored` reaches the scoreboard through the scene's
//! `connections` (wired by `PackedScene::instantiate`, M4.1).
//!
//! Every class is `#[derive(Node)]` with `#[export]` fields, so the
//! editor can open and save the scene (ADR-037). Behaviour stays in
//! hand-written `impl Node` (ADR-030): in `ready`, `Pong` reads its
//! children into a `GameContext` and the `systems` entities; each frame
//! it writes the entities' positions back, and the children draw
//! themselves. The pure logic (`systems`, `components`) is unchanged.

use crate::components::{Ball, Paddle};
use crate::systems::{self, GameContext, MatchState};
use ember::prelude::*;
use ember_core::{ClassDB, GameState};
use ember_macros::Node;
use ember_scene::{ClassDbNodes, PackedScene};

/// The scene file, built into the binary.
pub const SCENE: &str = include_str!("../scenes/pong.escene");

/// Emitted by `Pong` whenever a point is scored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PointScored {
    pub player_scored: bool,
}

/// Emitted when a finished match is restarted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchReset;

/// Frame time cap; prevents tunneling after a hitch.
const MAX_FRAME_DT: f32 = 1.0 / 60.0;

/// Gap between a paddle and its side of the court.
const PADDLE_MARGIN: f32 = 30.0;

// --- Pong (root) ------------------------------------------------------

/// The root: match rules (exported) and the running match (not saved).
#[derive(Node)]
#[class_name = "Pong"]
pub struct Pong {
    pub name: String,
    /// Court size in pixels; the window opens at this size.
    #[export]
    pub court: Vec2,
    #[export]
    pub win_score: i32,
    #[export]
    pub ball_speed: f32,
    #[export]
    pub ball_max_speed: f32,
    #[export]
    pub ball_speed_increment: f32,
    #[export]
    pub ai_reaction_time: f32,
    #[export]
    pub serve_angle_max: f32,
    #[export]
    pub paddle_velocity_factor: f32,
    /// Seed for the serve angles; read in `ready`.
    pub seed: u32,
    cfg: GameContext,
    player: Paddle,
    ai: Paddle,
    pub state: MatchState,
    views: Views,
}

/// The child nodes `Pong` mirrors its entities into.
#[derive(Debug, Clone, Copy, Default)]
struct Views {
    player: Option<NodeId>,
    ai: Option<NodeId>,
    ball: Option<NodeId>,
}

impl Default for Pong {
    fn default() -> Self {
        let cfg = GameContext::default();
        let paddle = Paddle::new(
            0.0,
            0.0,
            cfg.paddle_w,
            cfg.paddle_h,
            cfg.player_speed,
            cfg.player_color,
        );
        Self {
            name: "Pong".into(),
            court: Vec2::new(cfg.screen_w, cfg.screen_h),
            win_score: cfg.win_score,
            ball_speed: cfg.base_speed,
            ball_max_speed: cfg.max_ball_speed,
            ball_speed_increment: cfg.ball_speed_increment,
            ai_reaction_time: cfg.ai_reaction_time,
            serve_angle_max: cfg.serve_angle_max,
            paddle_velocity_factor: cfg.paddle_velocity_factor,
            seed: 1,
            state: MatchState::new_with_seed(&cfg, 1),
            player: paddle.clone(),
            ai: paddle,
            cfg,
            views: Views::default(),
        }
    }
}

impl Pong {
    pub fn ball(&self) -> &Ball {
        &self.state.ball
    }

    pub fn player(&self) -> &Paddle {
        &self.player
    }

    pub fn ai(&self) -> &Paddle {
        &self.ai
    }

    /// Writes the entities' positions and sizes into the child nodes.
    fn sync_views(&self, tree: &mut NodeTree) {
        let views = [
            (self.views.player, &self.player.transform),
            (self.views.ai, &self.ai.transform),
        ];
        for (id, t) in views {
            if let Some(view) = id.and_then(|id| node_mut::<PaddleNode>(tree, id)) {
                view.position = t.position;
                view.size = t.scale;
            }
        }
        if let Some(view) = self
            .views
            .ball
            .and_then(|id| node_mut::<BallNode>(tree, id))
        {
            view.position = self.state.ball.transform.position;
        }
    }
}

impl Node for Pong {
    fn ready(&mut self, ctx: &mut NodeContext) {
        let me = ctx.current_id();
        let scene = read_scene(self, ctx.tree(), me);
        let paddle = |p: &PaddleView| {
            Paddle::new(
                p.position.x,
                p.position.y,
                p.size.x,
                p.size.y,
                p.speed,
                p.color,
            )
        };
        self.player = paddle(&scene.player);
        self.ai = paddle(&scene.ai);
        self.state = MatchState::new_with_seed(&scene.cfg, self.seed);
        self.cfg = scene.cfg;
        self.views = Views {
            player: scene.player.id,
            ai: scene.ai.id,
            ball: scene.ball,
        };
        self.sync_views(ctx.tree());
    }

    fn process(&mut self, dt: f32, ctx: &mut NodeContext) {
        let dt = dt.min(MAX_FRAME_DT);
        let input = ctx.input();
        if input.is_key_pressed(Key::Escape) {
            ctx.quit();
            return;
        }
        let (up, down) = (
            input.is_key_down(Key::W) || input.is_key_down(Key::Up),
            input.is_key_down(Key::S) || input.is_key_down(Key::Down),
        );
        let (space, restart) = (
            input.is_key_pressed(Key::Space),
            input.is_key_pressed(Key::R),
        );

        match self.state.state {
            GameState::Start if space => {
                systems::reset_match(&mut self.player, &mut self.ai, &mut self.state, &self.cfg);
                self.state.state = GameState::Playing;
            }
            GameState::Playing => {
                self.player.reset_velocity();
                if up {
                    self.player.move_up(dt);
                }
                if down {
                    self.player.move_down(dt, self.cfg.screen_h);
                }
                let before = (self.state.score_player, self.state.score_ai);
                systems::update(
                    &mut self.player,
                    &mut self.ai,
                    &mut self.state,
                    &self.cfg,
                    dt,
                );
                if self.state.score_player > before.0 {
                    ctx.emit(PointScored {
                        player_scored: true,
                    });
                }
                if self.state.score_ai > before.1 {
                    ctx.emit(PointScored {
                        player_scored: false,
                    });
                }
            }
            GameState::GameOver | GameState::Win if restart => {
                self.state.state = GameState::Start;
                systems::reset_match(&mut self.player, &mut self.ai, &mut self.state, &self.cfg);
                ctx.emit(MatchReset);
            }
            _ => {}
        }
        self.sync_views(ctx.tree());
    }

    /// The state message, drawn over the paddles and the ball.
    fn draw_after_children(&self, ctx: &mut DrawContext) {
        let msg = match self.state.state {
            GameState::Start => "Press SPACE to start",
            GameState::GameOver => "Game over - press R",
            GameState::Win => "You win! - press R",
            _ => return,
        };
        let r = ctx.render();
        let m = r.measure_text(msg, 32.0);
        r.draw_text(
            msg,
            Vec2::new((self.court.x - m.width) / 2.0, self.court.y / 2.0),
            32.0,
            Color::WHITE,
        );
    }
}

// --- Paddle, Ball -----------------------------------------------------

/// A paddle. `ai` picks which side the player controls.
#[derive(Node)]
#[class_name = "Paddle"]
pub struct PaddleNode {
    pub name: String,
    /// Top-left corner. Only `x` matters in play: `y` is centred at
    /// each serve.
    #[export]
    pub position: Vec2,
    #[export]
    pub size: Vec2,
    #[export]
    pub speed: f32,
    #[export]
    pub color: Color,
    #[export]
    pub ai: bool,
}

impl Default for PaddleNode {
    fn default() -> Self {
        let cfg = GameContext::default();
        Self {
            name: "Paddle".into(),
            position: Vec2::ZERO,
            size: Vec2::new(cfg.paddle_w, cfg.paddle_h),
            speed: cfg.player_speed,
            color: cfg.player_color,
            ai: false,
        }
    }
}

impl Node for PaddleNode {
    fn draw(&self, ctx: &mut DrawContext) {
        let r = Rect::new(self.position.x, self.position.y, self.size.x, self.size.y);
        ctx.render().draw_rect(r, self.color);
    }
}

/// The ball. It starts in the middle of the court; `position` is only
/// where it is drawn.
#[derive(Node)]
#[class_name = "Ball"]
pub struct BallNode {
    pub name: String,
    #[export]
    pub position: Vec2,
    #[export]
    pub size: f32,
    #[export]
    pub color: Color,
}

impl Default for BallNode {
    fn default() -> Self {
        let cfg = GameContext::default();
        Self {
            name: "Ball".into(),
            position: Vec2::ZERO,
            size: cfg.ball_size,
            color: cfg.ball_color,
        }
    }
}

impl Node for BallNode {
    fn draw(&self, ctx: &mut DrawContext) {
        let r = Rect::new(self.position.x, self.position.y, self.size, self.size);
        ctx.render().draw_rect(r, self.color);
    }
}

// --- Scoreboard -------------------------------------------------------

/// Shows the score; updated only through `PointScored`.
#[derive(Node)]
#[class_name = "Scoreboard"]
pub struct Scoreboard {
    pub name: String,
    /// Centre of the text's baseline.
    #[export]
    pub position: Vec2,
    #[export]
    pub font_size: f32,
    #[export]
    pub color: Color,
    pub player: u32,
    pub ai: u32,
}

impl Default for Scoreboard {
    fn default() -> Self {
        Self {
            name: "Scoreboard".into(),
            position: Vec2::new(GameContext::default().screen_w / 2.0, 50.0),
            font_size: 48.0,
            color: Color::WHITE,
            player: 0,
            ai: 0,
        }
    }
}

impl Scoreboard {
    /// Receiver of `pong.escene`'s `PointScored` connection.
    pub fn on_point_scored(&mut self, p: &PointScored) {
        if p.player_scored {
            self.player += 1;
        } else {
            self.ai += 1;
        }
    }
}

impl Node for Scoreboard {
    fn ready(&mut self, ctx: &mut NodeContext) {
        let me = ctx.current_id();
        ctx.connect::<MatchReset>(me, |ctx, _| {
            let id = ctx.current_id();
            if let Some(board) = node_mut::<Scoreboard>(ctx.tree(), id) {
                board.player = 0;
                board.ai = 0;
            }
        });
    }

    fn draw(&self, ctx: &mut DrawContext) {
        let text = format!("{}   {}", self.player, self.ai);
        let r = ctx.render();
        let m = r.measure_text(&text, self.font_size);
        r.draw_text(
            &text,
            Vec2::new(self.position.x - m.width / 2.0, self.position.y),
            self.font_size,
            self.color,
        );
    }
}

// --- Reading and writing the scene ------------------------------------

fn node_mut<T: 'static>(tree: &mut NodeTree, id: NodeId) -> Option<&mut T> {
    tree.get_mut(id)?.as_any_mut().downcast_mut::<T>()
}

fn children(tree: &NodeTree, id: NodeId) -> Vec<NodeId> {
    tree.children(id).to_vec()
}

/// A paddle as the scene describes it.
#[derive(Debug, Clone)]
struct PaddleView {
    id: Option<NodeId>,
    position: Vec2,
    size: Vec2,
    speed: f32,
    color: Color,
}

/// What `Pong` needs from its children.
struct SceneData {
    cfg: GameContext,
    player: PaddleView,
    ai: PaddleView,
    ball: Option<NodeId>,
}

/// The match config and paddles described by `pong` and its children.
/// A missing child keeps the built-in defaults.
fn read_scene(pong: &Pong, tree: &NodeTree, root: NodeId) -> SceneData {
    let d = GameContext::default();
    let mut cfg = GameContext {
        screen_w: pong.court.x,
        screen_h: pong.court.y,
        win_score: pong.win_score,
        base_speed: pong.ball_speed,
        max_ball_speed: pong.ball_max_speed,
        ball_speed_increment: pong.ball_speed_increment,
        ai_reaction_time: pong.ai_reaction_time,
        serve_angle_max: pong.serve_angle_max,
        paddle_velocity_factor: pong.paddle_velocity_factor,
        ..d.clone()
    };
    let default_paddle = |x: f32, speed: f32, color: Color| PaddleView {
        id: None,
        position: Vec2::new(x, cfg.screen_h / 2.0 - d.paddle_h / 2.0),
        size: Vec2::new(d.paddle_w, d.paddle_h),
        speed,
        color,
    };
    let mut player = default_paddle(PADDLE_MARGIN, d.player_speed, d.player_color);
    let mut ai = default_paddle(
        cfg.screen_w - PADDLE_MARGIN - d.paddle_w,
        d.ai_speed,
        d.ai_color,
    );
    let mut ball = None;
    for id in children(tree, root) {
        let Some(node) = tree.get(id) else { continue };
        if let Some(p) = node.as_any().downcast_ref::<PaddleNode>() {
            let view = PaddleView {
                id: Some(id),
                position: p.position,
                size: p.size,
                speed: p.speed,
                color: p.color,
            };
            if p.ai { ai = view } else { player = view }
        } else if let Some(b) = node.as_any().downcast_ref::<BallNode>() {
            cfg.ball_size = b.size;
            cfg.ball_color = b.color;
            ball = Some(id);
        }
    }
    cfg.paddle_w = player.size.x;
    cfg.paddle_h = player.size.y;
    cfg.player_speed = player.speed;
    cfg.player_color = player.color;
    cfg.ai_speed = ai.speed;
    cfg.ai_color = ai.color;
    SceneData {
        cfg,
        player,
        ai,
        ball,
    }
}

/// The node classes of `pong.escene`, for loading and for the editor.
pub fn classes() -> ClassDB {
    let mut db = ClassDB::new();
    db.register_node::<Pong>();
    db.register_node::<PaddleNode>();
    db.register_node::<BallNode>();
    db.register_node::<Scoreboard>();
    db.register_receiver::<Scoreboard, PointScored>(
        "PointScored",
        "on_point_scored",
        Scoreboard::on_point_scored,
    );
    db
}

/// `scenes/pong.escene`, parsed.
pub fn packed_scene() -> PackedScene {
    PackedScene::from_ron(SCENE).expect("scenes/pong.escene is a valid scene")
}

/// The tree exactly as `pong.escene` describes it.
pub fn load_tree(seed: u32) -> NodeTree {
    tree_from(&packed_scene(), seed).expect("pong.escene instantiates")
}

/// The tree a Pong scene describes (for example one saved by the editor).
pub fn tree_from(scene: &PackedScene, seed: u32) -> Result<NodeTree, ember_scene::SceneError> {
    let mut tree = NodeTree::new();
    let root = scene.instantiate(&classes(), &mut tree)?;
    tree.set_root(root);
    if let Some(pong) = node_mut::<Pong>(&mut tree, root) {
        pong.seed = seed;
    }
    Ok(tree)
}

/// The config a tree describes, as `Pong` reads it in `ready`.
pub fn scene_config(tree: &NodeTree) -> GameContext {
    let root = tree.root().expect("root");
    let pong = tree
        .get(root)
        .and_then(|n| n.as_any().downcast_ref::<Pong>())
        .expect("root is Pong");
    read_scene(pong, tree, root).cfg
}

/// Writes `cfg` into the tree's nodes. With `base`, only the fields
/// where `cfg` differs from `base` are written, so a paddle's own size
/// or colour survives an `.env` that does not name it.
pub fn apply_config(tree: &mut NodeTree, cfg: &GameContext, base: Option<&GameContext>) {
    let changed = |f: fn(&GameContext) -> Vec<f32>| base.is_none_or(|b| f(b) != f(cfg));
    let court = changed(|c| vec![c.screen_w, c.screen_h]);
    let paddle_size = changed(|c| vec![c.paddle_w, c.paddle_h]);
    let ball_size = changed(|c| vec![c.ball_size]);
    let root = tree.root().expect("root");

    if let Some(pong) = node_mut::<Pong>(tree, root) {
        pong.court = Vec2::new(cfg.screen_w, cfg.screen_h);
        pong.win_score = cfg.win_score;
        pong.ball_speed = cfg.base_speed;
        pong.ball_max_speed = cfg.max_ball_speed;
        pong.ball_speed_increment = cfg.ball_speed_increment;
        pong.ai_reaction_time = cfg.ai_reaction_time;
        pong.serve_angle_max = cfg.serve_angle_max;
        pong.paddle_velocity_factor = cfg.paddle_velocity_factor;
    }
    for id in children(tree, root) {
        if let Some(p) = node_mut::<PaddleNode>(tree, id) {
            let (speed, color) = if p.ai {
                (cfg.ai_speed, cfg.ai_color)
            } else {
                (cfg.player_speed, cfg.player_color)
            };
            if paddle_size {
                p.size = Vec2::new(cfg.paddle_w, cfg.paddle_h);
            }
            if court || paddle_size {
                let x = if p.ai {
                    cfg.screen_w - PADDLE_MARGIN - p.size.x
                } else {
                    PADDLE_MARGIN
                };
                p.position = Vec2::new(x, cfg.screen_h / 2.0 - p.size.y / 2.0);
            }
            if base.is_none_or(|b| speed != if p.ai { b.ai_speed } else { b.player_speed }) {
                p.speed = speed;
            }
            let base_color = base.map(|b| if p.ai { b.ai_color } else { b.player_color });
            if base_color.is_none_or(|b| b != color) {
                p.color = color;
            }
        } else if let Some(b) = node_mut::<BallNode>(tree, id) {
            if ball_size {
                b.size = cfg.ball_size;
            }
            if court || ball_size {
                b.position = Vec2::new(cfg.screen_w, cfg.screen_h) / 2.0 - b.size / 2.0;
            }
            if base.is_none_or(|base| base.ball_color != cfg.ball_color) {
                b.color = cfg.ball_color;
            }
        } else if let Some(s) = node_mut::<Scoreboard>(tree, id)
            && court
        {
            s.position.x = cfg.screen_w / 2.0;
        }
    }
}

/// The scene's structure with every value taken from `cfg` (the tests'
/// hermetic path: same tree as before the scene file existed).
pub fn build_tree(cfg: GameContext, seed: u32) -> NodeTree {
    let mut tree = load_tree(seed);
    apply_config(&mut tree, &cfg, None);
    tree
}

/// Opens the window and plays `pong.escene`, with `.env` overrides.
pub fn run() {
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u32)
        .unwrap_or(1);
    let mut tree = load_tree(seed);
    let base = scene_config(&tree);
    let cfg = crate::config::load_config_over(&base);
    apply_config(&mut tree, &cfg, Some(&base));
    let (w, h) = (cfg.screen_w, cfg.screen_h);
    let game = SceneLoop::new(
        tree,
        MacroquadRenderer::new(),
        MacroquadAudio::new(),
        Rect::new(0.0, 0.0, w, h),
    );
    run_window("Pong - Powered by Ember", w as i32, h as i32, game);
}
