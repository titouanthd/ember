//! Trait contract tests. Every test proves one specific behaviour that
//! `ember_scene` and `ember_ui` will rely on in Phases 6–9.

use ember_core::math::Rect;
use ember_core::{Color, Image};
use ember_servers::*;
use glam::Vec2;

fn frame(renderer: &mut MockRenderer, f: impl FnOnce(&mut MockRenderer)) {
    renderer.begin_frame(Rect::new(0.0, 0.0, 800.0, 600.0));
    f(renderer);
    renderer.end_frame();
    renderer.assert_frame_closed();
}

#[test]
fn draw_calls_are_recorded_in_order() {
    let mut r = MockRenderer::new();
    frame(&mut r, |r| {
        r.draw_rect(Rect::new(0.0, 0.0, 10.0, 10.0), Color::RED);
        r.draw_circle(Vec2::new(5.0, 5.0), 3.0, Color::BLUE);
    });
    let calls = r.calls();
    assert_eq!(calls.len(), 2);
    assert!(matches!(calls[0], RecordedCall::DrawRect { .. }));
    assert!(matches!(calls[1], RecordedCall::DrawCircle { .. }));
}

#[test]
fn z_is_captured_on_each_call() {
    let mut r = MockRenderer::new();
    frame(&mut r, |r| {
        r.set_z(0.0);
        r.draw_rect(Rect::new(0.0, 0.0, 1.0, 1.0), Color::WHITE);
        r.set_z(10.0);
        r.draw_rect(Rect::new(0.0, 0.0, 1.0, 1.0), Color::WHITE);
    });
    let zs: Vec<f32> = r
        .calls()
        .iter()
        .filter_map(|c| match c {
            RecordedCall::DrawRect { z, .. } => Some(*z),
            _ => None,
        })
        .collect();
    assert_eq!(zs, vec![0.0, 10.0]);
}

#[test]
fn triangle_is_recorded() {
    let mut r = MockRenderer::new();
    frame(&mut r, |r| {
        r.draw_triangle(
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(5.0, 10.0),
            Color::GREEN,
        );
    });
    assert!(matches!(r.calls()[0], RecordedCall::DrawTriangle { .. }));
}

#[test]
fn rotated_rect_is_recorded() {
    let mut r = MockRenderer::new();
    frame(&mut r, |r| {
        r.draw_rect_rotated(Rect::new(0.0, 0.0, 10.0, 10.0), 0.5, Color::RED);
    });
    match &r.calls()[0] {
        RecordedCall::DrawRectRotated { rotation, .. } => assert!((rotation - 0.5).abs() < 1e-6),
        other => panic!("expected DrawRectRotated, got {other:?}"),
    }
}

#[test]
fn textures_are_unique_handles() {
    let mut r = MockRenderer::new();
    let img = Image::new(1, 1);
    let a = r.create_texture(&img).unwrap();
    let b = r.create_texture(&img).unwrap();
    assert_ne!(a, b);
}

#[test]
fn fonts_are_unique_handles() {
    let mut r = MockRenderer::new();
    let a = r.create_font(b"fake").unwrap();
    let b = r.create_font(b"fake").unwrap();
    assert_ne!(a, b);
}

#[test]
fn default_font_is_reported_on_draw() {
    let mut r = MockRenderer::new();
    let f = r.create_font(b"fake").unwrap();
    frame(&mut r, |r| {
        r.set_default_font(Some(f));
        r.draw_text_with_font(f, "hi", Vec2::ZERO, 16.0, Color::WHITE);
    });
    assert!(
        r.calls()
            .iter()
            .any(|c| matches!(c, RecordedCall::DrawTextWithFont { font, .. } if *font == f))
    );
}

#[test]
fn measure_text_scales_with_size() {
    let r = MockRenderer::new();
    let small = r.measure_text("abc", 10.0);
    let large = r.measure_text("abc", 20.0);
    assert!(large.width > small.width);
    assert!(large.height > small.height);
}

#[test]
fn capability_probe_defaults_and_grants() {
    let mut r = MockRenderer::new();
    assert!(r.supports(RenderFeature::AlphaBlending));
    assert!(!r.supports(RenderFeature::Instancing));
    r.grant(RenderFeature::Instancing);
    assert!(r.supports(RenderFeature::Instancing));
}

#[test]
#[should_panic(expected = "end_frame called without begin_frame")]
fn end_frame_without_begin_panics() {
    let mut r = MockRenderer::new();
    r.end_frame();
}

#[test]
#[should_panic(expected = "begin_frame called twice without end_frame")]
fn double_begin_frame_panics() {
    let mut r = MockRenderer::new();
    r.begin_frame(Rect::new(0.0, 0.0, 800.0, 600.0));
    r.begin_frame(Rect::new(0.0, 0.0, 800.0, 600.0));
}

