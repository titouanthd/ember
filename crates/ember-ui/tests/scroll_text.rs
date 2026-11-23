//! `ScrollContainer` and `TextInput` in a real `UiRoot` (Phase 5.4,
//! ADR-035).

mod common;
use common::*;
use ember_core::math::Rect;
use ember_input::{Input, Key};
use ember_scene::{DrawContext, NodeId};
use ember_servers::{RecordedCall, RenderingServer};
use ember_ui::prelude::*;
use glam::Vec2;

/// A 200×100 scroll area at the origin holding a `VBox` of ten 40 px
/// swatches (400 px of content). Returns the container and the swatches.
fn scroll_list(ui: &mut Ui) -> (NodeId, Vec<NodeId>) {
    let sc = ui.add(ScrollContainer::new().with_min_size(Vec2::new(200.0, 100.0)));
    let list = ui.tree.insert(VBox::new());
    ui.tree.attach(list, sc).unwrap();
    let rows = (0..10)
        .map(|_| {
            let id = ui.tree.insert(Swatch::new(
                ControlBase::new().with_min_size(Vec2::new(0.0, 40.0)),
            ));
            ui.tree.attach(id, list).unwrap();
            id
        })
        .collect();
    (sc, rows)
}

fn wheel(ui: &mut Ui, at: Vec2, y: f32) {
    ui.frame(Input {
        mouse_pos: at,
        mouse_wheel: Vec2::new(0.0, y),
        ..Input::default()
    });
}

// --- ScrollContainer ---------------------------------------------------

#[test]
fn content_overflowing_reserves_the_scrollbar_and_records_its_height() {
    let mut ui = Ui::new();
    let (sc, rows) = scroll_list(&mut ui);
    ui.frame(Input::default());
    ui.frame(Input::default()); // second layout sees the overflow
    let s = ui.get::<ScrollContainer>(sc);
    assert_eq!(s.content_height(), 400.0);
    assert_eq!(s.max_scroll(), 300.0);
    let w = 200.0 - widgets_scrollbar();
    assert_eq!(
        ui.get::<Swatch>(rows[0]).rect(),
        Rect::new(0.0, 0.0, w, 40.0)
    );
    assert_eq!(ui.get::<Swatch>(rows[9]).rect().y, 360.0);
}

fn widgets_scrollbar() -> f32 {
    ember_ui::widgets::SCROLLBAR_WIDTH
}

#[test]
fn wheel_over_the_container_scrolls_its_children_next_layout() {
    let mut ui = Ui::new();
    let (sc, rows) = scroll_list(&mut ui);
    ui.frame(Input::default());
    wheel(&mut ui, Vec2::new(50.0, 50.0), -2.0);
    assert_eq!(ui.get::<ScrollContainer>(sc).scroll(), 80.0);
    ui.frame(Input::default());
    assert_eq!(ui.get::<Swatch>(rows[0]).rect().y, -80.0);
    assert_eq!(ui.get::<Swatch>(rows[2]).rect().y, 0.0);

    wheel(&mut ui, Vec2::new(500.0, 400.0), -2.0);
    assert_eq!(
        ui.get::<ScrollContainer>(sc).scroll(),
        80.0,
        "the wheel elsewhere does nothing"
    );
    wheel(&mut ui, Vec2::new(50.0, 50.0), -100.0);
    assert_eq!(ui.get::<ScrollContainer>(sc).scroll(), 300.0, "clamped");
}

#[test]
fn children_draw_inside_the_clip_and_the_scrollbar_after_it() {
    let mut ui = Ui::new();
    let (_sc, _rows) = scroll_list(&mut ui);
    ui.frame(Input::default());
    ui.frame(Input::default());

    ui.render.begin_frame(VIEWPORT);
    ui.render.clear_calls();
    ui.tree.draw(&mut DrawContext::new(&mut ui.render));
    ui.render.end_frame();
    let calls = ui.render.calls();

    let push = calls
        .iter()
        .position(|c| matches!(c, RecordedCall::PushClipRect { .. }))
        .expect("clip pushed");
    let pop = calls
        .iter()
        .position(|c| matches!(c, RecordedCall::PopClipRect))
        .expect("clip popped");
    let viewport = Rect::new(0.0, 0.0, 200.0 - widgets_scrollbar(), 100.0);
    assert!(matches!(calls[push], RecordedCall::PushClipRect { rect, .. } if rect == viewport));
    let rects_between = calls[push..pop]
        .iter()
        .filter(|c| matches!(c, RecordedCall::DrawRect { .. }))
        .count();
    assert_eq!(rects_between, 10, "every swatch draws inside the clip");
    let thumb = Rect::new(192.0, 0.0, 8.0, 25.0);
    assert!(
        calls[pop..]
            .iter()
            .any(|c| matches!(c, RecordedCall::DrawRect { rect, .. } if *rect == thumb)),
        "scrollbar thumb drawn after the clip closes"
    );
    assert_eq!(
        calls
            .iter()
            .filter(|c| matches!(c, RecordedCall::PushClipRect { .. }))
            .count(),
        calls
            .iter()
            .filter(|c| matches!(c, RecordedCall::PopClipRect))
            .count(),
    );
}

