//! Bore Inc. as a node tree: one root node, `BoreInc`. It builds the
//! app in `ready` (the sprite atlas and background need the renderer),
//! runs `AppState` each frame, applies the music's queued plays, saves
//! progress, and draws through the `RenderingServer`.
//!
//! Build order in `ready` (was `main`'s preload):
//!   1. Font.
//!   2. Music (clips loaded; native starts at once, web after a click).
//!   3. Sprite atlas (sheet decoded, cells and truck baked, uploaded).
//!   4. Background textures.
//!   5. AppState.
//!
//! Screens follow the window size: the loop runs with
//! `with_viewport_following_window` (ADR-041 amendment) and the node
//! copies the viewport into `GameContext` each frame.

use crate::atlas::AtlasBuilder;
use crate::audio::{self, DEFAULT_TRACKS};
use crate::background::BackgroundData;
use crate::config::GameContext;
use crate::fonts;
use crate::progress::{self, Progress};
use crate::sprites::archie as archie_sprites;
use crate::sprites::truck::TruckSprites;
use crate::systems::{AppAction, AppScreen, AppState};
use ember::prelude::*;

/// The window the game opens with.
pub const WINDOW_W: f32 = 1280.0;
pub const WINDOW_H: f32 = 720.0;

/// Seed for the surface zone background. All levels in this zone
/// share the same backdrop.
const ZONE_SEED: u32 = 0x50_11;

/// Seconds between autosaves when progress is dirty.
const AUTOSAVE_INTERVAL: f32 = 2.0;

/// Frame time cap, as before.
const MAX_FRAME_DT: f32 = 0.05;

/// Where progress goes: `progress::save_progress` when playing,
/// something else in tests.
pub type Saver = fn(&Progress) -> Result<(), String>;

pub struct BoreInc {
    name: String,
    pub ctx: GameContext,
    /// Handed to `AppState` in `ready`.
    progress: Option<Progress>,
    /// Built in `ready`.
    pub state: Option<AppState>,
    saver: Saver,
}

impl BoreInc {
    pub fn new(ctx: GameContext, progress: Progress, saver: Saver) -> Self {
        Self {
            name: "BoreInc".into(),
            ctx,
            progress: Some(progress),
            state: None,
            saver,
        }
    }

    fn autosave(&mut self, dt: f32) {
        let Some(state) = self.state.as_mut() else {
            return;
        };
        state.time_since_save += dt;
        if state.progress_dirty && state.time_since_save >= AUTOSAVE_INTERVAL {
            match (self.saver)(&state.progress) {
                Ok(()) => {
                    state.progress_dirty = false;
                    state.time_since_save = 0.0;
                }
                Err(e) => {
                    eprintln!("[bore-inc] autosave failed: {e}");
                    state.time_since_save = 0.0;
                }
            }
        }
    }
}

impl Object for BoreInc {
    fn class_name() -> &'static str {
        "BoreInc"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl NodeName for BoreInc {
    fn name(&self) -> &str {
        &self.name
    }
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

impl Node for BoreInc {
    fn ready(&mut self, ctx: &mut NodeContext) {
        // 1. Font.
        fonts::load_default_font(ctx.render());

        // 2. Music.
        let mut music = audio::new_player();
        music.backend_mut().load(DEFAULT_TRACKS, ctx.audio());
        if !cfg!(target_arch = "wasm32") {
            music.start();
        }

        // 3. Sprite atlas.
        let mut builder = AtlasBuilder::new();
        let sheet = archie_sprites::load_sheet()
            .unwrap_or_else(|e| panic!("[bore-inc] failed to decode archie.png: {e}"));
        let archie_rects = archie_sprites::bake_from_image(&mut builder, &sheet);
        let truck_rects = crate::sprites::truck::bake_into(&mut builder);
        let atlas = builder
            .bake()
            .upload(ctx.render())
            .unwrap_or_else(|e| panic!("[bore-inc] failed to upload the atlas: {e}"));

        // 4. Background textures.
        let background = BackgroundData::from_seed(ZONE_SEED).upload(ctx.render());

        // 5. State.
        let truck_sprites = TruckSprites {
            r: truck_rects.0,
            l: truck_rects.1,
        };
        let progress = self.progress.take().unwrap_or_default();
        let mut state = AppState::new(
            progress,
            music,
            atlas,
            archie_rects,
            truck_sprites,
            background,
        );
        // The browser blocks audio until a click.
        if cfg!(target_arch = "wasm32") {
            state.screen = AppScreen::WebStart;
        }
        self.state = Some(state);
    }

    fn process(&mut self, dt: f32, ctx: &mut NodeContext) {
        let dt = dt.min(MAX_FRAME_DT);
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
        let Some(state) = self.state.as_mut() else {
            return;
        };
        let action = state.tick(&input, &self.ctx, dt);
        state.audio.backend_mut().apply(ctx.audio());
        if action == AppAction::Quit {
            ctx.quit();
        }
        self.autosave(dt);
    }

    fn draw(&self, ctx: &mut DrawContext) {
        let gfx = ctx.render();
        gfx.set_clear_color(self.ctx.colors.bg_bottom);
        if let Some(state) = &self.state {
            state.draw(gfx, &self.ctx);
        }
    }
}

/// The whole game: a single `BoreInc` root.
pub fn build_tree(ctx: GameContext, progress: Progress, saver: Saver) -> NodeTree {
    NodeTree::with_root(BoreInc::new(ctx, progress, saver))
}

/// Opens the window (resizable; the screens follow its size) and plays.
pub fn run() {
    let game = SceneLoop::new(
        build_tree(
            GameContext::from_env(),
            progress::load_progress(),
            progress::save_progress,
        ),
        MacroquadRenderer::new(),
        MacroquadAudio::new(),
        Rect::new(0.0, 0.0, WINDOW_W, WINDOW_H),
    )
    .with_viewport_following_window();
    run_window("Bore Inc.", WINDOW_W as i32, WINDOW_H as i32, game);
}
