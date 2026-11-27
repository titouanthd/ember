//! The editable classes the CLI knows (ADR-037: everything the editor
//! shows comes from `#[export]`).
//!
//! Positions are absolute (the engine has no transform hierarchy yet).

use ember_core::{ClassDB, Color};
use ember_macros::Node;
use ember_scene::{ClassDbNodes, DrawContext};
use glam::Vec2;

/// A character root. Draws a small marker at `position`.
#[derive(Node, Default)]
#[class_name = "Player"]
pub struct Player {
    pub name: String,
    #[export]
    pub position: Vec2,
    #[export]
    pub speed: f32,
    #[export]
    pub max_hp: f32,
    /// Hits taken, counted by `on_hit` (not saved).
    pub hits: u32,
}

/// The `hit` signal of the fixture scene's `Hitbox → Player` connection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hit;

impl Player {
    /// Receiver of the scene-declared `hit` connection.
    pub fn on_hit(&mut self, _: &Hit) {
        self.hits += 1;
    }
}

impl ember_scene::Node for Player {
    fn draw(&self, ctx: &mut DrawContext) {
        ctx.render()
            .draw_circle_outline(self.position, 6.0, 2.0, Color::WHITE);
    }
}

/// A coloured rectangle standing in for a texture. `texture` is the
/// asset path; the POC preview does not load it.
#[derive(Node)]
#[class_name = "Sprite"]
pub struct Sprite {
    pub name: String,
    #[export]
    pub texture: String,
    #[export]
    pub position: Vec2,
    #[export]
    pub size: Vec2,
    #[export]
    pub color: Color,
    #[export]
    pub visible: bool,
    #[export]
    pub z: f32,
}

impl Default for Sprite {
    fn default() -> Self {
        Self {
            name: "Sprite".into(),
            texture: String::new(),
            position: Vec2::ZERO,
            size: Vec2::new(32.0, 32.0),
            color: Color::new(0.4, 0.7, 1.0, 1.0),
            visible: true,
            z: 0.0,
        }
    }
}

impl ember_scene::Node for Sprite {
    fn z(&self) -> f32 {
        self.z
    }
    fn visible(&self) -> bool {
        self.visible
    }
    fn draw(&self, ctx: &mut DrawContext) {
        let r =
            ember_core::math::Rect::new(self.position.x, self.position.y, self.size.x, self.size.y);
        let render = ctx.render();
        render.draw_rect(r, self.color);
        render.draw_rect_outline(r, 1.0, Color::new(1.0, 1.0, 1.0, 0.4));
    }
}

/// Register the built-in classes.
pub fn register(classes: &mut ClassDB) {
    classes.register_node::<Player>();
    classes.register_node::<Sprite>();
    classes.register_receiver::<Player, Hit>("hit", "on_hit", Player::on_hit);
}

/// A `ClassDB` with the built-in classes.
pub fn classes() -> ClassDB {
    let mut db = ClassDB::new();
    register(&mut db);
    db
}
