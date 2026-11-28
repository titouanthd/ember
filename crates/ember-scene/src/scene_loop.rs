//! `SceneLoop`: drives a `NodeTree` as a `MainLoop`.

use crate::{DrawContext, NodeTree, SceneError, SceneLibrary, Servers};
use ember_assets::AssetServer;
use ember_core::math::Rect;
use ember_core::{ClassDB, Color, MainLoop};
use ember_input::{ActionMap, ActionResolver, Input, InputSink};
use ember_servers::{AudioServer, Camera2d, LinearPhysics, RenderingServer};
use glam::Vec2;
use std::sync::Arc;

/// A fixed canvas the game draws on, scaled to fit the window with its
/// aspect kept; the rest is `bars` (ADR-041).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VirtualResolution {
    pub size: Vec2,
    pub bars: Color,
}

/// Owns a tree plus the backends it runs against. The runner updates
/// `input_mut()` once per frame before calling `process`.
pub struct SceneLoop<R: RenderingServer, A: AudioServer> {
    pub tree: NodeTree,
    rendering: R,
    audio: A,
    physics: LinearPhysics,
    input: Input,
    viewport: Rect,
    quit: bool,
    virtual_resolution: Option<VirtualResolution>,
    /// Last size from `MainLoop::resize`; the viewport size until then.
    window: Vec2,
    /// Set by `with_viewport_following_window`.
    viewport_follows_window: bool,
    /// Fills `Input::actions` in `set_input` (ADR-052).
    actions: Option<ActionResolver>,
    /// What scene changes build with (ADR-054).
    classes: Option<Arc<ClassDB>>,
    assets: Option<Arc<AssetServer>>,
    /// The last scene change that failed, until taken.
    scene_error: Option<SceneError>,
}

impl<R: RenderingServer, A: AudioServer> SceneLoop<R, A> {
    pub fn new(tree: NodeTree, rendering: R, audio: A, viewport: Rect) -> Self {
        Self {
            tree,
            rendering,
            audio,
            physics: LinearPhysics::new(),
            input: Input::default(),
            viewport,
            quit: false,
            virtual_resolution: None,
            window: Vec2::new(viewport.w, viewport.h),
            viewport_follows_window: false,
            actions: None,
            classes: None,
            assets: None,
            scene_error: None,
        }
    }

    /// The `ClassDB` scene changes instance with (ADR-054). Needed for
    /// `ctx.change_scene(packed_scene)` and for a path.
    pub fn with_class_db(mut self, classes: impl Into<Arc<ClassDB>>) -> Self {
        self.classes = Some(classes.into());
        self
    }

    /// The `AssetServer` scene changes load with: `ctx.change_scene(
    /// "levels/2.escene")`, and `instance:` entries in a changed-to scene.
    pub fn with_assets(mut self, assets: impl Into<Arc<AssetServer>>) -> Self {
        self.assets = Some(assets.into());
        self
    }

    /// The last scene change that failed, if any; taking it clears it.
    /// A failed change keeps the current scene.
    pub fn take_scene_error(&mut self) -> Option<SceneError> {
        self.scene_error.take()
    }

    /// Apply a change the tree asked for (`NodeTree::apply_scene_change`).
    /// `init` and `process` call it at their end.
    fn apply_scene_change(&mut self) {
        if !self.tree.scene_change_pending() {
            return;
        }
        let mut servers = Servers {
            rendering: &mut self.rendering,
            audio: &mut self.audio,
            physics: &mut self.physics,
            input: &self.input,
        };
        let library = SceneLibrary::new(self.classes.as_deref(), self.assets.as_deref());
        if let Some(Err(e)) = self.tree.apply_scene_change(&mut servers, &library) {
            eprintln!("warning: {e}; keeping the current scene");
            self.scene_error = Some(e);
        }
    }

    /// Resolve `map`'s named actions into every `Input` this loop is
    /// given, so nodes read `ctx.input().action_pressed("jump")`
    /// (ADR-052).
    pub fn with_action_map(mut self, map: ActionMap) -> Self {
        self.set_action_map(Some(map));
        self
    }

    /// Swap the action map at runtime (rebinding, spec 15 §11 Q3), or
    /// drop it with `None`. Held actions start over: one held across
    /// the swap reads as pressed again on the next frame.
    pub fn set_action_map(&mut self, map: Option<ActionMap>) {
        self.actions = map.map(ActionResolver::new);
    }

    pub fn action_map(&self) -> Option<&ActionMap> {
        self.actions.as_ref().map(ActionResolver::map)
    }

    /// Make the viewport track the window: each `resize` sets it to
    /// `(0, 0, width, height)`, so a game that lays itself out on the
    /// window size reads that size from `RenderingServer::viewport` (from
    /// the next frame on). Ignored while a virtual resolution is set,
    /// whose canvas stays the viewport.
    pub fn with_viewport_following_window(mut self) -> Self {
        self.viewport_follows_window = true;
        self
    }

