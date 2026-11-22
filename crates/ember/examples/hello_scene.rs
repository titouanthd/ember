//! Phase 1 deliverable: a node tree running on the macroquad backends.
//! Arrow keys move the square; Escape quits.

use ember::prelude::*;

#[derive(Node)]
#[class_name = "Root"]
struct Root {
    name: String,
}

impl Root {
    fn new() -> Self {
        Self {
            name: "Root".into(),
        }
    }
}

// No behaviour: the derive provides `Object` + `NodeName`; the empty
// `impl Node` keeps every lifecycle default.
impl Node for Root {}

// Hand-written on purpose (no derive): shows the full manual form.
struct Player {
    name: String,
    pos: Vec2,
}

impl Player {
    const SIZE: f32 = 48.0;
    const SPEED: f32 = 240.0;
}

impl Object for Player {
    fn class_name() -> &'static str {
        "Player"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl NodeName for Player {
    fn name(&self) -> &str {
        &self.name
    }
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

impl Node for Player {
    fn process(&mut self, dt: f32, ctx: &mut NodeContext) {
        let input = ctx.input();
        let mut dir = Vec2::ZERO;
        for key in &input.keys_down {
            match key {
                Key::Left => dir.x -= 1.0,
                Key::Right => dir.x += 1.0,
                Key::Up => dir.y -= 1.0,
                Key::Down => dir.y += 1.0,
                _ => {}
            }
        }
        if input.keys_pressed.contains(&Key::Escape) {
            ctx.quit();
        }
        self.pos += dir.normalize_or_zero() * Self::SPEED * dt;
    }

    fn draw(&self, ctx: &mut DrawContext) {
        let r = Rect::new(self.pos.x, self.pos.y, Self::SIZE, Self::SIZE);
        ctx.render().draw_rect(r, Color::rgb(1.0, 0.45, 0.1));
    }
}

fn main() {
    let mut tree = NodeTree::with_root(Root::new());
    let root = tree.root().expect("root");
    let player = tree.insert(Player {
        name: "Player".into(),
        pos: Vec2::new(100.0, 100.0),
    });
    tree.attach(player, root).expect("attach");

    let viewport = Rect::new(0.0, 0.0, 800.0, 600.0);
    let scene = SceneLoop::new(
        tree,
        MacroquadRenderer::new(),
        MacroquadAudio::new(),
        viewport,
    );
    run_window("hello_scene", 800, 600, scene);
}
