//! M4.2 / ADR-045: every edit is an undoable command, and a save after
//! edits keeps the scene's connections pointing at the right nodes.

use ember_core::Variant;
use ember_editor::nodes;
use ember_editor::{Command, Document, EditorError};
use std::path::PathBuf;

fn open() -> Document {
    let p =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/test_fixtures/player.escene");
    Document::open(p, nodes::classes()).unwrap()
}

fn names(doc: &Document) -> Vec<String> {
    doc.outline()
        .iter()
        .map(|r| format!("{}{}", "  ".repeat(r.depth), r.name))
        .collect()
}

fn ron(doc: &Document) -> String {
    doc.to_scene().unwrap().to_ron()
}

#[test]
fn set_property_undoes_and_redoes() {
    let mut doc = open();
    let sprite = doc.find("Sprite").unwrap();
    doc.set_property_text(sprite, "size", "64, 64").unwrap();
    assert!(doc.is_dirty());
    assert!(doc.undo().unwrap());
    assert_eq!(doc.property_text(sprite, "size").unwrap(), "32.0, 32.0");
    assert!(!doc.is_dirty(), "back at the opened state");
    assert!(doc.redo().unwrap());
    assert_eq!(doc.property_text(sprite, "size").unwrap(), "64.0, 64.0");
    assert!(!doc.redo().unwrap());
}

#[test]
fn a_no_op_edit_pushes_nothing() {
    let mut doc = open();
    let sprite = doc.find("Sprite").unwrap();
    doc.set_property(sprite, "size", &Variant::Vec2(glam::Vec2::splat(32.0)))
        .unwrap();
    assert!(!doc.can_undo());
    assert!(!doc.is_dirty());
}

#[test]
fn add_names_after_the_class_and_undoes() {
    let mut doc = open();
    let root = doc.root();
    let before = ron(&doc);
    let a = doc.add_node(root, "Sprite").unwrap();
    let b = doc.add_node(root, "Sprite").unwrap();
    assert_eq!(
        names(&doc),
        [
            "Player",
            "  Sprite",
            "    Gun",
            "  Hitbox",
            "  Sprite2",
            "  Sprite3"
        ],
        "`Sprite` is taken, so the new ones count on"
    );
    assert!(doc.is_live(a) && doc.is_live(b));
    doc.undo().unwrap();
    doc.undo().unwrap();
    assert!(!doc.is_live(a));
    assert_eq!(ron(&doc), before);
    doc.redo().unwrap();
    assert!(doc.is_live(a), "redo re-attaches the same node");
    assert!(matches!(
        doc.add_node(root, "Nope"),
        Err(EditorError::UnknownClass(_))
    ));
}

#[test]
fn remove_takes_the_subtree_and_undo_restores_its_place() {
    let mut doc = open();
    let before = ron(&doc);
    let sprite = doc.find("Sprite").unwrap();
    let gun = doc.find("Sprite/Gun").unwrap();
    doc.remove_node(sprite).unwrap();
    assert_eq!(names(&doc), ["Player", "  Hitbox"]);
    assert!(!doc.is_live(gun));
    doc.undo().unwrap();
    assert_eq!(names(&doc), ["Player", "  Sprite", "    Gun", "  Hitbox"]);
    assert_eq!(ron(&doc), before, "order and content come back");
    assert_eq!(doc.remove_node(doc.root()), Err(EditorError::IsRoot));
}

#[test]
fn rename_checks_names_and_follows_connections() {
    let mut doc = open();
    let hitbox = doc.find("Hitbox").unwrap();
    doc.rename_node(hitbox, "Hurtbox").unwrap();
    let text = ron(&doc);
    assert!(text.contains(r#"from: "Hurtbox""#), "{text}");
    assert_eq!(
        doc.rename_node(hitbox, "Sprite"),
        Err(EditorError::NameTaken("Sprite".into()))
    );
    assert_eq!(
        doc.rename_node(hitbox, "a/b"),
        Err(EditorError::InvalidName("a/b".into()))
    );
    doc.undo().unwrap();
    assert!(ron(&doc).contains(r#"from: "Hitbox""#));
}

#[test]
fn reparent_moves_updates_paths_and_undoes() {
    let mut doc = open();
    let before = ron(&doc);
    let hitbox = doc.find("Hitbox").unwrap();
    let gun = doc.find("Sprite/Gun").unwrap();
    doc.reparent(hitbox, gun, None).unwrap();
    assert_eq!(
        names(&doc),
        ["Player", "  Sprite", "    Gun", "      Hitbox"]
    );
    assert!(ron(&doc).contains(r#"from: "Sprite/Gun/Hitbox""#));
    // Into its own subtree: refused, nothing pushed.
    let sprite = doc.find("Sprite").unwrap();
    let depth = doc.undo_stack().len();
    assert!(doc.reparent(sprite, hitbox, None).is_err());
    assert_eq!(doc.undo_stack().len(), depth);
    assert!(matches!(
        doc.undo_stack().last(),
        Some(Command::Reparent { .. })
    ));
    doc.undo().unwrap();
    assert_eq!(ron(&doc), before);
}

#[test]
fn a_removed_connection_end_leaves_the_file_until_undone() {
    let mut doc = open();
    let hitbox = doc.find("Hitbox").unwrap();
    doc.remove_node(hitbox).unwrap();
    assert!(!ron(&doc).contains("connections"));
    doc.undo().unwrap();
    assert!(ron(&doc).contains(r#"method: "on_hit""#));
}

#[test]
fn a_new_edit_drops_the_redo_tail_and_dirty_tracks_the_save() {
    let dir = std::env::temp_dir().join(format!("ember-commands-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("player.escene");
    let mut doc = open();
    let root = doc.root();
    doc.add_node(root, "Sprite").unwrap();
    doc.save_as(&path).unwrap();
    assert!(!doc.is_dirty());
    doc.undo().unwrap();
    assert!(doc.is_dirty(), "undo past the save");
    doc.rename_node(root, "Hero").unwrap();
    assert!(!doc.can_redo());
    assert!(doc.is_dirty(), "the saved state is gone from the stack");
    let added = Document::open(&path, nodes::classes()).unwrap();
    assert_eq!(added.outline().len(), 5, "the added node was saved");
}

#[test]
fn addable_classes_come_from_the_class_db() {
    assert_eq!(open().addable_classes(), ["Player", "Sprite"]);
}
