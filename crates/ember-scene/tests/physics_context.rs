use ember_core::MainLoop;
use ember_core::math::{Aabb, Rect};
use ember_scene::prelude::*;
use ember_servers::{CollisionMask, MockAudio, MockRenderer, PhysicsShape};
use glam::Vec2;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

struct Sensor {
    hits: Arc<AtomicUsize>,
}

impl Object for Sensor {
    fn class_name() -> &'static str {
        "Sensor"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl Node for Sensor {
    fn name(&self) -> &str {
        "Sensor"
    }
    fn set_name(&mut self, _: String) {}
    fn z(&self) -> f32 {
        0.0
    }
    fn visible(&self) -> bool {
        true
    }
    fn ready(&mut self, ctx: &mut NodeContext) {
        let wall = Aabb::from_center_half(Vec2::new(10.0, 0.0), Vec2::new(1.0, 5.0));
        ctx.physics()
            .register(PhysicsShape::Aabb(wall), CollisionMask::ALL);
    }
    fn physics_process(&mut self, _dt: f32, ctx: &mut NodeContext) {
        let hit = ctx
            .physics()
            .raycast(Vec2::ZERO, Vec2::new(20.0, 0.0), CollisionMask::ALL);
        if hit.is_some() {
            self.hits.fetch_add(1, Ordering::SeqCst);
        }
    }
}

#[test]
fn nodes_reach_the_physics_server_through_the_context() {
    let hits = Arc::new(AtomicUsize::new(0));
    let mut tree = NodeTree::new();
    let root = tree.insert(Sensor { hits: hits.clone() });
    tree.set_root(root);
    let mut l = SceneLoop::new(
        tree,
        MockRenderer::new(),
        MockAudio::new(),
        Rect::new(0.0, 0.0, 800.0, 600.0),
    );
    assert!(l.init());
    l.physics_process(1.0 / 60.0);
    assert_eq!(hits.load(Ordering::SeqCst), 1);
    assert_eq!(l.physics_mut().len(), 1);
}
