//! The editor window, headless (session 6.2): select a node in the
//! outline, edit `size` in the inspector, see the preview, save.

use ember_core::MainLoop;
use ember_core::math::Rect;
use ember_editor::app::{EditorApp, build_tree, preview_rect};
use ember_editor::{Document, nodes};
use ember_input::{Input, InputSink, Key};
use ember_scene::{NodeId, SceneLoop};
use ember_servers::{MockAudio, MockRenderer, RecordedCall};
use ember_ui::prelude::*;
use glam::Vec2;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

type Editor = SceneLoop<MockRenderer, MockAudio>;

/// The fixture, copied so saving does not touch the repo.
fn document() -> Document {
    static N: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "ember-editor-app-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/test_fixtures/player.escene");
    let path = dir.join("player.escene");
    std::fs::copy(fixture, &path).unwrap();
    Document::open(path, nodes::classes()).unwrap()
}

fn editor() -> Editor {
    let mut e = SceneLoop::new(
        build_tree(document()),
        MockRenderer::new(),
        MockAudio::new(),
        Rect::new(0.0, 0.0, 1100.0, 700.0),
    );
    assert!(e.init());
    e.draw();
    step(&mut e, Input::default());
    e
}

fn step(e: &mut Editor, input: Input) -> bool {
    e.set_input(input);
    let running = e.process(1.0 / 60.0);
    e.draw();
    running
}

fn app(e: &Editor) -> &EditorApp {
    let root = e.tree.root().unwrap();
    e.tree
        .get(root)
        .unwrap()
        .as_any()
        .downcast_ref::<EditorApp>()
        .unwrap()
}

fn widget<T: 'static>(e: &Editor, id: NodeId) -> &T {
    e.tree
        .get(id)
        .unwrap()
        .as_any()
        .downcast_ref::<T>()
        .unwrap()
}

fn widget_mut<T: 'static>(e: &mut Editor, id: NodeId) -> &mut T {
    e.tree
        .get_mut(id)
        .unwrap()
        .as_any_mut()
        .downcast_mut::<T>()
        .unwrap()
}

fn label(e: &Editor, id: NodeId) -> String {
    widget::<Label>(e, id).text.clone()
}

fn click_rect(e: &mut Editor, r: Rect) {
    let pos = Vec2::new(r.x + r.w * 0.5, r.y + r.h * 0.5);
    let press = Input {
        mouse_pos: pos,
        mouse_left_pressed: true,
        mouse_left_down: true,
        ..Input::default()
    };
    let release = Input {
        mouse_pos: pos,
        mouse_left_released: true,
        ..Input::default()
    };
    step(e, press);
    step(e, release);
    step(e, Input::default()); // intents apply on the next process
}

fn click_button(e: &mut Editor, id: NodeId) {
    let r = widget::<Button>(e, id).rect();
    click_rect(e, r);
}

fn press(e: &mut Editor, key: Key) -> bool {
    let running = step(
        e,
        Input {
            keys_pressed: vec![key],
            keys_down: vec![key],
            ..Input::default()
        },
    );
    step(e, Input::default());
    running
}

/// The outline button of the scene node called `name`.
fn outline_button(e: &Editor, name: &str) -> NodeId {
    let rows = app(e).doc.outline();
    let ui = app(e).widgets().unwrap();
    let i = rows.iter().position(|r| r.name == name).unwrap();
    ui.outline[i].0
}

/// `(key, input)` of the inspector row showing `key`.
fn row(e: &Editor, key: &str) -> NodeId {
    let ui = app(e).widgets().unwrap();
    ui.rows
        .iter()
        .find(|&&(row, k, _)| e.tree.get(row).unwrap().visible() && label(e, k) == key)
        .map(|&(_, _, input)| input)
        .unwrap_or_else(|| panic!("no row for {key}"))
}

fn select_sprite(e: &mut Editor) {
    let b = outline_button(e, "Sprite");
    click_button(e, b);
}

