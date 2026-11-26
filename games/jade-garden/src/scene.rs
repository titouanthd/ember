//! Jade Garden as a node tree: one root node, `JadeGarden`, mouse plus
//! a few keys. It runs the app (`AppState`: menus, levels, cinematics),
//! ticks the music playlist on the engine's audio, and draws through the
//! `RenderingServer`.
//!
//! Every screen lays itself out on the window size, so the loop runs
//! with `with_viewport_following_window` (ADR-041 amendment) and the node
//! copies the viewport into `GameContext` each frame.

use crate::app::AppState;
use crate::audio::{MusicPlayer, default_playlist};
use crate::clock;
use crate::config::GameContext;
use crate::fonts::{self, PoemFont};
use crate::progress::{self, Progress};
use ember::prelude::*;

/// The window the game opens with.
pub const WINDOW_W: f32 = 1400.0;
pub const WINDOW_H: f32 = 900.0;

/// Frame time cap for the game logic, as before.
const MAX_FRAME_DT: f32 = 0.05;

pub struct JadeGarden {
    name: String,
    pub ctx: GameContext,
    pub app: AppState,
}

impl JadeGarden {
    /// A game over `progress`, configured by `ctx`.
    pub fn new(ctx: GameContext, progress: Progress) -> Self {
        let mut app = AppState::new(
            PoemFont::hermetic(),
            MusicPlayer::new(default_playlist()),
            progress,
        );
        // The browser blocks audio until a click.
        if cfg!(target_arch = "wasm32") {
            app.show_web_start();
        }
        Self {
            name: "JadeGarden".into(),
            ctx,
            app,
        }
    }
}

impl Object for JadeGarden {
    fn class_name() -> &'static str {
        "JadeGarden"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl NodeName for JadeGarden {
    fn name(&self) -> &str {
        &self.name
    }
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

impl Node for JadeGarden {
    fn ready(&mut self, ctx: &mut NodeContext) {
        fonts::load_default_font(ctx.render());
        self.app.set_poem_font(PoemFont::load(ctx.render()));
        self.app.music().load(ctx.audio());
        // Native plays at once; the web waits for the first click.
        if !cfg!(target_arch = "wasm32") {
            self.app.music().start();
        }
    }

    fn process(&mut self, dt: f32, ctx: &mut NodeContext) {
        clock::advance(dt);
        // The last frame's viewport is the window (zero before the first
        // frame, when the opening size stands).
        let viewport = ctx.render().viewport();
        let (w, h) = if viewport.w > 0.0 && viewport.h > 0.0 {
            (viewport.w, viewport.h)
        } else {
            (WINDOW_W, WINDOW_H)
        };
        self.ctx.viewport_w = w;
        self.ctx.viewport_h = h;

        let input = ctx.input().clone();
        let running = self.app.update(&self.ctx, &input, dt.min(MAX_FRAME_DT));
        // Real time for the playlist's track timer.
        self.app.music().tick(dt, ctx.audio());
        if !running {
            ctx.quit();
        }
    }

    fn draw(&self, ctx: &mut DrawContext) {
        self.app.draw(ctx.render(), &self.ctx);
    }
}

/// The whole game: a single `JadeGarden` root.
pub fn build_tree(ctx: GameContext, progress: Progress) -> NodeTree {
    NodeTree::with_root(JadeGarden::new(ctx, progress))
}

/// Opens the window (resizable; every screen follows its size) and plays.
pub fn run() {
    let game = SceneLoop::new(
        build_tree(GameContext::from_env(), progress::load_progress()),
        MacroquadRenderer::new(),
        MacroquadAudio::new(),
        Rect::new(0.0, 0.0, WINDOW_W, WINDOW_H),
    )
    .with_viewport_following_window();
    let config = WindowConfig::new("jade-garden", WINDOW_W as i32, WINDOW_H as i32);
    #[cfg(not(target_arch = "wasm32"))]
    let config = match window_icon() {
        Some(icon) => config.with_icon(icon),
        None => config,
    };
    run_window_with(config, game);
}

/// The window icon, resized from `assets/icon-256.png` (native only; the
/// browser tab uses the page's own icon).
#[cfg(not(target_arch = "wasm32"))]
fn window_icon() -> Option<WindowIcon> {
    let bytes = include_bytes!("../assets/icon-256.png");
    let img = image::load_from_memory_with_format(bytes, image::ImageFormat::Png).ok()?;
    let at = |side: u32| {
        let resized =
            image::imageops::resize(&img, side, side, image::imageops::FilterType::Lanczos3);
        ember_core::Image::from_rgba8(side, side, resized.into_raw())
    };
    Some(WindowIcon {
        small: at(16)?,
        medium: at(32)?,
        big: at(64)?,
    })
}
