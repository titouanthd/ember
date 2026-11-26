// games/pong/src/scene.rs
//! Pong as a node tree: `Pong` (root, owns the match) and `Scoreboard`
//! (child, listens to `PointScored`). The pure logic stays in
//! `systems`; nodes only bridge input, signals and drawing.

use crate::components::{Ball, Paddle};
use crate::systems::{self, GameContext, MatchState};
use ember::prelude::*;
use ember_core::GameState;

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

fn rect(t: &ember_stdlib::Transform) -> Rect {
    Rect::new(t.position.x, t.position.y, t.scale.x, t.scale.y)
}

pub struct Pong {
    name: String,
    cfg: GameContext,
    player: Paddle,
    ai: Paddle,
    pub state: MatchState,
}

impl Pong {
    pub fn new(cfg: GameContext, seed: u32) -> Self {
        let y = cfg.screen_h / 2.0 - cfg.paddle_h / 2.0;
        let player = Paddle::new(
            30.0,
            y,
            cfg.paddle_w,
            cfg.paddle_h,
            cfg.player_speed,
            cfg.player_color,
        );
        let ai = Paddle::new(
            cfg.screen_w - 30.0 - cfg.paddle_w,
            y,
            cfg.paddle_w,
            cfg.paddle_h,
            cfg.ai_speed,
            cfg.ai_color,
        );
        let state = MatchState::new_with_seed(&cfg, seed);
        Self {
            name: "Pong".into(),
            cfg,
            player,
            ai,
            state,
        }
    }

    pub fn ball(&self) -> &Ball {
        &self.state.ball
    }
}

impl Object for Pong {
    fn class_name() -> &'static str {
        "Pong"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl NodeName for Pong {
    fn name(&self) -> &str {
        &self.name
    }
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

impl Node for Pong {
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
    }

    fn draw(&self, ctx: &mut DrawContext) {
        let (w, h) = (self.cfg.screen_w, self.cfg.screen_h);
        let r = ctx.render();
        r.draw_rect(rect(&self.player.transform), self.player.sprite.color);
        r.draw_rect(rect(&self.ai.transform), self.ai.sprite.color);
        r.draw_rect(
            rect(&self.state.ball.transform),
            self.state.ball.sprite.color,
        );
        let msg = match self.state.state {
            GameState::Start => Some("Press SPACE to start"),
            GameState::GameOver => Some("Game over - press R"),
            GameState::Win => Some("You win! - press R"),
            _ => None,
        };
        if let Some(msg) = msg {
            let m = r.measure_text(msg, 32.0);
            r.draw_text(
                msg,
                Vec2::new((w - m.width) / 2.0, h / 2.0),
                32.0,
                Color::WHITE,
            );
        }
    }
}

/// Shows the score; updated only through `PointScored`.
pub struct Scoreboard {
    name: String,
    width: f32,
    pub player: u32,
    pub ai: u32,
}

impl Scoreboard {
    pub fn new(width: f32) -> Self {
        Self {
            name: "Scoreboard".into(),
            width,
            player: 0,
            ai: 0,
        }
    }
}

impl Object for Scoreboard {
    fn class_name() -> &'static str {
        "Scoreboard"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl NodeName for Scoreboard {
    fn name(&self) -> &str {
        &self.name
    }
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

impl Node for Scoreboard {
    fn ready(&mut self, ctx: &mut NodeContext) {
        let me = ctx.current_id();
        ctx.connect::<PointScored>(me, |ctx, p| {
            let id = ctx.current_id();
            if let Some(board) = ctx
                .tree()
                .get_mut(id)
                .and_then(|n| n.as_any_mut().downcast_mut::<Scoreboard>())
            {
                if p.player_scored {
                    board.player += 1;
                } else {
                    board.ai += 1;
                }
            }
        });
        ctx.connect::<MatchReset>(me, |ctx, _| {
            let id = ctx.current_id();
            if let Some(board) = ctx
                .tree()
                .get_mut(id)
                .and_then(|n| n.as_any_mut().downcast_mut::<Scoreboard>())
            {
                board.player = 0;
                board.ai = 0;
            }
        });
    }

    fn draw(&self, ctx: &mut DrawContext) {
        let text = format!("{}   {}", self.player, self.ai);
        let r = ctx.render();
        let m = r.measure_text(&text, 48.0);
        r.draw_text(
            &text,
            Vec2::new((self.width - m.width) / 2.0, 50.0),
            48.0,
            Color::WHITE,
        );
    }
}

/// The whole game as a tree: `Pong` with a `Scoreboard` child.
pub fn build_tree(cfg: GameContext, seed: u32) -> NodeTree {
    let width = cfg.screen_w;
    let mut tree = NodeTree::with_root(Pong::new(cfg, seed));
    let root = tree.root().expect("root");
    let board = tree.insert(Scoreboard::new(width));
    tree.attach(board, root).expect("attach");
    tree
}

/// Opens the window and plays Pong.
pub fn run() {
    let cfg = crate::config::load_config();
    let (w, h) = (cfg.screen_w, cfg.screen_h);
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u32)
        .unwrap_or(1);
    let game = SceneLoop::new(
        build_tree(cfg, seed),
        MacroquadRenderer::new(),
        MacroquadAudio::new(),
        Rect::new(0.0, 0.0, w, h),
    );
    run_window("Pong - Powered by Ember", w as i32, h as i32, game);
}
