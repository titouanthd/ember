//! `scenes/pong.escene`: what it loads into, and that saving it is
//! lossless.

use ember::prelude::*;
use ember_core::{Color, Variant};
use ember_scene::{NodeOrigins, PackedScene};
use ember_servers::{MockAudio, MockRenderer};
use pong::GameContext;
use pong::scene::{self, BallNode, PaddleNode, Pong, SCENE};

/// The values `.env` held before the scene file existed.
fn shipped_config() -> GameContext {
    GameContext {
        base_speed: 600.0,
        ball_color: Color::new(0.7, 0.0, 1.0, 1.0),
        ..GameContext::default()
    }
}

fn outline(tree: &NodeTree) -> Vec<(String, &'static str)> {
    let root = tree.root().unwrap();
    let classes = scene::classes();
    std::iter::once(root)
        .chain(tree.children(root).iter().copied())
        .map(|id| {
            let n = tree.get(id).unwrap();
            let class = classes
                .class_of(std::any::Any::type_id(n.as_any()))
                .unwrap();
            (n.name().to_string(), class)
        })
        .collect()
}

fn save(tree: &NodeTree) -> String {
    scene::packed_scene()
        .resave(
            tree,
            tree.root().unwrap(),
            &scene::classes(),
            &NodeOrigins::new(),
        )
        .unwrap()
        .to_ron()
}

fn started(tree: NodeTree) -> SceneLoop<MockRenderer, MockAudio> {
    let mut l = SceneLoop::new(
        tree,
        MockRenderer::new(),
        MockAudio::new(),
        Rect::new(0.0, 0.0, 800.0, 600.0),
    );
    assert!(l.init());
    l
}

fn pong(l: &SceneLoop<MockRenderer, MockAudio>) -> &Pong {
    let root = l.tree.root().unwrap();
    l.tree
        .get(root)
        .unwrap()
        .as_any()
        .downcast_ref::<Pong>()
        .unwrap()
}

#[test]
fn the_scene_has_the_old_hand_built_shape() {
    let tree = scene::load_tree(1);
    let names: Vec<_> = outline(&tree)
        .into_iter()
        .map(|(n, c)| format!("{n}:{c}"))
        .collect();
    assert_eq!(
        names,
        [
            "Pong:Pong",
            "PlayerPaddle:Paddle",
            "AiPaddle:Paddle",
            "Ball:Ball",
            "Scoreboard:Scoreboard"
        ]
    );
    assert_eq!(scene::scene_config(&tree), shipped_config());
}

#[test]
fn the_scene_plays_like_the_hand_built_tree() {
    let from_file = started(scene::load_tree(42));
    let from_code = started(scene::build_tree(shipped_config(), 42));
    let (a, b) = (pong(&from_file), pong(&from_code));
    assert_eq!(a.player().transform.position, Vec2::new(30.0, 260.0));
    assert_eq!(a.ai().transform.position, Vec2::new(755.0, 260.0));
    assert_eq!(a.player().transform, b.player().transform);
    assert_eq!(a.ai().transform, b.ai().transform);
    assert_eq!(a.ball().transform, b.ball().transform);
    assert_eq!(a.ball().sprite.color, b.ball().sprite.color);
}

#[test]
fn save_load_save_is_identical_to_the_file() {
    let first = save(&scene::load_tree(1));
    assert_eq!(
        first, SCENE,
        "pong.escene is not in the writer's canonical form"
    );
    let mut tree = NodeTree::new();
    let root = PackedScene::from_ron(&first)
        .unwrap()
        .instantiate(&scene::classes(), &mut tree)
        .unwrap();
    tree.set_root(root);
    assert_eq!(save(&tree), first);
}

#[test]
fn an_edited_paddle_size_reaches_the_game() {
    let mut tree = scene::load_tree(1);
    let root = tree.root().unwrap();
    let id = tree.find(root, "PlayerPaddle").unwrap();
    let paddle = tree.get_mut(id).unwrap();
    assert!(paddle.set_property("size", &Variant::Vec2(Vec2::new(16.0, 160.0))));

    let mut l = started(tree);
    assert_eq!(pong(&l).player().transform.scale, Vec2::new(16.0, 160.0));
    assert_eq!(pong(&l).ai().transform.scale, Vec2::new(15.0, 80.0));

    // A serve centres each paddle on its own height.
    let mut input = Input::default();
    input.keys_pressed.push(Key::Space);
    l.set_input(input);
    l.process(1.0 / 60.0);
    assert_eq!(pong(&l).player().transform.position.y, 300.0 - 80.0);
    assert_eq!(pong(&l).ai().transform.position.y, 300.0 - 40.0);
}

#[test]
fn the_children_draw_where_the_match_is() {
    let mut l = started(scene::load_tree(7));
    let mut input = Input::default();
    input.keys_pressed.push(Key::Space);
    l.set_input(input);
    l.process(1.0 / 60.0);
    l.set_input(Input::default());
    l.process(1.0 / 60.0);
    let ball_at = pong(&l).ball().transform.position;
    let root = l.tree.root().unwrap();
    let id = l.tree.find(root, "Ball").unwrap();
    let view = l
        .tree
        .get(id)
        .unwrap()
        .as_any()
        .downcast_ref::<BallNode>()
        .unwrap();
    assert_eq!(view.position, ball_at);
    let id = l.tree.find(root, "AiPaddle").unwrap();
    let view = l
        .tree
        .get(id)
        .unwrap()
        .as_any()
        .downcast_ref::<PaddleNode>()
        .unwrap();
    assert_eq!(view.position, pong(&l).ai().transform.position);
}

#[test]
fn an_override_only_writes_what_it_changes() {
    let mut tree = scene::load_tree(1);
    let root = tree.root().unwrap();
    let ai = tree.find(root, "AiPaddle").unwrap();
    let ai_node = tree.get_mut(ai).unwrap();
    assert!(ai_node.set_property("size", &Variant::Vec2(Vec2::new(15.0, 120.0))));

    // As if `.env` set only BALL_SPEED: the AI paddle keeps its own size.
    let base = scene::scene_config(&tree);
    let cfg = GameContext {
        base_speed: 900.0,
        ..base.clone()
    };
    scene::apply_config(&mut tree, &cfg, Some(&base));
    let l = started(tree);
    assert_eq!(pong(&l).ai().transform.scale, Vec2::new(15.0, 120.0));
    assert_eq!(scene::scene_config(&l.tree).base_speed, 900.0);

    // PADDLE_HEIGHT applies to both paddles.
    let mut tree = scene::load_tree(1);
    let base = scene::scene_config(&tree);
    let cfg = GameContext {
        paddle_h: 100.0,
        ..base.clone()
    };
    scene::apply_config(&mut tree, &cfg, Some(&base));
    let l = started(tree);
    assert_eq!(pong(&l).player().transform.scale.y, 100.0);
    assert_eq!(pong(&l).ai().transform.scale.y, 100.0);
}
