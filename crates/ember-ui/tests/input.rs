//! Input routing, focus and signals through a real `UiRoot` (Phase 5.3,
//! ADR-036).

mod common;
use common::*;
use ember_core::math::Rect;
use ember_core::{Color, Object};
use ember_input::{GamepadButton, Input, Key, MouseButton};
use ember_scene::NodeName;
use ember_ui::prelude::*;
use ember_ui::widgets;
use glam::Vec2;
use std::any::Any;

/// Records every mouse callback it gets.
struct Recorder {
    name: String,
    base: ControlBase,
    log: Vec<String>,
}

impl Recorder {
    fn new(rect: Rect) -> Self {
        Self {
            name: "Recorder".into(),
            base: ControlBase::new()
                .with_offset(Vec2::new(rect.x, rect.y))
                .with_min_size(rect.size()),
            log: Vec::new(),
        }
    }
}

impl Object for Recorder {
    fn class_name() -> &'static str {
        "Recorder"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl NodeName for Recorder {
    fn name(&self) -> &str {
        &self.name
    }
    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

impl Control for Recorder {
    fn base(&self) -> &ControlBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ControlBase {
        &mut self.base
    }
    fn draw_control(&self, ui: &mut UiDrawContext) {
        ui.draw_background(self.rect(), Color::RED);
    }
    fn on_mouse_enter(&mut self) {
        self.log.push("enter".into());
    }
    fn on_mouse_leave(&mut self) {
        self.log.push("leave".into());
    }
    fn on_mouse_down(&mut self, button: MouseButton, _pos: Vec2) {
        self.log.push(format!("down {button:?}"));
    }
    fn on_mouse_up(&mut self, button: MouseButton, _pos: Vec2) {
        self.log.push(format!("up {button:?}"));
    }
    fn on_mouse_wheel(&mut self, delta: f32) {
        self.log.push(format!("wheel {delta}"));
    }
}

control_node!(Recorder);

// --- Focus -------------------------------------------------------------

#[test]
fn tab_cycles_focus_in_tree_order_and_skips_disabled() {
    let mut ui = Ui::with_root(
        UiRoot::default()
            .register::<Recorder>()
            .register::<Swatch>(),
    );
    let play = ui.button("Play", 10.0, 10.0);
    let off = ui.add(
        Button::new("Off")
            .disabled()
            .with_offset(Vec2::new(10.0, 60.0)),
    );
    let quit = ui.button("Quit", 10.0, 110.0);
    let _ = off;

    ui.keys(&[Key::Tab]);
    assert_eq!(ui.focused(), Some(play));
    assert!(ui.get::<Button>(play).base().focused);

    ui.keys(&[Key::Tab]);
    assert_eq!(ui.focused(), Some(quit));
    assert!(!ui.get::<Button>(play).base().focused);

    ui.keys(&[Key::Tab]);
    assert_eq!(ui.focused(), Some(play), "wraps around");

    ui.keys(&[Key::Up]);
    assert_eq!(ui.focused(), Some(quit), "Up goes backwards");

    let changes = ui.focus_changes.lock().unwrap().clone();
    assert_eq!(changes.len(), 4);
    assert_eq!(
        changes[0],
        FocusChanged {
            from: None,
            to: Some(play)
        }
    );
    assert_eq!(
        changes[1],
        FocusChanged {
            from: Some(play),
            to: Some(quit)
        }
    );
}

#[test]
fn dpad_moves_focus() {
    let mut ui = Ui::with_root(
        UiRoot::default()
            .register::<Recorder>()
            .register::<Swatch>(),
    );
    let a = ui.button("A", 10.0, 10.0);
    let b = ui.button("B", 10.0, 60.0);
    let pad = |button| Input {
        gamepad_pressed: vec![button],
        ..Input::default()
    };
    ui.frame(pad(GamepadButton::DPadDown));
    assert_eq!(ui.focused(), Some(a));
    ui.frame(pad(GamepadButton::DPadDown));
    assert_eq!(ui.focused(), Some(b));
    ui.frame(pad(GamepadButton::DPadUp));
    assert_eq!(ui.focused(), Some(a));
}

#[test]
fn request_focus_applies_next_frame_and_ignores_non_focusables() {
    let mut ui = Ui::with_root(
        UiRoot::default()
            .register::<Recorder>()
            .register::<Swatch>(),
    );
    let _a = ui.button("A", 10.0, 10.0);
    let b = ui.button("B", 10.0, 60.0);
    let label = ui.add(Label::new("Score"));

    let root = ui.root;
    ui.get_mut::<UiRoot>(root).request_focus(Some(b));
    assert_eq!(ui.focused(), None);
    ui.frame(Input::default());
    assert_eq!(ui.focused(), Some(b));

    ui.get_mut::<UiRoot>(root).request_focus(Some(label));
    ui.frame(Input::default());
    assert_eq!(ui.focused(), None, "a label cannot take focus");
}

#[test]
fn hiding_the_focused_control_drops_focus() {
    let mut ui = Ui::with_root(
        UiRoot::default()
            .register::<Recorder>()
            .register::<Swatch>(),
    );
    let a = ui.button("A", 10.0, 10.0);
    ui.keys(&[Key::Tab]);
    assert_eq!(ui.focused(), Some(a));
    ui.get_mut::<Button>(a).base_mut().visible = false;
    ui.frame(Input::default());
    assert_eq!(ui.focused(), None);
}

// --- Activation --------------------------------------------------------

#[test]
fn enter_space_and_gamepad_south_click_the_focused_button() {
    let mut ui = Ui::with_root(
        UiRoot::default()
            .register::<Recorder>()
            .register::<Swatch>(),
    );
    let _a = ui.button("A", 10.0, 10.0);
    let b = ui.button("B", 10.0, 60.0);

    ui.keys(&[Key::Enter]);
    assert!(ui.clicks().is_empty(), "nothing focused yet");

    ui.keys(&[Key::Tab]);
    ui.keys(&[Key::Tab]);
    ui.keys(&[Key::Enter]);
    ui.keys(&[Key::Space]);
    ui.frame(Input {
        gamepad_pressed: vec![GamepadButton::South],
        ..Input::default()
    });
    assert_eq!(ui.clicks(), vec![b, b, b]);
}

#[test]
fn enter_and_space_in_one_frame_click_once() {
    let mut ui = Ui::with_root(
        UiRoot::default()
            .register::<Recorder>()
            .register::<Swatch>(),
    );
    let a = ui.button("A", 10.0, 10.0);
    ui.keys(&[Key::Tab]);
    ui.keys(&[Key::Enter, Key::Space]);
    assert_eq!(ui.clicks(), vec![a]);
}

// --- Mouse -------------------------------------------------------------

#[test]
fn press_and_release_inside_clicks_and_focuses() {
    let mut ui = Ui::with_root(
        UiRoot::default()
            .register::<Recorder>()
            .register::<Swatch>(),
    );
    let play = ui.button("Play", 10.0, 10.0);
    let inside = Vec2::new(20.0, 20.0);

    ui.mouse(inside, false, false);
    assert_eq!(ui.get::<Button>(play).state(), ButtonState::Hover);

    ui.mouse(inside, true, false);
    assert_eq!(ui.get::<Button>(play).state(), ButtonState::Pressed);
    assert_eq!(ui.focused(), Some(play), "a click focuses");
    assert!(ui.clicks().is_empty(), "clicks on release");

    ui.mouse(inside, false, true);
    assert_eq!(ui.clicks(), vec![play]);
    assert_eq!(ui.get::<Button>(play).state(), ButtonState::Hover);
    assert!(!ui.get::<Button>(play).base().pressed);
}

#[test]
fn releasing_outside_does_not_click() {
    let mut ui = Ui::with_root(
        UiRoot::default()
            .register::<Recorder>()
            .register::<Swatch>(),
    );
    let play = ui.button("Play", 10.0, 10.0);
    ui.mouse(Vec2::new(20.0, 20.0), true, false);
    ui.mouse(Vec2::new(400.0, 400.0), false, true);
    assert!(ui.clicks().is_empty());
    assert_eq!(ui.get::<Button>(play).state(), ButtonState::Normal);
    assert!(!ui.get::<Button>(play).base().pressed);
}

#[test]
fn pressing_outside_then_releasing_inside_does_not_click() {
    let mut ui = Ui::with_root(
        UiRoot::default()
            .register::<Recorder>()
            .register::<Swatch>(),
    );
    let play = ui.button("Play", 10.0, 10.0);
    ui.mouse(Vec2::new(400.0, 400.0), true, false);
    ui.mouse(Vec2::new(20.0, 20.0), false, true);
    assert!(ui.clicks().is_empty());
    assert_eq!(ui.focused(), None);
    let _ = play;
}

#[test]
fn disabled_buttons_ignore_the_mouse() {
    let mut ui = Ui::with_root(
        UiRoot::default()
            .register::<Recorder>()
            .register::<Swatch>(),
    );
    let off = ui.add(
        Button::new("Off")
            .disabled()
            .with_offset(Vec2::new(10.0, 10.0)),
    );
    ui.mouse(Vec2::new(20.0, 20.0), true, false);
    ui.mouse(Vec2::new(20.0, 20.0), false, true);
    assert!(ui.clicks().is_empty());
    assert_eq!(ui.focused(), None);
    assert_eq!(ui.get::<Button>(off).state(), ButtonState::Disabled);
}

#[test]
fn hover_enter_leave_and_wheel_reach_every_control_under_the_pointer() {
    let mut ui = Ui::with_root(
        UiRoot::default()
            .register::<Recorder>()
            .register::<Swatch>(),
    );
    let back = ui.add(Recorder::new(Rect::new(0.0, 0.0, 200.0, 200.0)));
    let front = ui.add(Recorder::new(Rect::new(50.0, 50.0, 50.0, 50.0)));

    ui.mouse(Vec2::new(10.0, 10.0), false, false);
    ui.mouse(Vec2::new(60.0, 60.0), true, false);
    ui.frame(Input {
        mouse_pos: Vec2::new(60.0, 60.0),
        mouse_wheel: Vec2::new(0.0, -1.0),
        ..Input::default()
    });
    ui.mouse(Vec2::new(300.0, 300.0), false, true);

    assert_eq!(
        ui.get::<Recorder>(back).log,
        ["enter", "down Left", "wheel -1", "leave", "up Left"]
    );
    assert_eq!(
        ui.get::<Recorder>(front).log,
        ["enter", "down Left", "wheel -1", "leave", "up Left"],
        "v0.1 does not consume events (spec §9.1)"
    );
}

/// A `Recorder` that is only hit inside the circle its rect encloses.
struct Round(Recorder);

impl Object for Round {
    fn class_name() -> &'static str {
        "Round"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl NodeName for Round {
    fn name(&self) -> &str {
        self.0.name()
    }
    fn set_name(&mut self, name: String) {
        self.0.set_name(name);
    }
}

impl Control for Round {
    fn base(&self) -> &ControlBase {
        self.0.base()
    }
    fn base_mut(&mut self) -> &mut ControlBase {
        self.0.base_mut()
    }
    fn draw_control(&self, ui: &mut UiDrawContext) {
        self.0.draw_control(ui);
    }
    fn hit_test(&self, pos: Vec2) -> bool {
        let r = self.rect();
        let c = Vec2::new(r.x + r.w * 0.5, r.y + r.h * 0.5);
        let d = pos - c;
        d.x * d.x + d.y * d.y <= (r.w * 0.5) * (r.w * 0.5)
    }
    fn on_mouse_enter(&mut self) {
        self.0.on_mouse_enter();
    }
    fn on_mouse_down(&mut self, button: MouseButton, pos: Vec2) {
        self.0.on_mouse_down(button, pos);
    }
}

control_node!(Round);

#[test]
fn hit_test_narrows_the_pointer_area() {
    let mut ui = Ui::with_root(UiRoot::default().register::<Round>());
    let r = ui.add(Round(Recorder::new(Rect::new(0.0, 0.0, 100.0, 100.0))));
    ui.mouse(Vec2::new(5.0, 5.0), true, false); // rect corner, outside the circle
    assert!(ui.get::<Round>(r).0.log.is_empty());
    ui.mouse(Vec2::new(50.0, 50.0), true, false);
    assert_eq!(ui.get::<Round>(r).0.log, ["enter", "down Left"]);
}

#[test]
fn hidden_controls_get_no_mouse_events() {
    let mut ui = Ui::with_root(
        UiRoot::default()
            .register::<Recorder>()
            .register::<Swatch>(),
    );
    let r = ui.add(Recorder::new(Rect::new(0.0, 0.0, 100.0, 100.0)).hidden());
    ui.mouse(Vec2::new(10.0, 10.0), true, false);
    assert!(ui.get::<Recorder>(r).log.is_empty());
}

// --- Draw --------------------------------------------------------------

#[test]
fn button_measures_its_text_plus_padding() {
    let mut ui = Ui::with_root(
        UiRoot::default()
            .register::<Recorder>()
            .register::<Swatch>(),
    );
    let play = ui.button("Play", 10.0, 10.0);
    ui.keys(&[Key::Tab]);
    let button = ui.get::<Button>(play);
    // MockRenderer: width = chars × 0.6 × size, height = size.
    let size = Theme::dark().font_size;
    assert_eq!(
        button.rect(),
        Rect::new(
            10.0,
            10.0,
            4.0 * 0.6 * size + 2.0 * widgets::BUTTON_PADDING_H,
            size + 2.0 * widgets::BUTTON_PADDING_V,
        )
    );
    assert!(button.base().focused);
}
