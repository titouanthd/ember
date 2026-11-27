//! M4.1 lossless documents: a `Document` saves back what it opened,
//! ext resources, connections and unknown properties included.

use ember_editor::Document;
use ember_editor::nodes::{self, Hit, Player};
use std::path::{Path, PathBuf};

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

const FIXTURE: &str = "assets/test_fixtures/player.escene";

/// Every `.escene` in the repo and the test that round-trips it. A new
/// scene needs a byte-for-byte test (with its game's `ClassDB`) and a
/// line here.
const COVERED: &[(&str, &str)] = &[
    (FIXTURE, "crates/ember-editor/tests/lossless.rs"),
    (
        "games/pong/scenes/pong.escene",
        "games/pong/tests/editor.rs",
    ),
];

fn scenes_under(dir: &Path, root: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let path = e.path();
        let name = e.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if !matches!(
                name.as_ref(),
                "target" | ".git" | ".claude" | "node_modules"
            ) {
                scenes_under(&path, root, out);
            }
        } else if name.ends_with(".escene") {
            let rel = path.strip_prefix(root).unwrap();
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}

#[test]
fn every_escene_in_the_repo_has_a_round_trip_test() {
    let root = repo().canonicalize().unwrap();
    let mut found = Vec::new();
    scenes_under(&root, &root, &mut found);
    found.sort();
    let mut covered: Vec<_> = COVERED.iter().map(|(s, _)| s.to_string()).collect();
    covered.sort();
    assert_eq!(found, covered, "add a round-trip test for each new scene");
}

fn scratch(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ember-lossless-{}-{test}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("player.escene");
    std::fs::copy(repo().join(FIXTURE), &path).unwrap();
    path
}

#[test]
fn opening_and_saving_the_fixture_changes_no_byte() {
    let path = scratch("bytes");
    let before = std::fs::read(&path).unwrap();
    let mut doc = Document::open(&path, nodes::classes()).unwrap();
    doc.save().unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn an_edit_keeps_ext_resources_connections_and_unknown_properties() {
    let path = scratch("edit");
    let mut doc = Document::open(&path, nodes::classes()).unwrap();
    let sprite = doc.find("Sprite").unwrap();
    doc.set_property_text(sprite, "size", "64, 64").unwrap();
    doc.save().unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    for kept in [
        r#"uid: "uid://sprite_player_a3b1c""#,
        r#"path: "sprites/player.png""#,
        r#"method: "on_hit""#,
        r#""mystery": Bool(true)"#,
        r#""speed": Int(5)"#,
        "Vec2(64.0, 64.0)",
    ] {
        assert!(text.contains(kept), "`{kept}` missing from\n{text}");
    }
    // Defaults the file left out stay out.
    assert!(!text.contains("texture"), "{text}");
}

#[test]
fn an_edited_value_is_written_as_the_node_holds_it() {
    let path = scratch("retype");
    let mut doc = Document::open(&path, nodes::classes()).unwrap();
    doc.set_property_text(doc.root(), "speed", "7").unwrap();
    doc.save().unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains(r#""speed": Float(7.0)"#), "{text}");
    assert!(!text.contains("Int(5)"), "{text}");
}

#[test]
fn the_fixture_connection_is_wired_in_the_document() {
    let doc = Document::open(scratch("wired"), nodes::classes()).unwrap();
    assert_eq!(doc.tree().signals().len(), 1);
    // The receiver itself, as the wired handler calls it.
    let mut p = Player::default();
    p.on_hit(&Hit);
    assert_eq!(p.hits, 1);
}
