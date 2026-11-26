// games/taiji/src/scene.rs
//! Tàijí as a node tree: one root node, `Taiji`, keyboard only. It
//! runs the menus (`App`) and the match (`systems`) and draws both on
//! an 800 × 600 canvas that `SceneLoop` letterboxes into the window
//! (ADR-041; it replaces the old render target).

use crate::app::{App, AppState, centered};
use crate::config::{GameContext, WHITE};
use crate::systems::{self, MatchState, ScoreEvent};
use crate::{Paddle, QiBall};
use ember::prelude::*;

/// Frame time cap, as before.
const MAX_FRAME_DT: f32 = 1.0 / 60.0;

/// Below this time scale the "Souffle du Temps" (Yin) is shown.
const YIN_SHOWN_BELOW: f32 = 0.95;

pub struct Taiji {
    name: String,
    pub ctx: GameContext,
    pub app: App,
    pub match_state: MatchState,
}

impl Default for Taiji {
    fn default() -> Self {
        let ctx = GameContext::default();
        Self {
            name: "Taiji".into(),
            match_state: MatchState::new(&ctx),
            app: App::new(),
            ctx,
        }
    }
}

impl Object for Taiji {
    fn class_name() -> &'static str {
        "Taiji"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl NodeName for Taiji {
    fn name(&self) -> &str {
        &self.name
    }
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

impl Taiji {
    fn yin(&self) -> bool {
        self.match_state.time_scale < YIN_SHOWN_BELOW
    }

    fn on_score(&mut self, event: ScoreEvent) {
        let ctx = &self.ctx;
        let (score, direction) = match event {
            ScoreEvent::PlayerScored => (&mut self.app.score_player, -1.0),
            ScoreEvent::AiScored => (&mut self.app.score_ai, 1.0),
        };
        *score += 1;
        if *score >= ctx.win_score {
            self.app.state = AppState::GameOver;
        } else {
            self.match_state.reset_round(ctx, direction);
        }
    }

    fn draw_paddle(&self, r: &mut dyn RenderingServer, paddle: &Paddle) {
        let p = paddle.transform.position;
        let rect = Rect::new(p.x, p.y, self.ctx.paddle_w, self.ctx.paddle_h);
        r.draw_rect(rect, paddle.color);
    }

    fn draw_ball(&self, r: &mut dyn RenderingServer, ball: &QiBall) {
        // The ball turns blue while time is slowed.
        let color = if self.yin() {
            Color::new(0.0, 0.5, 1.0, 1.0) // Bleu Yin
        } else {
            self.ctx.color_ball
        };
        r.draw_circle(ball.transform.position, self.ctx.ball_radius, color);
    }
}

impl Node for Taiji {
    fn process(&mut self, dt: f32, ctx: &mut NodeContext) {
        let dt = dt.min(MAX_FRAME_DT);
        let input = ctx.input();

        let prev_state = self.app.state;
        let running = self.app.update(input);

        // A new game resets the match.
        if matches!(prev_state, AppState::MainMenu | AppState::GameOver)
            && self.app.state == AppState::Playing
        {
            self.match_state = MatchState::new(&self.ctx);
        }

        if self.app.state == AppState::Playing
            && let Some(event) = systems::update(&mut self.match_state, &self.ctx, dt, input)
        {
            self.on_score(event);
        }

        if !running {
            ctx.quit();
        }
    }

    fn draw(&self, ctx: &mut DrawContext) {
        let (w, h) = (self.ctx.screen_w, self.ctx.screen_h);
        let r = ctx.render();
        // The canvas background (was `clear_background(WHITE)` on the
        // render target; clearing now would paint over the bars).
        r.draw_rect(Rect::new(0.0, 0.0, w, h), WHITE);

        // The Souffle du Temps tints the court blue.
        if self.yin() {
            let intensity = (1.0 - self.match_state.time_scale) / 0.75;
            r.draw_rect(
                Rect::new(0.0, 0.0, w, h),
                Color::new(0.4, 0.6, 0.8, intensity * 0.2),
            );
            // A subtle "YIN".
            centered(
                r,
                "YIN",
                w / 2.0,
                50.0,
                20.0,
                Color::new(0.0, 0.0, 0.0, intensity * 0.5),
            );
        }

        if matches!(self.app.state, AppState::Playing | AppState::Paused) {
            self.draw_paddle(r, &self.match_state.player);
            self.draw_paddle(r, &self.match_state.ai);
            self.draw_ball(r, &self.match_state.ball);
        }

        self.app.draw(&self.ctx, r);
    }
}

/// The whole game: a single `Taiji` root.
pub fn build_tree() -> NodeTree {
    NodeTree::with_root(Taiji::default())
}

/// Opens the window (resizable, letterboxed) and plays.
pub fn run() {
    let ctx = GameContext::default();
    let (w, h) = (ctx.screen_w, ctx.screen_h);
    let game = SceneLoop::new(
        build_tree(),
        MacroquadRenderer::new(),
        MacroquadAudio::new(),
        Rect::new(0.0, 0.0, w, h),
    )
    .with_virtual_resolution(w, h);
    run_window("TÀIJÍ", w as i32, h as i32, game);
}
