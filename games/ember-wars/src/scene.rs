//! Ember Wars as a node tree, on a high-DPI window whose viewport
//! follows its size:
//!
//! ```text
//! EmberWars        the campaign: menu, upgrade modal, progress, saving
//! └── World        the level being played, when there is one (world.rs)
//! ```
//!
//! The root spawns a `World` when a level is picked and listens to its
//! signals: `LevelFinished` banks and saves, `RestartLevel` swaps in a
//! fresh world, `LeaveLevel` despawns it.

use std::path::Path;

use crate::app::{App, MenuOutcome, Saver, seed_from_time};
use crate::config::GameContext;
use crate::menu::AppScreen;
use crate::progress::{self, Progress};
use crate::systems::Game;
use crate::world::{LeaveLevel, LevelFinished, RestartLevel, World};
use crate::{fonts, frame};
use ember::prelude::*;

/// The window the game opens with: 1600×900 (16:9) avoids the cramped
/// layouts of macroquad's default 800×600.
pub const WINDOW_W: f32 = 1600.0;
pub const WINDOW_H: f32 = 900.0;

/// Frame time cap, as before (the world caps its battle `dt`).
pub const MAX_FRAME_DT: f32 = 0.05;

pub struct EmberWars {
    name: String,
    pub ctx: GameContext,
    pub app: App,
    /// The level being played.
    pub world: Option<NodeId>,
}

impl EmberWars {
    /// A game over `progress`, saving through `save`, seeding levels
    /// with `seed`.
    pub fn new(ctx: GameContext, progress: Progress, save: Saver, seed: fn() -> u32) -> Self {
        Self {
            name: "EmberWars".into(),
            ctx,
            app: App::new(progress, save, seed),
            world: None,
        }
    }

    fn play(&mut self, game: Game, ctx: &mut NodeContext) {
        let me = ctx.current_id();
        self.world = Some(ctx.spawn_child(me, World::new(&self.ctx, game)));
    }
}

/// The root, from a signal handler (which runs with the root as its
/// target).
fn root_of<'a>(ctx: &'a mut NodeContext) -> Option<&'a mut EmberWars> {
    let me = ctx.current_id();
    ctx.tree()
        .get_mut(me)?
        .as_any_mut()
        .downcast_mut::<EmberWars>()
}

impl Object for EmberWars {
    fn class_name() -> &'static str {
        "EmberWars"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl NodeName for EmberWars {
    fn name(&self) -> &str {
        &self.name
    }
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

impl Node for EmberWars {
    fn ready(&mut self, ctx: &mut NodeContext) {
        fonts::load_default_font(ctx.render());
        let me = ctx.current_id();
        ctx.connect::<LevelFinished>(me, |ctx, ev| {
            if let Some(root) = root_of(ctx) {
                root.app.finish_level(ev);
            }
        });
        ctx.connect::<LeaveLevel>(me, |ctx, _| {
            let Some(root) = root_of(ctx) else { return };
            let world = root.world.take();
            root.app.screen = AppScreen::Menu;
            if let Some(world) = world {
                ctx.despawn(world);
            }
        });
        ctx.connect::<RestartLevel>(me, |ctx, ev| {
            let Some(root) = root_of(ctx) else { return };
            let Ok(game) = root.app.new_game(&root.ctx, &ev.level_id) else {
                return;
            };
            let world = World::new(&root.ctx, game);
            let old = root.world.take();
            if let Some(old) = old {
                ctx.despawn(old);
            }
            let me = ctx.current_id();
            let new = ctx.spawn_child(me, world);
            if let Some(root) = root_of(ctx) {
                root.world = Some(new);
            }
        });
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
        self.ctx.viewport_w = size.x;
        self.ctx.viewport_h = size.y;
        let input = ctx.input().clone();
        frame::begin(size, input.mouse_pos, dt);

        match self.app.update(&input, &self.ctx) {
            MenuOutcome::Stay => {}
            MenuOutcome::Quit => ctx.quit(),
            MenuOutcome::Play(game) => self.play(*game, ctx),
        }
    }

    fn draw(&self, ctx: &mut DrawContext) {
        self.app.draw(ctx.render(), &self.ctx);
    }
}

/// The whole game: a single `EmberWars` root.
pub fn build_tree(
    ctx: GameContext,
    progress: Progress,
    save: Saver,
    seed: fn() -> u32,
) -> NodeTree {
    NodeTree::with_root(EmberWars::new(ctx, progress, save, seed))
}

/// Opens the window (high DPI, resizable, as before) and plays.
pub fn run() {
    let (progress, store) = progress::load(Path::new(env!("CARGO_MANIFEST_DIR")));
    let save: Saver = Box::new(move |p: &Progress| {
        store.save(p);
    });
    let game = SceneLoop::new(
        build_tree(GameContext::from_env(), progress, save, seed_from_time),
        MacroquadRenderer::new(),
        MacroquadAudio::new(),
        Rect::new(0.0, 0.0, WINDOW_W, WINDOW_H),
    )
    .with_viewport_following_window();
    run_window_with(
        WindowConfig::new("ember-wars", WINDOW_W as i32, WINDOW_H as i32).high_dpi(true),
        game,
    );
}
