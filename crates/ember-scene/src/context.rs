//! `NodeContext` and `DrawContext`, plus the `Servers` bundle the tree
//! hands to every callback.

use crate::{Node, NodeId, NodeTree};
use ember_core::math::Rect;
use ember_input::Input;
use ember_servers::{AudioServer, RenderingServer};
use glam::Vec2;

/// The backends a frame runs against. Borrowed from the `MainLoop`.
pub struct Servers<'a> {
    pub rendering: &'a mut dyn RenderingServer,
    pub audio: &'a mut dyn AudioServer,
    pub input: &'a Input,
}

/// Everything a node may touch during `ready`, `process` and
/// `physics_process`. A reborrow of the loop's state; never stored.
///
/// While a node's callback runs, the node itself is checked out of the
/// tree: `ctx.tree().get(ctx.current_id())` is `None`.
pub struct NodeContext<'a> {
    tree: &'a mut NodeTree,
    rendering: &'a mut dyn RenderingServer,
    audio: &'a mut dyn AudioServer,
    input: &'a Input,
    current: NodeId,
    delta: f32,
}

impl<'a> NodeContext<'a> {
    pub(crate) fn new(
        tree: &'a mut NodeTree,
        servers: &'a mut Servers<'_>,
        current: NodeId,
        delta: f32,
    ) -> Self {
        Self {
            tree,
            rendering: &mut *servers.rendering,
            audio: &mut *servers.audio,
            input: servers.input,
            current,
            delta,
        }
    }

    // --- Identity ---

    pub fn current_id(&self) -> NodeId {
        self.current
    }

    pub fn parent_id(&self) -> Option<NodeId> {
        self.tree.parent(self.current)
    }

    /// `dt` for `process`, the fixed step for `physics_process`, `0.0`
    /// for `ready`.
    pub fn delta(&self) -> f32 {
        self.delta
    }

    // --- Servers ---

    pub fn render(&mut self) -> &mut dyn RenderingServer {
        &mut *self.rendering
    }

    pub fn audio(&mut self) -> &mut dyn AudioServer {
        &mut *self.audio
    }

    pub fn input(&self) -> &Input {
        self.input
    }

    // --- Tree ---

    pub fn tree(&mut self) -> &mut NodeTree {
        self.tree
    }

    /// Resolve `path` relative to the current node (see `NodeTree::find`).
    pub fn find(&self, path: &str) -> Option<NodeId> {
        self.tree.find(self.current, path)
    }

    // --- Deferred structure changes ---

    /// Queue `node` for insertion. The id is valid immediately, but the
    /// node is only visible to the tree after the current pass flushes.
    pub fn spawn(&mut self, node: impl Node) -> NodeId {
        self.tree.spawn(Box::new(node), None)
    }

    /// Like `spawn`, attached under `parent` at flush time.
    pub fn spawn_child(&mut self, parent: NodeId, node: impl Node) -> NodeId {
        self.tree.spawn(Box::new(node), Some(parent))
    }

    /// Queue `id` and its descendants for removal at flush time.
    pub fn despawn(&mut self, id: NodeId) {
        self.tree.despawn(id);
    }
}

/// Read-only rendering access for `Node::draw`. There is no tree here by
/// construction.
pub struct DrawContext<'a> {
    rendering: &'a mut dyn RenderingServer,
}

impl<'a> DrawContext<'a> {
    pub fn new(rendering: &'a mut dyn RenderingServer) -> Self {
        Self { rendering }
    }

    pub fn render(&mut self) -> &mut dyn RenderingServer {
        &mut *self.rendering
    }

    pub fn viewport(&self) -> Rect {
        self.rendering.viewport()
    }

    pub fn camera_offset(&self) -> Vec2 {
        self.rendering.camera_offset()
    }

    pub fn current_z(&self) -> f32 {
        self.rendering.current_z()
    }

    pub fn world_to_screen(&self, world: Vec2) -> Vec2 {
        world - self.camera_offset()
    }

    pub fn screen_to_world(&self, screen: Vec2) -> Vec2 {
        screen + self.camera_offset()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ember_servers::MockRenderer;

    #[test]
    fn world_screen_round_trip_through_camera() {
        let mut r = MockRenderer::new();
        r.set_camera(Vec2::new(10.0, 20.0));
        let ctx = DrawContext::new(&mut r);
        let w = Vec2::new(15.0, 25.0);
        assert_eq!(ctx.world_to_screen(w), Vec2::new(5.0, 5.0));
        assert_eq!(ctx.screen_to_world(ctx.world_to_screen(w)), w);
    }
}
