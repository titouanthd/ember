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

// --- The editor window on Pong (M1.4 preview) -------------------------

mod window {
    use super::*;
    use ember_editor::app::{EditorApp, build_app_tree, preview_camera, preview_rect};
    use ember_servers::{Camera2d, RecordedCall};

    type Editor = SceneLoop<MockRenderer, MockAudio>;

    fn editor(test: &str) -> Editor {
        let doc = Document::open(scratch(test), scene::classes()).unwrap();
        let app = EditorApp::new(doc).with_viewport(Some(Vec2::new(800.0, 600.0)));
        let mut e = SceneLoop::new(
            build_app_tree(app),
            MockRenderer::new(),
            MockAudio::new(),
            Rect::new(0.0, 0.0, 1100.0, 700.0),
        );
        assert!(e.init());
        e
    }

    fn app_mut(e: &mut Editor) -> &mut EditorApp {
        let root = e.tree.root().unwrap();
        e.tree
            .get_mut(root)
            .unwrap()
            .as_any_mut()
            .downcast_mut::<EditorApp>()
            .unwrap()
    }

    /// The rects drawn while the preview camera was set.
    fn preview_rects(e: &mut Editor) -> Vec<Rect> {
        e.rendering_mut().clear_calls();
        e.draw();
        let mut inside = false;
        let mut out = Vec::new();
        for call in e.rendering().calls() {
            match call {
                RecordedCall::SetCamera(c) => inside = c.zoom != 1.0,
                RecordedCall::DrawRect { rect, .. } if inside => out.push(*rect),
                _ => {}
            }
        }
        out
    }

    #[test]
    fn the_court_is_scaled_into_the_preview_panel() {
        let camera = preview_camera(Some(Vec2::new(800.0, 600.0)));
        let area = preview_rect();
        let court = camera.rect_to_screen(Rect::new(0.0, 0.0, 800.0, 600.0));
        assert!(camera.zoom < 1.0);
        assert!(court.x >= area.x && court.x + court.w <= area.x + area.w);
        assert!(court.y >= area.y && court.y + court.h <= area.y + area.h);
        // Without a viewport the POC placement is unchanged.
        assert_eq!(
            preview_camera(None),
            Camera2d::placing_origin_at(Vec2::new(area.x + 20.0, area.y + 20.0), 1.0)
        );
    }

    #[test]
    fn the_preview_ran_ready_and_draws_the_paddles() {
        let mut e = editor("ready");
        let app = app_mut(&mut e);
        let root = app.doc.root();
        let pong = app
            .doc
            .tree()
            .get(root)
            .unwrap()
            .as_any()
            .downcast_ref::<Pong>()
            .unwrap();
        assert_eq!(pong.player().transform.scale, Vec2::new(15.0, 80.0));
        assert!(!app.doc.is_dirty(), "ready does not dirty the document");

        let rects = preview_rects(&mut e);
        assert!(rects.contains(&Rect::new(0.0, 0.0, 800.0, 600.0)), "court");
        assert!(
            rects.contains(&Rect::new(30.0, 260.0, 15.0, 80.0)),
            "player paddle"
        );
        assert!(
            rects.contains(&Rect::new(755.0, 260.0, 15.0, 80.0)),
            "AI paddle"
        );
    }

    #[test]
    fn a_size_edit_shows_in_the_next_preview_frame() {
        let mut e = editor("edit-preview");
        let doc = &mut app_mut(&mut e).doc;
        let paddle = doc.find("PlayerPaddle").unwrap();
        doc.set_property_text(paddle, "size", "16, 160").unwrap();
        let rects = preview_rects(&mut e);
        assert!(
            rects.contains(&Rect::new(30.0, 260.0, 16.0, 160.0)),
            "{rects:?}"
        );
    }
}