#[test]
fn controls_scrolled_out_of_view_get_no_pointer() {
    let mut ui = Ui::new();
    let sc = ui.add(ScrollContainer::new().with_min_size(Vec2::new(200.0, 100.0)));
    let list = ui.tree.insert(VBox::new());
    ui.tree.attach(list, sc).unwrap();
    let mut buttons = Vec::new();
    for i in 0..5 {
        let b = ui.tree.insert(Button::new(format!("B{i}")));
        ui.tree.attach(b, list).unwrap();
        buttons.push(b);
    }
    ui.frame(Input::default());
    ui.frame(Input::default());
    let hidden = buttons[4];
    let r = ui.get::<Button>(hidden).rect();
    assert!(r.y >= 100.0, "last button starts below the view");

    let p = Vec2::new(r.x + 5.0, r.y + 5.0);
    ui.mouse(p, true, false);
    ui.mouse(p, false, true);
    assert!(ui.clicks().is_empty());
    assert_eq!(ui.get::<Button>(hidden).state(), ButtonState::Normal);

    let first = buttons[0];
    let r = ui.get::<Button>(first).rect();
    let p = Vec2::new(r.x + 5.0, r.y + 5.0);
    ui.mouse(p, true, false);
    ui.mouse(p, false, true);
    assert_eq!(ui.clicks(), vec![first]);
}

// --- TextInput ---------------------------------------------------------

fn typing(chars: &str, keys: &[Key]) -> Input {
    Input {
        chars_typed: chars.chars().collect(),
        keys_pressed: keys.to_vec(),
        ..Input::default()
    }
}

#[test]
fn focused_text_input_takes_typing_space_and_editing_keys() {
    let mut ui = Ui::new();
    let field = ui.add(TextInput::new().with_placeholder("Name"));
    ui.keys(&[Key::Tab]);
    assert_eq!(ui.focused(), Some(field));

    ui.frame(typing("hi", &[Key::H, Key::I]));
    ui.frame(typing(" ", &[Key::Space]));
    ui.frame(typing("yo\u{8}", &[Key::Y, Key::O, Key::Backspace]));
    assert_eq!(ui.get::<TextInput>(field).text(), "hi y");
    assert!(
        ui.submits.lock().unwrap().is_empty(),
        "Space types, it does not submit"
    );

    let changes = ui.text_changes.lock().unwrap().clone();
    assert_eq!(changes.len(), 6);
    assert_eq!(changes.last().unwrap().text, "hi y");
    assert!(changes.iter().all(|c| c.input == field));

    ui.keys(&[Key::Enter]);
    assert_eq!(
        *ui.submits.lock().unwrap(),
        [TextSubmitted {
            input: field,
            text: "hi y".into()
        }]
    );
}

#[test]
fn unfocused_text_input_ignores_typing() {
    let mut ui = Ui::new();
    let field = ui.add(TextInput::new());
    ui.frame(typing("abc", &[Key::A, Key::B, Key::C]));
    assert_eq!(ui.get::<TextInput>(field).text(), "");
}

#[test]
fn space_still_activates_a_focused_button() {
    let mut ui = Ui::new();
    let b = ui.button("Go", 10.0, 10.0);
    ui.keys(&[Key::Tab]);
    ui.frame(typing(" ", &[Key::Space]));
    assert_eq!(ui.clicks(), vec![b]);
}

#[test]
fn text_input_measures_one_line_plus_padding() {
    let mut ui = Ui::new();
    let field = ui.add(TextInput::new());
    ui.frame(Input::default());
    let size = Theme::dark().font_size;
    assert_eq!(
        ui.get::<TextInput>(field).rect().size(),
        Vec2::new(
            ember_ui::widgets::TEXT_INPUT_WIDTH,
            size + 2.0 * ember_ui::widgets::TEXT_INPUT_PADDING
        )
    );
}
