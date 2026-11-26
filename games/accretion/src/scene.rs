//! Accretion as a node tree: one root node, `Accretion`, mouse plus a
//! few keys. It runs the app (`App`: title, game, prestige, endgame,
//! endings), saves through a `Saver`, and draws through the
//! `RenderingServer`.
//!
//! The screens lay themselves out on the window size, so the loop runs
//! with `with_viewport_following_window` (ADR-041 amendment) and the node
//! hands the size, the pointer and the clock to `frame` each frame.

use crate::app::App;
use crate::frame;
use crate::systems::Game;
use crate::{fonts, save};
use ember::prelude::*;

/// The window the game opens with.
pub const WINDOW_W: f32 = 1280.0;
pub const WINDOW_H: f32 = 800.0;

/// Where progress goes: `save::save` when playing, something else in
/// tests. Returns whether it worked.
pub type Saver = fn(&Game) -> bool;

pub struct Accretion {
    name: String,
    pub app: App,
    saver: Saver,
}

impl Accretion {
    pub fn new(game: Game, saver: Saver) -> Self {
        Self {
            name: "Accretion".into(),
            app: App::new(game),
            saver,
        }
    }
}

impl Object for Accretion {
    fn class_name() -> &'static str {
        "Accretion"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl NodeName for Accretion {
    fn name(&self) -> &str {
        &self.name
    }
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

impl Node for Accretion {
    fn ready(&mut self, ctx: &mut NodeContext) {
        fonts::load_default_font(ctx.render());
    }

    fn process(&mut self, dt: f32, ctx: &mut NodeContext) {
        // The last frame's viewport is the window (zero before the first
        // frame, when the opening size stands).
        let viewport = ctx.render().viewport();
        let size = if viewport.w > 0.0 && viewport.h > 0.0 {
            viewport.size()
        } else {
            Vec2::new(WINDOW_W, WINDOW_H)
        };
        let input = ctx.input().clone();
        frame::begin(size, input.mouse_pos, dt);

        if self.app.update(&input, dt) {
            (self.saver)(&self.app.game);
        }
    }

    fn draw(&self, ctx: &mut DrawContext) {
        self.app.draw(ctx.render());
    }
}

/// The whole game: a single `Accretion` root.
pub fn build_tree(game: Game, saver: Saver) -> NodeTree {
    NodeTree::with_root(Accretion::new(game, saver))
}

/// Opens the window (high DPI, as before; the screens follow its size)
/// and plays the saved run, or a new one.
pub fn run() {
    let game = SceneLoop::new(
        build_tree(save::load().unwrap_or_default(), save::save),
        MacroquadRenderer::new(),
        MacroquadAudio::new(),
        Rect::new(0.0, 0.0, WINDOW_W, WINDOW_H),
    )
    .with_viewport_following_window();
    run_window_with(
        WindowConfig::new("Accretion", WINDOW_W as i32, WINDOW_H as i32).high_dpi(true),
        game,
    );
}
