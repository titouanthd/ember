//! Headless render tests. Require a window (or `xvfb-run`).
//!
//! These tests do not assert on pixels — that's v0.2's visual
//! regression suite. They assert that every `RenderingServer` method
//! runs without panicking and produces sane handle state.

use ember_core::math::Rect;
use ember_core::{Color, Image};
use ember_render_macroquad::MacroquadRenderer;
use ember_servers::{RenderFeature, RenderingServer};
use glam::Vec2;

#[test]
#[ignore = "requires a macroquad window (run with --ignored under xvfb-run)"]
fn full_frame_cycle_does_not_panic() {
    let mut r = MacroquadRenderer::new();

    let tex = {
        let mut img = Image::new(2, 2);
        img.set_pixel(0, 0, Color::WHITE);
        img.set_pixel(1, 0, Color::RED);
        img.set_pixel(0, 1, Color::GREEN);
        img.set_pixel(1, 1, Color::BLUE);
        r.create_texture(&img).unwrap()
    };

    r.begin_frame(Rect::new(0.0, 0.0, 100.0, 100.0));
    r.set_clear_color(Color::BLACK);
    r.set_camera(Vec2::new(10.0, 20.0));
    r.set_z(1.0);
    r.draw_rect(Rect::new(0.0, 0.0, 10.0, 10.0), Color::RED);
    r.draw_rect_outline(Rect::new(0.0, 0.0, 10.0, 10.0), 2.0, Color::WHITE);
    r.draw_rect_rotated(Rect::new(0.0, 0.0, 10.0, 10.0), 0.5, Color::GREEN);
    r.draw_circle(Vec2::new(5.0, 5.0), 3.0, Color::BLUE);
    r.draw_circle_outline(Vec2::new(5.0, 5.0), 3.0, 1.0, Color::WHITE);
    r.draw_line(Vec2::ZERO, Vec2::new(10.0, 10.0), 1.0, Color::WHITE);
    r.draw_triangle(
        Vec2::ZERO,
        Vec2::new(1.0, 0.0),
        Vec2::new(0.0, 1.0),
        Color::WHITE,
    );
    r.draw_sprite(tex, Rect::new(0.0, 0.0, 20.0, 20.0), None, Color::WHITE);
    r.draw_text("hello", Vec2::new(5.0, 5.0), 16.0, Color::WHITE);
    r.end_frame();

    r.destroy_texture(tex);
}

#[test]
#[ignore = "requires a macroquad window"]
fn create_texture_returns_unique_handles() {
    let mut r = MacroquadRenderer::new();
    let img = Image::new(4, 4);
    let a = r.create_texture(&img).unwrap();
    let b = r.create_texture(&img).unwrap();
    assert_ne!(a, b);
}

#[test]
#[ignore = "requires a macroquad window"]
fn empty_image_is_rejected() {
    let mut r = MacroquadRenderer::new();
    let img = Image::new(0, 0);
    assert!(r.create_texture(&img).is_err());
}

#[test]
#[ignore = "requires a macroquad window"]
fn draw_with_unknown_texture_is_a_noop() {
    let mut r = MacroquadRenderer::new();
    r.begin_frame(Rect::new(0.0, 0.0, 100.0, 100.0));
    let fake = ember_servers::TextureId::new(999);
    r.draw_sprite(fake, Rect::new(0.0, 0.0, 10.0, 10.0), None, Color::WHITE);
    r.end_frame();
}

#[test]
#[ignore = "requires a macroquad window"]
fn measure_text_returns_positive_size_for_nonempty() {
    let r = MacroquadRenderer::new();
    let m = r.measure_text("hello", 16.0);
    assert!(m.width > 0.0);
    assert!(m.height > 0.0);
}

#[test]
#[ignore = "requires a macroquad window"]
fn capabilities_are_honest() {
    let r = MacroquadRenderer::new();
    assert!(r.supports(RenderFeature::AlphaBlending));
    assert!(r.supports(RenderFeature::CustomShaders));
    assert!(!r.supports(RenderFeature::TextureArray));
    assert!(!r.supports(RenderFeature::Instancing));
}
