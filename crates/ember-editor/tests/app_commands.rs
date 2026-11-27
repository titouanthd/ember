//! M4.2 in the editor window, headless: shortcuts, outline Add / Delete /
//! Rename, and the unsaved-changes dialog (ADR-045).

use ember_core::MainLoop;
use ember_core::math::Rect;
use ember_editor::app::{EditorApp, Outline, Toolbar, build_tree, panel};
use ember_editor::{Document, nodes};
use ember_input::{Input, InputSink, Key};
use ember_scene::{NodeId, SceneLoop};
use ember_servers::{MockAudio, MockRenderer};
use ember_ui::prelude::*;
use glam::Vec2;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

type Editor = SceneLoop<MockRenderer, MockAudio>;

fn scratch() -> PathBuf {
    static N: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "ember-editor-cmd-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/test_fixtures/player.escene");
    let path = dir.join("player.escene");
    std::fs::copy(fixture, &path).unwrap();
    path
}

fn editor_at(path: &PathBuf) -> Editor {
    let doc = Document::open(path, nodes::classes()).unwrap();
    let mut e = SceneLoop::new(
        build_tree(doc),
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

fn idle(e: &mut Editor) -> bool {
    step(e, Input::default())
}

fn app(e: &Editor) -> &EditorApp {
    let root = e.tree.root().unwrap();
    e.tree.get(root).unwrap().as_any().downcast_ref().unwrap()
}

fn widget<T: 'static>(e: &Editor, id: NodeId) -> &T {
    e.tree
        .get(id)
        .unwrap()
        .as_any()
        .downcast_ref::<T>()
        .unwrap()
}

fn click_button(e: &mut Editor, id: NodeId) -> bool {
    let r = widget::<Button>(e, id).rect();
    let pos = Vec2::new(r.x + r.w * 0.5, r.y + r.h * 0.5);
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
    idle(e)
}

/// Press `key` with `held` down.
fn chord(e: &mut Editor, held: &[Key], key: Key) -> bool {
    let mut down = held.to_vec();
    down.push(key);
    let running = step(
        e,
        Input {
            keys_pressed: vec![key],
            keys_down: down,
            ..Input::default()
        },
    );
    running && idle(e)
}

fn outline_names(e: &Editor) -> Vec<String> {
    app(e).doc.outline().into_iter().map(|r| r.name).collect()
}

/// Click the outline row of `name` (after the outline was rebuilt).
fn select(e: &mut Editor, name: &str) {
    let rows = app(e).doc.outline();
    let i = rows.iter().position(|r| r.name == name).unwrap();
    let button = panel::<Outline>(&e.tree).unwrap().rows[i].0;
    click_button(e, button);
    assert_eq!(app(e).selected, Some(rows[i].id));
}

fn ui(e: &Editor) -> Toolbar {
    panel::<Toolbar>(&e.tree).unwrap().clone()
}

#[test]
fn delete_then_ctrl_z_brings_the_node_back() {
    let mut e = editor_at(&scratch());
    select(&mut e, "Hitbox");
    let w = ui(&e);
    click_button(&mut e, w.delete);
    assert_eq!(outline_names(&e), ["Player", "Sprite", "Gun"]);
    assert_eq!(
        panel::<Outline>(&e.tree).unwrap().rows.len(),
        3,
        "the outline was rebuilt"
    );
    assert!(app(&e).doc.is_dirty());
    chord(&mut e, &[Key::LeftControl], Key::Z);
    assert_eq!(outline_names(&e), ["Player", "Sprite", "Gun", "Hitbox"]);
    assert!(!app(&e).doc.is_dirty());
    chord(&mut e, &[Key::LeftSuper, Key::LeftShift], Key::Z);
    assert_eq!(
        outline_names(&e),
        ["Player", "Sprite", "Gun"],
        "Cmd+Shift+Z redoes"
    );
}

#[test]
fn the_delete_key_removes_the_selection_when_not_typing() {
    let mut e = editor_at(&scratch());
    select(&mut e, "Gun");
    chord(&mut e, &[], Key::Delete);
    assert_eq!(outline_names(&e), ["Player", "Sprite", "Hitbox"]);
    // The selection moved to the parent, and the rebuilt rows click.
    assert_eq!(app(&e).selected, app(&e).doc.find("Sprite"));
    select(&mut e, "Hitbox");
}

#[test]
fn add_picks_a_class_and_puts_it_under_the_selection() {
    let mut e = editor_at(&scratch());
    select(&mut e, "Sprite");
    let w = ui(&e);
    assert!(!e.tree.get(w.classes).unwrap().visible());
    click_button(&mut e, w.add);
    assert!(e.tree.get(w.classes).unwrap().visible(), "the picker opens");
    let (sprite_button, _) = *w
        .class_buttons
        .iter()
        .find(|(_, c)| *c == "Sprite")
        .unwrap();
    click_button(&mut e, sprite_button);
    assert!(!e.tree.get(w.classes).unwrap().visible());
    let rows = app(&e).doc.outline();
    // `Sprite` is free among the selected Sprite's children.
    let added = rows
        .iter()
        .find(|r| r.name == "Sprite" && r.depth == 2)
        .expect("added under the selected Sprite");
    assert_eq!(app(&e).selected, Some(added.id));
    chord(&mut e, &[Key::RightControl], Key::Z);
    assert_eq!(outline_names(&e), ["Player", "Sprite", "Gun", "Hitbox"]);
    assert_eq!(app(&e).selected, None, "an undone add drops the selection");
}

#[test]
fn the_name_field_renames_the_selection() {
    let mut e = editor_at(&scratch());
    select(&mut e, "Hitbox");
    let w = ui(&e);
    assert_eq!(widget::<TextInput>(&e, w.name_input).text(), "Hitbox");
    e.tree
        .get_mut(w.name_input)
        .unwrap()
        .as_any_mut()
        .downcast_mut::<TextInput>()
        .unwrap()
        .set_text("Hurtbox");
    click_button(&mut e, w.rename);
    assert_eq!(outline_names(&e), ["Player", "Sprite", "Gun", "Hurtbox"]);
    let text = app(&e).doc.to_scene().unwrap().to_ron();
    assert!(
        text.contains(r#"from: "Hurtbox""#),
        "the connection follows"
    );
}

#[test]
fn ctrl_s_saves() {
    let path = scratch();
    let mut e = editor_at(&path);
    select(&mut e, "Hitbox");
    {
        let b = ui(&e).delete;
        click_button(&mut e, b)
    };
    chord(&mut e, &[Key::LeftControl], Key::S);
    assert!(!app(&e).doc.is_dirty());
    let back = Document::open(&path, nodes::classes()).unwrap();
    assert_eq!(back.outline().len(), 3);
}

#[test]
fn closing_with_changes_opens_the_dialog_and_cancel_keeps_editing() {
    let mut e = editor_at(&scratch());
    select(&mut e, "Hitbox");
    {
        let b = ui(&e).delete;
        click_button(&mut e, b)
    };
    let close = Input {
        close_requested: true,
        ..Input::default()
    };
    assert!(step(&mut e, close), "the close button only asks");
    let w = ui(&e);
    assert!(e.tree.get(w.dialog).unwrap().visible());
    idle(&mut e);
    assert!(
        click_button(&mut e, w.cancel_quit),
        "Cancel keeps the editor open"
    );
    assert!(!e.tree.get(w.dialog).unwrap().visible());
    assert!(app(&e).doc.is_dirty());
}

#[test]
fn save_and_quit_writes_the_file_then_quits() {
    let path = scratch();
    let mut e = editor_at(&path);
    select(&mut e, "Hitbox");
    {
        let b = ui(&e).delete;
        click_button(&mut e, b)
    };
    chord(&mut e, &[], Key::Escape);
    let w = ui(&e);
    assert!(!click_button(&mut e, w.save_quit), "quits");
    let back = Document::open(&path, nodes::classes()).unwrap();
    assert_eq!(back.outline().len(), 3);
}

#[test]
fn quit_without_saving_leaves_the_file() {
    let path = scratch();
    let before = std::fs::read(&path).unwrap();
    let mut e = editor_at(&path);
    select(&mut e, "Hitbox");
    {
        let b = ui(&e).delete;
        click_button(&mut e, b)
    };
    chord(&mut e, &[], Key::Escape);
    assert!(!{
        let b = ui(&e).discard_quit;
        click_button(&mut e, b)
    });
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn a_clean_close_quits_at_once() {
    let mut e = editor_at(&scratch());
    let close = Input {
        close_requested: true,
        ..Input::default()
    };
    assert!(!step(&mut e, close));
}
