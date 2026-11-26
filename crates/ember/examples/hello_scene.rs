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

/// The demo scene, generic over the backends so a test can run it on mocks.
fn build_scene<R: RenderingServer, A: AudioServer>(rendering: R, audio: A) -> SceneLoop<R, A> {
    let mut tree = NodeTree::with_root(Root::new());
    let root = tree.root().expect("root");
    let player = tree.insert(Player {
        name: "Player".into(),
        pos: Vec2::new(100.0, 100.0),
    });
    tree.attach(player, root).expect("attach");

    let viewport = Rect::new(0.0, 0.0, 800.0, 600.0);
    SceneLoop::new(tree, rendering, audio, viewport)
}

fn main() {
    let scene = build_scene(MacroquadRenderer::new(), MacroquadAudio::new());
    run_window("hello_scene", 800, 600, scene);
}

// Phase 1 exit criterion, checked headless: the same scene, on mock
// backends, draws the player and moves it with the arrow keys
// (`cargo test -p ember --example hello_scene`; part of the workspace run).
#[cfg(test)]
mod tests {
    use super::*;
    use ember_servers::{MockAudio, MockRenderer, RecordedCall};

    const ORANGE: Color = Color::rgb(1.0, 0.45, 0.1);

    /// The player squares drawn in the last frame.
    fn drawn(scene: &SceneLoop<MockRenderer, MockAudio>) -> Vec<Rect> {
        scene
            .rendering()
            .calls()
            .iter()
            .filter_map(|c| match c {
                RecordedCall::DrawRect { rect, color, .. } if *color == ORANGE => Some(*rect),
                _ => None,
            })
            .collect()
    }

    fn frame(scene: &mut SceneLoop<MockRenderer, MockAudio>, dt: f32) -> bool {
        scene.rendering_mut().clear_calls();
        let running = scene.process(dt);
        scene.draw();
        scene.rendering().assert_frame_closed();
        running
    }

    #[test]
    fn renders_the_player_square() {
        let mut scene = build_scene(MockRenderer::new(), MockAudio::new());
        assert!(scene.init());
        assert!(frame(&mut scene, 1.0 / 60.0));
        assert_eq!(drawn(&scene), vec![Rect::new(100.0, 100.0, 48.0, 48.0)]);
    }

    #[test]
    fn arrow_keys_move_the_player() {
        let mut scene = build_scene(MockRenderer::new(), MockAudio::new());
        assert!(scene.init());
        scene.input_mut().keys_down = vec![Key::Right, Key::Down];
        for _ in 0..30 {
            assert!(frame(&mut scene, 1.0 / 60.0));
        }
        let r = drawn(&scene)[0];
        // Half a second at 240 px/s along the normalised diagonal.
        let step = 120.0 / 2f32.sqrt();
        assert!((r.x - (100.0 + step)).abs() < 1e-3, "x = {}", r.x);
        assert!((r.y - (100.0 + step)).abs() < 1e-3, "y = {}", r.y);
    }

    #[test]
    fn escape_quits() {
        let mut scene = build_scene(MockRenderer::new(), MockAudio::new());
        assert!(scene.init());
        scene.input_mut().keys_pressed = vec![Key::Escape];
        assert!(!frame(&mut scene, 1.0 / 60.0));
    }
}
