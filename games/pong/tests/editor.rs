//! The editor proof, headless: `pong.escene` opens in
//! `ember_editor::Document`, an edit saves and reloads, and the game
//! plays the saved scene (ADR-039).

use ember::prelude::*;
use ember_editor::{Document, EditorConfig, run::outline_text};
use ember_scene::PackedScene;
use ember_servers::{MockAudio, MockRenderer};
use pong::scene::{self, Pong, SCENE};
use std::path::PathBuf;

/// A scratch copy of `pong.escene`, one per test.
fn scratch(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pong-editor-{}-{test}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("pong.escene");
    std::fs::write(&path, SCENE).unwrap();
    path
}

#[test]
fn the_outline_is_the_pong_tree() {
    let doc = Document::open(scratch("outline"), scene::classes()).unwrap();
    let rows: Vec<_> = doc
        .outline()
        .iter()
        .map(|r| format!("{}{}", "  ".repeat(r.depth), r.name))
        .collect();
    assert_eq!(
        rows,
        [
            "Pong",
            "  PlayerPaddle",
            "  AiPaddle",
            "  Ball",
            "  Scoreboard"
        ]
    );
    let text = outline_text(&doc);
    assert!(text.contains("PlayerPaddle (Paddle)"), "{text}");
    assert!(text.contains("· size = 15.0, 80.0"), "{text}");
}

#[test]
fn opening_and_saving_changes_no_byte() {
    let path = scratch("lossless");
    let mut doc = Document::open(&path, scene::classes()).unwrap();
    doc.save().unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), SCENE);
}

#[test]
fn an_edit_saves_reloads_and_plays() {
    let path = scratch("edit");
    let mut doc = Document::open(&path, scene::classes()).unwrap();
    let paddle = doc.find("PlayerPaddle").unwrap();
    for key in ["size", "speed", "color"] {
        assert!(doc.property_text(paddle, key).is_ok(), "{key} is editable");
    }
    doc.set_property_text(paddle, "size", "16, 160").unwrap();
    assert!(doc.is_dirty());
    doc.save().unwrap();

    let reopened = Document::open(&path, scene::classes()).unwrap();
    let paddle = reopened.find("PlayerPaddle").unwrap();
    assert_eq!(
        reopened.property_text(paddle, "size").unwrap(),
        "16.0, 160.0"
    );

    let saved = PackedScene::from_file(&path).unwrap();
    let mut game = SceneLoop::new(
        scene::tree_from(&saved, 1).unwrap(),
        MockRenderer::new(),
        MockAudio::new(),
        Rect::new(0.0, 0.0, 800.0, 600.0),
    );
    assert!(game.init());
    let root = game.tree.root().unwrap();
    let pong = game
        .tree
        .get(root)
        .unwrap()
        .as_any()
        .downcast_ref::<Pong>()
        .unwrap();
    assert_eq!(pong.player().transform.scale, Vec2::new(16.0, 160.0));
}

#[test]
fn the_edit_example_parses_its_arguments() {
    let c = EditorConfig::parse(scene::classes(), ["scenes/pong.escene", "--print"]).unwrap();
    assert_eq!(c.scene, PathBuf::from("scenes/pong.escene"));
    assert!(c.print);
    assert!(EditorConfig::parse(scene::classes(), Vec::<String>::new()).is_err());
    let c = c.with_viewport(800.0, 600.0);
    assert_eq!(c.viewport, Some(Vec2::new(800.0, 600.0)));
}