#[test]
fn audio_play_returns_unique_sounds() {
    let mut a = MockAudio::new();
    let clip = a.load_clip(b"fake").unwrap();
    let s1 = a.play(clip, 1.0, 1.0);
    let s2 = a.play(clip, 0.5, 1.2);
    assert_ne!(s1, s2);
    assert_eq!(
        a.calls()
            .iter()
            .filter(|c| matches!(c, RecordedAudioCall::Play { .. }))
            .count(),
        2
    );
}

#[test]
fn audio_master_volume_clamps() {
    let mut a = MockAudio::new();
    a.set_master_volume(2.0);
    assert_eq!(a.master_volume(), 1.0);
    a.set_master_volume(-1.0);
    assert_eq!(a.master_volume(), 0.0);
}

#[test]
fn audio_stop_records_the_sound() {
    let mut a = MockAudio::new();
    let clip = a.load_clip(b"fake").unwrap();
    let s = a.play(clip, 1.0, 1.0);
    a.stop(s);
    assert!(
        a.calls()
            .iter()
            .any(|c| matches!(c, RecordedAudioCall::Stop(id) if *id == s))
    );
}

#[test]
fn every_recorded_variant_is_constructible() {
    // Compile-time assertion that each variant ember_scene / ember_ui
    // will match against in Phases 6–9 exists with its expected shape.
    // No assertions on values — the shape is the contract.
    use ember_core::Color;
    use ember_core::math::Rect;
    use ember_servers::*;
    use glam::Vec2;

    let _ = RecordedCall::SetClearColor(Color::WHITE);
    let _ = RecordedCall::SetCamera(Vec2::ZERO);
    let _ = RecordedCall::SetZ(0.0);
    let _ = RecordedCall::SetDefaultFont(None);
    let _ = RecordedCall::DrawRect {
        rect: Rect::default(),
        color: Color::WHITE,
        z: 0.0,
    };
    let _ = RecordedCall::DrawRectOutline {
        rect: Rect::default(),
        thickness: 1.0,
        color: Color::WHITE,
        z: 0.0,
    };
    let _ = RecordedCall::DrawRectRotated {
        rect: Rect::default(),
        rotation: 0.0,
        color: Color::WHITE,
        z: 0.0,
    };
    let _ = RecordedCall::DrawCircle {
        center: Vec2::ZERO,
        radius: 1.0,
        color: Color::WHITE,
        z: 0.0,
    };
    let _ = RecordedCall::DrawCircleOutline {
        center: Vec2::ZERO,
        radius: 1.0,
        thickness: 1.0,
        color: Color::WHITE,
        z: 0.0,
    };
    let _ = RecordedCall::DrawLine {
        a: Vec2::ZERO,
        b: Vec2::ZERO,
        thickness: 1.0,
        color: Color::WHITE,
        z: 0.0,
    };
    let _ = RecordedCall::DrawTriangle {
        a: Vec2::ZERO,
        b: Vec2::ZERO,
        c: Vec2::ZERO,
        color: Color::WHITE,
        z: 0.0,
    };
    let _ = RecordedCall::DrawSprite {
        texture: TextureId::new(0),
        dst: Rect::default(),
        src: None,
        tint: Color::WHITE,
        z: 0.0,
    };
    let _ = RecordedCall::DrawText {
        text: String::new(),
        pos: Vec2::ZERO,
        size: 12.0,
        color: Color::WHITE,
        z: 0.0,
    };
    let _ = RecordedCall::DrawTextWithFont {
        font: FontId::new(0),
        text: String::new(),
        pos: Vec2::ZERO,
        size: 12.0,
        color: Color::WHITE,
        z: 0.0,
    };
    let _ = RecordedCall::CreateTexture(TextureId::new(0));
    let _ = RecordedCall::DestroyTexture(TextureId::new(0));
    let _ = RecordedCall::CreateFont(FontId::new(0));
    let _ = RecordedCall::DestroyFont(FontId::new(0));

    let _ = RecordedAudioCall::Play {
        clip: AudioClipId::new(0),
        volume: 1.0,
        pitch: 1.0,
        sound: SoundId::new(0),
    };
    let _ = RecordedAudioCall::Stop(SoundId::new(0));
    let _ = RecordedAudioCall::StopAll;
    let _ = RecordedAudioCall::SetMasterVolume(1.0);
    let _ = RecordedAudioCall::Flush;
    let _ = RecordedAudioCall::LoadClip(AudioClipId::new(0));
    let _ = RecordedAudioCall::DestroyClip(AudioClipId::new(0));
}