/// Type `text` into the property row `key` and press Enter.
fn edit(e: &mut Editor, key: &str, text: &str) {
    let input = row(e, key);
    let r = widget::<TextInput>(e, input).rect();
    click_rect(e, r); // focus
    widget_mut::<TextInput>(e, input).set_text(text);
    press(e, Key::Enter);
}

fn drawn_sizes(e: &Editor) -> Vec<Vec2> {
    e.rendering()
        .calls()
        .iter()
        .filter_map(|c| match c {
            RecordedCall::DrawRect { rect, .. } => Some(Vec2::new(rect.w, rect.h)),
            _ => None,
        })
        .collect()
}

#[test]
fn the_outline_lists_every_scene_node() {
    let e = editor();
    let rows = app(&e).doc.outline();
    let ui = app(&e).widgets().unwrap();
    assert_eq!(ui.outline.len(), rows.len());
    for (&(button, _), row) in ui.outline.iter().zip(&rows) {
        assert!(widget::<Button>(&e, button).text.ends_with(&row.name));
    }
    assert_eq!(label(&e, ui.header), "Nothing selected");
}

#[test]
fn clicking_a_node_fills_the_inspector() {
    let mut e = editor();
    select_sprite(&mut e);
    let ui = app(&e).widgets().unwrap();
    assert_eq!(label(&e, ui.header), "Sprite (Sprite)");
    let size = row(&e, "size");
    assert_eq!(widget::<TextInput>(&e, size).text(), "32.0, 32.0");
    let _texture = row(&e, "texture");
    let b = outline_button(&e, "Sprite");
    assert!(
        widget::<Button>(&e, b).text.starts_with('>'),
        "selection marker"
    );
}

#[test]
fn editing_size_updates_the_document_and_the_preview() {
    let mut e = editor();
    select_sprite(&mut e);
    edit(&mut e, "size", "64,64");
    let a = app(&e);
    assert!(a.doc.is_dirty());
    assert!(a.status.contains("size = 64.0, 64.0"), "{}", a.status);
    assert_eq!(
        widget::<TextInput>(&e, row(&e, "size")).text(),
        "64.0, 64.0"
    );
    assert!(drawn_sizes(&e).contains(&Vec2::new(64.0, 64.0)));
    assert_eq!(
        widget::<Button>(&e, a.widgets().unwrap().save).text,
        "Save *"
    );
}

#[test]
fn a_bad_value_is_reported_and_the_field_restored() {
    let mut e = editor();
    select_sprite(&mut e);
    edit(&mut e, "size", "huge");
    assert!(!app(&e).doc.is_dirty());
    assert!(app(&e).status.contains("x, y"), "{}", app(&e).status);
    assert_eq!(
        widget::<TextInput>(&e, row(&e, "size")).text(),
        "32.0, 32.0"
    );
}

#[test]
fn save_writes_the_edit_and_clears_dirty() {
    let mut e = editor();
    select_sprite(&mut e);
    edit(&mut e, "size", "64, 64");
    let save = app(&e).widgets().unwrap().save;
    click_button(&mut e, save);
    assert!(!app(&e).doc.is_dirty());
    assert!(app(&e).status.starts_with("Saved"), "{}", app(&e).status);
    let path = app(&e).doc.path().unwrap().to_path_buf();
    let text = std::fs::read_to_string(path).unwrap();
    assert!(text.contains("Vec2(64.0, 64.0)"), "{text}");
}

#[test]
fn the_preview_is_clipped_to_its_area() {
    let e = editor();
    let area = preview_rect();
    assert!(
        e.rendering()
            .calls()
            .iter()
            .any(|c| matches!(c, RecordedCall::PushClipRect { rect, .. } if *rect == area))
    );
}

#[test]
fn escape_quits_but_asks_twice_with_unsaved_changes() {
    let mut e = editor();
    select_sprite(&mut e);
    edit(&mut e, "size", "64, 64");
    assert!(press(&mut e, Key::Escape), "first Escape only warns");
    assert!(app(&e).status.contains("Escape again"));
    assert!(!press(&mut e, Key::Escape), "second Escape quits");
}
