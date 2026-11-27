//! M4.3a in the editor window, headless: typed inspector editors (each
//! edit one undoable `SetProperty`), no row cap, preview click-to-select
//! and outline moves.

use ember_core::math::Rect;
use ember_core::{ClassDB, Color, MainLoop, Variant};
use ember_editor::app::{EditorApp, Inspector, Outline, build_tree, panel, preview_camera};
use ember_editor::{Command, Document, nodes};
use ember_input::{Input, InputSink, Key};
use ember_macros::Node;
use ember_scene::{ClassDbNodes, NodeId, SceneLoop};
use ember_servers::{MockAudio, MockRenderer};
use ember_ui::prelude::*;
use glam::Vec2;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

type Editor = SceneLoop<MockRenderer, MockAudio>;

fn scratch(file: &str) -> PathBuf {
    static N: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "ember-editor-insp-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(file)
}

fn player() -> Document {
    let path = scratch("player.escene");
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/test_fixtures/player.escene");
    std::fs::copy(fixture, &path).unwrap();
    Document::open(path, nodes::classes()).unwrap()
}

fn editor(doc: Document) -> Editor {
    let mut e = SceneLoop::new(
        build_tree(doc),
        MockRenderer::new(),
        MockAudio::new(),
        Rect::new(0.0, 0.0, 1100.0, 700.0),
    );
    assert!(e.init());
    e.draw();
    idle(&mut e);
    e
}

fn step(e: &mut Editor, input: Input) {
    e.set_input(input);
    e.process(1.0 / 60.0);
    e.draw();
}

fn idle(e: &mut Editor) {
    step(e, Input::default());
}

fn app(e: &Editor) -> &EditorApp {
    let root = e.tree.root().unwrap();
    e.tree.get(root).unwrap().as_any().downcast_ref().unwrap()
}

fn inspector(e: &Editor) -> Inspector {
    panel::<Inspector>(&e.tree).unwrap().clone()
}

fn widget<T: 'static>(e: &Editor, id: NodeId) -> &T {
    e.tree.get(id).unwrap().as_any().downcast_ref().unwrap()
}

fn click_at(e: &mut Editor, pos: Vec2) {
    step(
        e,
        Input {
            mouse_pos: pos,
            mouse_left_pressed: true,
            mouse_left_down: true,
            ..Input::default()
        },
    );
    step(
        e,
        Input {
            mouse_pos: pos,
            mouse_left_released: true,
            ..Input::default()
        },
    );
    idle(e); // requests apply on the next process
    idle(e); // rows spawned then are laid out
}

fn click(e: &mut Editor, id: NodeId) {
    let r = e
        .tree
        .get(id)
        .unwrap()
        .as_any()
        .downcast_ref::<Button>()
        .map(|b| b.rect())
        .or_else(|| {
            e.tree
                .get(id)
                .unwrap()
                .as_any()
                .downcast_ref::<TextInput>()
                .map(|t| t.rect())
        })
        .unwrap();
    click_at(e, Vec2::new(r.x + r.w / 2.0, r.y + r.h / 2.0));
}

fn select(e: &mut Editor, name: &str) {
    let rows = app(e).doc.outline();
    let i = rows.iter().position(|r| r.name == name).unwrap();
    let button = panel::<Outline>(&e.tree).unwrap().rows[i].0;
    click(e, button);
    assert_eq!(app(e).selected, Some(rows[i].id));
}

fn value(e: &Editor, name: &str, key: &str) -> Variant {
    let doc = &app(e).doc;
    let id = doc.find(if name == "Player" { "" } else { name }).unwrap();
    doc.properties(id)
        .unwrap()
        .into_iter()
        .find(|(k, _)| *k == key)
        .unwrap()
        .1
}

fn type_into(e: &mut Editor, field: NodeId, text: &str) {
    click(e, field);
    e.tree
        .get_mut(field)
        .unwrap()
        .as_any_mut()
        .downcast_mut::<TextInput>()
        .unwrap()
        .set_text(text);
    let enter = Input {
        keys_pressed: vec![Key::Enter],
        keys_down: vec![Key::Enter],
        ..Input::default()
    };
    step(e, enter);
    idle(e);
}

fn chord_undo(e: &mut Editor) {
    let held = Input {
        keys_pressed: vec![Key::Z],
        keys_down: vec![Key::LeftControl, Key::Z],
        ..Input::default()
    };
    step(e, held);
    idle(e);
}

#[test]
fn a_bool_is_a_toggle_button() {
    let mut e = editor(player());
    select(&mut e, "Sprite");
    let toggle = inspector(&e).button("visible").expect("a toggle");
    assert_eq!(widget::<Button>(&e, toggle).text, "[x] true");
    click(&mut e, toggle);
    assert_eq!(value(&e, "Sprite", "visible"), Variant::Bool(false));
    assert_eq!(
        widget::<Button>(&e, toggle).text,
        "[ ] false",
        "same row, updated"
    );
    assert_eq!(app(&e).doc.undo_stack().len(), 1);
    chord_undo(&mut e);
    assert_eq!(value(&e, "Sprite", "visible"), Variant::Bool(true));
}

