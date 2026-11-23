//! Phase 5.2: Label, Panel and Image drawing; built-in registration.

use ember_core::Color;
use ember_core::math::Rect;
use ember_scene::{DrawContext, Node, NodeName};
use ember_servers::{MockRenderer, RecordedCall, RenderingServer, TextureId};
use ember_ui::prelude::*;
use glam::Vec2;
use std::sync::Arc;

const SCREEN: Rect = Rect::new(0.0, 0.0, 800.0, 600.0);

/// Draw one node into a fresh `MockRenderer` and return its calls.
fn draw(node: &dyn Node) -> Vec<RecordedCall> {
    let mut r = MockRenderer::new();
    r.begin_frame(SCREEN);
    node.draw(&mut DrawContext::new(&mut r));
    r.end_frame();
    r.take_calls()
}

/// Give a control a rect and a theme, as `UiRoot` would during layout.
fn placed<C: Control>(mut c: C, rect: Rect, theme: Theme) -> C {
    c.set_rect(rect);
    c.base_mut().theme = Arc::new(theme);
    c
}

#[test]
fn label_measures_its_text_with_the_theme_size() {
    let r = MockRenderer::new(); // 0.6 * size per char, height = size
    let theme = Theme::dark(); // font_size 20
    let m = Measure::new(&r, &theme);
    assert_eq!(
        Label::new("Play").measure(SCREEN.size(), &m),
        Vec2::new(48.0, 20.0)
    );
    assert_eq!(
        Label::new("Play")
            .with_size(10.0)
            .measure(SCREEN.size(), &m),
        Vec2::new(24.0, 10.0)
    );
}

#[test]
fn label_draws_centred_text_on_its_baseline_in_the_theme_colour() {
    let label = placed(
        Label::new("Play").with_align(Alignment::Center),
        Rect::new(0.0, 0.0, 100.0, 40.0),
        Theme::light(),
    );
    // text box 48 x 20, centred: top-left (26, 10); baseline = top + offset_y (20)
    assert_eq!(
        draw(&label),
        vec![RecordedCall::DrawText {
            text: "Play".into(),
            pos: Vec2::new(26.0, 30.0),
            size: 20.0,
            color: Theme::light().text,
            z: 0.0,
        }]
    );
}

#[test]
fn an_empty_label_draws_nothing() {
    let label = placed(Label::new(""), SCREEN, Theme::dark());
    assert!(draw(&label).is_empty());
}

#[test]
fn panel_draws_background_then_border_from_the_theme() {
    let rect = Rect::new(10.0, 10.0, 200.0, 100.0);
    let panel = placed(Panel::new(), rect, Theme::dark());
    assert_eq!(
        draw(&panel),
        vec![
            RecordedCall::DrawRect {
                rect,
                color: Theme::dark().panel_bg,
                z: 0.0,
            },
            RecordedCall::DrawRectOutline {
                rect,
                thickness: 1.0,
                color: Theme::dark().panel_border,
                z: 0.0,
            },
        ]
    );
}

#[test]
fn panel_overrides_and_no_border() {
    let panel = placed(
        Panel::new()
            .with_background(Color::BLUE)
            .with_border(0.0, None),
        SCREEN,
        Theme::dark(),
    );
    assert_eq!(
        draw(&panel),
        vec![RecordedCall::DrawRect {
            rect: SCREEN,
            color: Color::BLUE,
            z: 0.0,
        }]
    );
}

#[test]
fn image_draws_the_texture_letterboxed() {
    let tex = TextureId::new(7);
    let image = placed(
        Image::new(tex, Vec2::new(200.0, 100.0)),
        Rect::new(0.0, 0.0, 100.0, 100.0),
        Theme::dark(),
    );
    assert_eq!(
        draw(&image),
        vec![RecordedCall::DrawSprite {
            texture: tex,
            dst: Rect::new(0.0, 25.0, 100.0, 50.0),
            src: None,
            tint: Color::WHITE,
            z: 0.0,
        }]
    );
}

#[test]
fn widgets_sit_on_the_ui_layer() {
    assert_eq!(Label::new("x").z(), UI_Z);
    assert_eq!(Panel::new().z(), UI_Z);
    assert_eq!(VBox::new().z(), UI_Z);
    assert!(!Label::new("x").hidden().visible());
}

#[test]
fn ui_root_registers_every_builtin() {
    let root = UiRoot::default();
    let reg = root.registry();
    assert!(reg.is_registered::<Label>());
    assert!(reg.is_registered::<Button>());
    assert!(reg.is_registered::<Panel>());
    assert!(reg.is_registered::<Image>());
    assert!(reg.is_registered::<VBox>());
    assert!(reg.is_registered::<HBox>());
    assert!(reg.is_registered::<Stretch>());
    assert!(reg.is_registered::<TextInput>());
    assert!(reg.is_registered::<ScrollContainer>());
    assert_eq!(reg.len(), 9);
}

#[test]
fn builders_set_names_and_anchors() {
    let l = Label::new("Score")
        .with_name("ScoreLabel")
        .with_anchors(Anchors::TOP_RIGHT)
        .with_offset(Vec2::new(-10.0, 10.0));
    assert_eq!(l.name(), "ScoreLabel");
    assert_eq!(l.anchors(), Anchors::TOP_RIGHT);
    assert_eq!(l.base().offset, Vec2::new(-10.0, 10.0));
}
