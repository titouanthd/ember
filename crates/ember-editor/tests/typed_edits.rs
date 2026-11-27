//! M4.3a, headless `Document`: enum hints (ADR-049), one-part edits of
//! `Vec2` / `Color`, and outline moves (`shift`), each one undoable
//! command.

use ember_core::{Color, Variant};
use ember_editor::{Command, Document, EditorError, Shift, nodes};
use ember_scene::PropertyHint;
use glam::Vec2;
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

#[test]
fn the_document_passes_enum_hints_on() {
    let doc = open();
    let sprite = doc.find("Sprite").unwrap();
    assert_eq!(
        doc.property_hint(sprite, "fit"),
        PropertyHint::Enum(&["Stretch", "Tile", "Keep"])
    );
    assert_eq!(doc.property_hint(sprite, "size"), PropertyHint::None);
    assert_eq!(doc.property_hint(doc.root(), "speed"), PropertyHint::None);
}

#[test]
fn an_enum_value_is_set_as_a_string_and_refused_when_unknown() {
    let mut doc = open();
    let sprite = doc.find("Sprite").unwrap();
    doc.set_property(sprite, "fit", &Variant::String("Tile".into()))
        .unwrap();
    assert_eq!(doc.property_text(sprite, "fit").unwrap(), "Tile");
    assert_eq!(
        doc.set_property(sprite, "fit", &Variant::String("Squash".into())),
        Err(EditorError::Rejected("fit".into()))
    );
    assert_eq!(doc.undo_stack().len(), 1);
    assert!(doc.undo().unwrap());
    assert_eq!(doc.property_text(sprite, "fit").unwrap(), "Stretch");
}

#[test]
fn one_part_of_a_vec2_or_colour_is_one_command() {
    let mut doc = open();
    let sprite = doc.find("Sprite").unwrap();
    doc.set_property_component(sprite, "size", 1, "48").unwrap();
    doc.set_property_component(sprite, "color", 3, "0.5")
        .unwrap();
    let props = doc.properties(sprite).unwrap();
    let get = |k: &str| props.iter().find(|(key, _)| *key == k).unwrap().1.clone();
    assert_eq!(get("size"), Variant::Vec2(Vec2::new(32.0, 48.0)));
    assert_eq!(get("color"), Variant::Color(Color::new(0.4, 0.7, 1.0, 0.5)));
    assert_eq!(doc.undo_stack().len(), 2);
    assert!(matches!(
        doc.undo_stack()[0],
        Command::SetProperty { key: "size", .. }
    ));
    assert_eq!(
        doc.set_property_component(sprite, "size", 0, "wide"),
        Err(EditorError::BadValue {
            key: "size".into(),
            expected: "a number"
        })
    );
    assert_eq!(doc.undo_stack().len(), 2, "a refused edit pushes nothing");
}

#[test]
fn shift_moves_up_down_in_and_out_undoably() {
    let mut doc = open();
    let hitbox = doc.find("Hitbox").unwrap();
    let start = names(&doc);
    assert_eq!(start, ["Player", "  Sprite", "    Gun", "  Hitbox"]);

    doc.shift(hitbox, Shift::Up).unwrap();
    assert_eq!(names(&doc), ["Player", "  Hitbox", "  Sprite", "    Gun"]);
    assert_eq!(
        doc.shift(hitbox, Shift::Up),
        Err(EditorError::CannotMove("already first"))
    );
    assert_eq!(
        doc.shift(hitbox, Shift::In),
        Err(EditorError::CannotMove("no sibling above"))
    );
    doc.shift(hitbox, Shift::Down).unwrap();
    doc.shift(hitbox, Shift::In).unwrap();
    assert_eq!(names(&doc), ["Player", "  Sprite", "    Gun", "    Hitbox"]);
    doc.shift(hitbox, Shift::Out).unwrap();
    assert_eq!(names(&doc), start);
    assert_eq!(
        doc.shift(hitbox, Shift::Out),
        Err(EditorError::CannotMove("already under the root"))
    );
    assert_eq!(doc.shift(doc.root(), Shift::Up), Err(EditorError::IsRoot));

    assert_eq!(doc.undo_stack().len(), 4);
    while doc.undo().unwrap() {}
    assert_eq!(names(&doc), start);
    assert!(!doc.is_dirty());
}
