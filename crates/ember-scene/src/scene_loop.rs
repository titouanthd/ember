//! `SceneLoop`: drives a `NodeTree` as a `MainLoop`.

use crate::{DrawContext, NodeTree, Servers};
use ember_core::MainLoop;
use ember_core::math::Rect;
use ember_input::Input;
use ember_servers::{AudioServer, RenderingServer};

/// Owns a tree plus the backends it runs against. The runner updates
/// `input_mut()` once per frame before calling `process`.
pub struct SceneLoop<R: RenderingServer, A: AudioServer> {
    pub tree: NodeTree,
    rendering: R,
    audio: A,
    input: Input,
    viewport: Rect,
    quit: bool,
}

impl<R: RenderingServer, A: AudioServer> SceneLoop<R, A> {
    pub fn new(tree: NodeTree, rendering: R, audio: A, viewport: Rect) -> Self {
        Self {
            tree,
            rendering,
            audio,
            input: Input::default(),
            viewport,
            quit: false,
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
            input: &self.input,
        };
        self.tree.flush(&mut servers);
        true
    }

    fn process(&mut self, dt: f32) -> bool {
        let mut servers = Servers {
            rendering: &mut self.rendering,
            audio: &mut self.audio,
            input: &self.input,
        };
        self.tree.process(&mut servers, dt);
        !self.quit
    }

    fn physics_process(&mut self, fixed_dt: f32) {
        let mut servers = Servers {
            rendering: &mut self.rendering,
            audio: &mut self.audio,
            input: &self.input,
        };
        self.tree.physics_process(&mut servers, fixed_dt);
    }

    fn draw(&mut self) {
        self.rendering.begin_frame(self.viewport);
        self.tree.draw(&mut DrawContext::new(&mut self.rendering));
        self.rendering.end_frame();
        self.audio.flush();
    }

    fn finalize(&mut self) {
        self.tree.clear();
    }
}