    /// Draw on a fixed `width` × `height` canvas, letterboxed into the
    /// window with black bars (ADR-041). Nodes see that canvas as their
    /// viewport, and mouse and touch positions arrive in its coordinates.
    pub fn with_virtual_resolution(mut self, width: f32, height: f32) -> Self {
        self.set_virtual_resolution(Some(VirtualResolution {
            size: Vec2::new(width, height),
            bars: Color::BLACK,
        }));
        self
    }

    pub fn set_virtual_resolution(&mut self, virtual_resolution: Option<VirtualResolution>) {
        if let Some(v) = virtual_resolution {
            self.viewport = Rect::new(0.0, 0.0, v.size.x, v.size.y);
        }
        self.virtual_resolution = virtual_resolution;
    }

    pub fn virtual_resolution(&self) -> Option<VirtualResolution> {
        self.virtual_resolution
    }

    pub fn window_size(&self) -> Vec2 {
        self.window
    }

    /// The camera that maps the canvas onto the window: the letterbox
    /// with a virtual resolution, else the identity.
    pub fn screen_camera(&self) -> Camera2d {
        match self.virtual_resolution {
            Some(v) => Camera2d::letterbox(self.window, v.size),
            None => Camera2d::IDENTITY,
        }
    }

    pub fn input_mut(&mut self) -> &mut Input {
        &mut self.input
    }

    pub fn set_viewport(&mut self, viewport: Rect) {
        self.viewport = viewport;
    }

    pub fn rendering(&self) -> &R {
        &self.rendering
    }

    pub fn rendering_mut(&mut self) -> &mut R {
        &mut self.rendering
    }

    pub fn physics_mut(&mut self) -> &mut LinearPhysics {
        &mut self.physics
    }

    pub fn audio(&self) -> &A {
        &self.audio
    }

    /// Make the next `process` return `false`.
    pub fn request_quit(&mut self) {
        self.quit = true;
    }
}

impl<R: RenderingServer, A: AudioServer> MainLoop for SceneLoop<R, A> {
    fn init(&mut self) -> bool {
        let mut servers = Servers {
            rendering: &mut self.rendering,
            audio: &mut self.audio,
            physics: &mut self.physics,
            input: &self.input,
        };
        self.tree.flush(&mut servers);
        self.apply_scene_change();
        true
    }

    fn process(&mut self, dt: f32) -> bool {
        let mut servers = Servers {
            rendering: &mut self.rendering,
            audio: &mut self.audio,
            physics: &mut self.physics,
            input: &self.input,
        };
        self.tree.process(&mut servers, dt);
        // The frame boundary: the new scene is readied before `draw`.
        self.apply_scene_change();
        !(self.quit || self.tree.quit_requested())
    }

    fn physics_process(&mut self, fixed_dt: f32) {
        let mut servers = Servers {
            rendering: &mut self.rendering,
            audio: &mut self.audio,
            physics: &mut self.physics,
            input: &self.input,
        };
        self.tree.physics_process(&mut servers, fixed_dt);
    }

    fn draw(&mut self) {
        self.rendering.begin_frame(self.viewport);
        if let Some(v) = self.virtual_resolution {
            // Bars first; the canvas is drawn over them, clipped to it.
            self.rendering.set_clear_color(v.bars);
            self.rendering.set_camera(self.screen_camera());
            self.rendering
                .push_clip_rect(Camera2d::letterbox_rect(self.window, v.size));
        }
        self.tree.draw(&mut DrawContext::new(&mut self.rendering));
        if self.virtual_resolution.is_some() {
            self.rendering.pop_clip_rect();
            self.rendering.set_camera(Camera2d::IDENTITY);
        }
        self.rendering.end_frame();
        self.audio.flush();
    }

    fn finalize(&mut self) {
        self.tree.clear();
    }

    fn resize(&mut self, width: f32, height: f32) {
        self.window = Vec2::new(width, height);
        if self.viewport_follows_window && self.virtual_resolution.is_none() {
            self.viewport = Rect::new(0.0, 0.0, width, height);
        }
    }
}

impl<R: RenderingServer, A: AudioServer> InputSink for SceneLoop<R, A> {
    /// With a virtual resolution, mouse and touch positions are mapped
    /// from window pixels to canvas coordinates. Then, with an action
    /// map, `input.actions` is filled (swipe distances are therefore in
    /// canvas units).
    fn set_input(&mut self, mut input: Input) {
        if self.virtual_resolution.is_some() {
            let camera = self.screen_camera();
            input.mouse_pos = camera.screen_to_world(input.mouse_pos);
            for touch in &mut input.touch_points {
                touch.position = camera.screen_to_world(touch.position);
            }
        }
        if let Some(actions) = &mut self.actions {
            actions.resolve(&mut input);
        }
        self.input = input;
    }
}
