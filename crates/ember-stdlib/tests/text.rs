//! `ember_stdlib::text` (V1.4): alignment, shadow, bold, fonts, on the
//! mock renderer (text is 0.6 × size wide per character).

use ember_core::Color;
use ember_servers::{FontId, MockRenderer, RecordedCall};
use ember_stdlib::text::{self, Align, BOLD_OFFSET, TextStyle};
use glam::Vec2;

const RED: Color = Color::new(1.0, 0.0, 0.0, 1.0);

/// `(text, pos, color)` of every text call, default font or not.
fn texts(r: &MockRenderer) -> Vec<(String, Vec2, Color)> {
    r.calls()
        .iter()
        .filter_map(|c| match c {
            RecordedCall::DrawText {
                text, pos, color, ..
            }
            | RecordedCall::DrawTextWithFont {
                text, pos, color, ..
            } => Some((text.clone(), *pos, *color)),
            _ => None,
        })
        .collect()
}

#[test]
fn alignment_moves_the_left_edge_by_the_measured_width() {
    // "ABCD" at size 10: 24 wide.
    for (align, x) in [
        (Align::Left, 100.0),
        (Align::Center, 88.0),
        (Align::Right, 76.0),
    ] {
        let mut r = MockRenderer::new();
        text::draw(
            &mut r,
            "ABCD",
            Vec2::new(100.0, 50.0),
            &TextStyle::new(10.0, RED).align(align),
        );
        assert_eq!(
            texts(&r),
            [("ABCD".into(), Vec2::new(x, 50.0), RED)],
            "{align:?}"
        );
    }
}

#[test]
fn shorthands_match_the_style_forms() {
    let mut r = MockRenderer::new();
    text::centered(&mut r, "ABCD", 100.0, 50.0, 10.0, RED);
    text::right(&mut r, "ABCD", 100.0, 60.0, 10.0, RED);
    assert_eq!(
        texts(&r),
        [
            ("ABCD".into(), Vec2::new(88.0, 50.0), RED),
            ("ABCD".into(), Vec2::new(76.0, 60.0), RED),
        ]
    );
}

#[test]
fn the_shadow_is_drawn_first_from_the_same_left_edge() {
    let mut r = MockRenderer::new();
    text::centered_shadowed(&mut r, "ABCD", 100.0, 50.0, 10.0, RED, 2.0);
    assert_eq!(
        texts(&r),
        [
            ("ABCD".into(), Vec2::new(90.0, 52.0), Color::BLACK),
            ("ABCD".into(), Vec2::new(88.0, 50.0), RED),
        ]
    );
}

#[test]
fn bold_draws_twice_and_measures_wider() {
    let style = TextStyle::new(10.0, RED).bold();
    assert_eq!(style.bold, BOLD_OFFSET);
    let r = MockRenderer::new();
    assert_eq!(text::measure(&r, "ABCD", &style).width, 24.0 + BOLD_OFFSET);
    assert_eq!(text::measure(&r, "", &style).width, 0.0);

    let mut r = MockRenderer::new();
    text::draw(&mut r, "AB", Vec2::new(10.0, 20.0), &style);
    assert_eq!(
        texts(&r),
        [
            ("AB".into(), Vec2::new(10.0, 20.0), RED),
            ("AB".into(), Vec2::new(10.0 + BOLD_OFFSET, 20.0), RED),
        ]
    );
    assert_eq!(TextStyle::new(1.0, RED).bold_by(-3.0).bold, 0.0, "clamped");
}

#[test]
fn a_bold_shadowed_title_draws_four_passes() {
    let style = TextStyle::new(10.0, RED)
        .shadow(Vec2::new(1.0, 3.0), Color::BLACK)
        .bold_by(1.0);
    let mut r = MockRenderer::new();
    text::draw(&mut r, "A", Vec2::new(0.0, 0.0), &style);
    let got: Vec<(Vec2, Color)> = texts(&r).into_iter().map(|(_, p, c)| (p, c)).collect();
    assert_eq!(
        got,
        [
            (Vec2::new(1.0, 3.0), Color::BLACK),
            (Vec2::new(2.0, 3.0), Color::BLACK),
            (Vec2::new(0.0, 0.0), RED),
            (Vec2::new(1.0, 0.0), RED),
        ]
    );
}

#[test]
fn a_font_goes_through_the_with_font_calls() {
    let font = FontId::new(7);
    let mut r = MockRenderer::new();
    text::draw(
        &mut r,
        "ABCD",
        Vec2::new(100.0, 0.0),
        &TextStyle::new(10.0, RED).font(font).right(),
    );
    assert!(matches!(
        r.calls(),
        [RecordedCall::DrawTextWithFont { font: f, pos, .. }] if *f == font && *pos == Vec2::new(76.0, 0.0)
    ));
}

#[test]
fn empty_text_draws_nothing() {
    let mut r = MockRenderer::new();
    text::centered_shadowed(&mut r, "", 0.0, 0.0, 10.0, RED, 2.0);
    assert!(texts(&r).is_empty());
}

#[test]
fn left_edge_and_color_builder() {
    let r = MockRenderer::new();
    let s = TextStyle::new(10.0, RED).centered().color(Color::WHITE);
    assert_eq!(s.color, Color::WHITE);
    assert_eq!(text::left_edge(&r, "ABCD", 100.0, &s), 88.0);
}
