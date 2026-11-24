//! Phase 6 exit criteria on the model (roadmap §9.2): open
//! `player.escene`, inspect `Sprite`, edit `size`, see it drawn, save.

use ember_core::Variant;
use ember_core::math::Rect;
use ember_editor::nodes::{self, Player, Sprite};
use ember_editor::{Document, EditorError};
use ember_scene::{DrawContext, NodeId};
use ember_servers::{MockRenderer, RecordedCall, RenderingServer};
use glam::Vec2;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/test_fixtures/player.escene")
}

fn temp_scene() -> PathBuf {
    static N: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "ember-editor-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("player.escene")
}

fn open() -> Document {
    Document::open(fixture(), nodes::classes()).unwrap()
}

fn sprite(doc: &Document) -> NodeId {
    doc.find("Sprite").unwrap()
}

fn get<T: 'static>(doc: &Document, id: NodeId) -> &T {
    doc.tree()
        .get(id)
        .unwrap()
        .as_any()
        .downcast_ref::<T>()
        .unwrap()
}

fn drawn_rects(doc: &Document) -> Vec<Rect> {
    let mut r = MockRenderer::new();
    r.begin_frame(Rect::new(0.0, 0.0, 800.0, 600.0));
    doc.tree().draw(&mut DrawContext::new(&mut r));
    r.end_frame();
    r.calls()
        .iter()
        .filter_map(|c| match c {
            RecordedCall::DrawRect { rect, .. } => Some(*rect),
            _ => None,
        })
        .collect()
}

#[test]
fn opens_the_fixture_and_lists_the_tree() {
    let doc = open();
    let rows = doc.outline();
    let names: Vec<_> = rows
        .iter()
        .map(|r| (r.name.as_str(), r.class, r.depth))
        .collect();
    assert_eq!(
        names,
        [
            ("Player", "Player", 0),
            ("Sprite", "Sprite", 1),
            ("Gun", "Sprite", 2),
            ("Hitbox", "Sprite", 1),
        ]
    );
    assert!(!doc.is_dirty());
}

#[test]
fn sprite_shows_texture_and_size() {
    let doc = open();
    let keys: Vec<_> = doc
        .properties(sprite(&doc))
        .unwrap()
        .into_iter()
        .map(|(k, _)| k)
        .collect();
    assert!(keys.contains(&"texture"));
    assert!(keys.contains(&"size"));
    assert_eq!(
        doc.property_text(sprite(&doc), "size").unwrap(),
        "32.0, 32.0"
    );
    let player = doc.root();
    assert_eq!(doc.property_text(player, "speed").unwrap(), "5.0");
    assert_eq!(doc.property_text(player, "max_hp").unwrap(), "100.0");
}

#[test]
fn editing_size_updates_the_preview() {
    let mut doc = open();
    let id = sprite(&doc);
    let small = Rect::new(0.0, 0.0, 32.0, 32.0);
    let big = Rect::new(0.0, 0.0, 64.0, 64.0);
    let count = |rects: &[Rect], r: Rect| rects.iter().filter(|&&x| x == r).count();
    // Sprite and Gun (default size) are drawn; Hitbox is hidden.
    let before = drawn_rects(&doc);
    assert_eq!((count(&before, small), count(&before, big)), (2, 0));

    doc.set_property_text(id, "size", "64, 64").unwrap();
    assert!(doc.is_dirty());
    assert_eq!(get::<Sprite>(&doc, id).size, Vec2::new(64.0, 64.0));
    let after = drawn_rects(&doc);
    assert_eq!((count(&after, small), count(&after, big)), (1, 1));
}

#[test]
fn saving_round_trips_the_change() {
    let path = temp_scene();
    let mut doc = open();
    let id = sprite(&doc);
    doc.set_property_text(id, "size", "64, 64").unwrap();
    doc.save_as(&path).unwrap();
    assert!(!doc.is_dirty());
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("Vec2(64.0, 64.0)"), "{text}");

    let back = Document::open(&path, nodes::classes()).unwrap();
    let s = get::<Sprite>(&back, back.find("Sprite").unwrap());
    assert_eq!((s.size, s.z), (Vec2::new(64.0, 64.0), 1.0));
    let hitbox = get::<Sprite>(&back, back.find("Hitbox").unwrap());
    assert!(!hitbox.visible, "hidden nodes stay hidden");
    let p = get::<Player>(&back, back.root());
    assert_eq!((p.speed, p.max_hp), (5.0, 100.0));
    assert_eq!(back.outline().len(), 4);

    // A second save of the reloaded scene writes the same file.
    let mut again = back;
    again.save().unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
}

#[test]
fn bad_edits_are_refused_and_leave_the_document_clean() {
    let mut doc = open();
    let id = sprite(&doc);
    assert_eq!(
        doc.set_property_text(id, "size", "64"),
        Err(EditorError::BadValue {
            key: "size".into(),
            expected: "x, y"
        })
    );
    assert_eq!(
        doc.set_property_text(id, "mass", "3"),
        Err(EditorError::UnknownProperty("mass".into()))
    );
    assert_eq!(
        doc.set_property(id, "size", &Variant::Bool(true)),
        Err(EditorError::Rejected("size".into()))
    );
    assert!(!doc.is_dirty());
    assert_eq!(get::<Sprite>(&doc, id).size, Vec2::new(32.0, 32.0));
}

#[test]
fn a_document_without_a_file_cannot_save() {
    let scene = ember_scene::PackedScene::from_ron(
        r#"(format: 1, uid: "uid://new", nodes: [(name: "Root", class: "Player")])"#,
    )
    .unwrap();
    let mut doc = Document::from_scene(&scene, nodes::classes()).unwrap();
    assert_eq!(doc.save(), Err(EditorError::NoPath));
    let path = temp_scene();
    doc.save_as(&path).unwrap();
    assert_eq!(doc.path(), Some(path.as_path()));
}