#[test]
fn an_enum_cycles_through_its_names() {
    let mut e = editor(player());
    select(&mut e, "Sprite");
    let cycle = inspector(&e).button("fit").expect("a cycle button");
    assert_eq!(widget::<Button>(&e, cycle).text, "Stretch >");
    for want in ["Tile", "Keep", "Stretch"] {
        click(&mut e, cycle);
        assert_eq!(value(&e, "Sprite", "fit"), Variant::String(want.into()));
    }
    assert_eq!(app(&e).doc.undo_stack().len(), 3, "one command per click");
}

#[test]
fn a_colour_has_a_swatch_and_four_fields() {
    let mut e = editor(player());
    select(&mut e, "Sprite");
    let i = inspector(&e);
    let fields = i.fields("color");
    assert_eq!(fields.len(), 4);
    let swatch = i.swatch("color").expect("a swatch");
    type_into(&mut e, fields[0], "1");
    let want = Color::new(1.0, 0.7, 1.0, 1.0);
    assert_eq!(value(&e, "Sprite", "color"), Variant::Color(want));
    assert_eq!(widget::<Panel>(&e, swatch).background, Some(want));
    assert!(matches!(
        app(&e).doc.undo_stack(),
        [Command::SetProperty { key: "color", .. }]
    ));
}

#[test]
fn numbers_and_vec2_parts_are_text_fields() {
    let mut e = editor(player());
    select(&mut e, "Player");
    let speed = inspector(&e).fields("speed");
    assert_eq!(speed.len(), 1);
    type_into(&mut e, speed[0], "7.5");
    assert_eq!(value(&e, "Player", "speed"), Variant::Float(7.5));
    let position = inspector(&e).fields("position");
    assert_eq!(position.len(), 2);
    type_into(&mut e, position[1], "40");
    assert_eq!(
        value(&e, "Player", "position"),
        Variant::Vec2(Vec2::new(0.0, 40.0))
    );
    assert_eq!(app(&e).doc.undo_stack().len(), 2);
}

// --- No row cap ---------------------------------------------------------

#[derive(Node, Default)]
#[class_name = "Many"]
struct Many {
    name: String,
    #[export]
    p00: f32,
    #[export]
    p01: f32,
    #[export]
    p02: f32,
    #[export]
    p03: f32,
    #[export]
    p04: f32,
    #[export]
    p05: f32,
    #[export]
    p06: f32,
    #[export]
    p07: f32,
    #[export]
    p08: f32,
    #[export]
    p09: f32,
    #[export]
    p10: f32,
    #[export]
    p11: f32,
    #[export]
    p12: f32,
    #[export]
    p13: f32,
    #[export]
    p14: f32,
    #[export]
    p15: f32,
    #[export]
    p16: f32,
    #[export]
    p17: f32,
    #[export]
    p18: f32,
    #[export]
    p19: f32,
}

impl ember_scene::Node for Many {}

fn many(path: &Path) -> Document {
    std::fs::write(
        path,
        r#"(format: 1, uid: "uid://many", nodes: [(name: "Many", class: "Many")])"#,
    )
    .unwrap();
    let mut classes = ClassDB::new();
    classes.register_node::<Many>();
    Document::open(path, classes).unwrap()
}

#[test]
fn every_property_gets_a_row_in_a_scroll_area() {
    let mut e = editor(many(&scratch("many.escene")));
    select(&mut e, "Many");
    let i = inspector(&e);
    assert_eq!(i.rows.len(), 20, "no 16-row cap");
    let scroll = widget::<ScrollContainer>(&e, i.scroll);
    assert!(scroll.max_scroll() > 0.0, "the rows overflow and scroll");
}

// --- Preview and outline ------------------------------------------------

#[test]
fn clicking_the_preview_selects_the_topmost_node() {
    let mut e = editor(player());
    // Sprite (z 1) and Gun (z 0) both cover 0..32; Sprite is on top.
    let at = preview_camera(None).world_to_screen(Vec2::new(16.0, 16.0));
    click_at(&mut e, at);
    let sprite = app(&e).doc.find("Sprite").unwrap();
    assert_eq!(app(&e).selected, Some(sprite));
    assert_eq!(inspector(&e).node, Some(sprite));
    let empty = preview_camera(None).world_to_screen(Vec2::new(300.0, 300.0));
    click_at(&mut e, empty);
    assert_eq!(app(&e).selected, None);
}

#[test]
fn outline_buttons_move_the_selection_undoably() {
    let mut e = editor(player());
    select(&mut e, "Hitbox");
    let moves = panel::<Outline>(&e.tree).unwrap().moves.clone();
    let button = |name: &str| {
        moves[["Up", "Down", "In", "Out"]
            .iter()
            .position(|n| *n == name)
            .unwrap()]
        .0
    };
    click(&mut e, button("Up"));
    let names =
        |e: &Editor| -> Vec<String> { app(e).doc.outline().into_iter().map(|r| r.name).collect() };
    assert_eq!(names(&e), ["Player", "Hitbox", "Sprite", "Gun"]);
    assert_eq!(panel::<Outline>(&e.tree).unwrap().rows.len(), 4, "rebuilt");
    click(&mut e, button("Up"));
    assert!(
        app(&e).status.contains("already first"),
        "{}",
        app(&e).status
    );
    chord_undo(&mut e);
    assert_eq!(names(&e), ["Player", "Sprite", "Gun", "Hitbox"]);
}
