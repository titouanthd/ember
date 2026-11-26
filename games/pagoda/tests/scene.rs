//! Headless run of the node-based Pagoda on mock backends.

use ember::prelude::*;
use ember_input::{TouchPhase, TouchPoint};
use ember_servers::{Camera2d, MockAudio, MockRenderer, RecordedAudioCall, RecordedCall};
use pagoda::AppState;
use pagoda::persistence::BestScore;
use pagoda::scene::{Pagoda, build_tree};
use pagoda::ui::MuteButton;
use std::path::PathBuf;

type Game = SceneLoop<MockRenderer, MockAudio>;

/// A best-score file of its own, so tests never touch the real one.
fn best_file(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pagoda-scene-{}-{test}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("best_score.ron");
    let _ = std::fs::remove_file(&path);
    path
}

fn game(test: &str) -> Game {
    let mut l = SceneLoop::new(
        build_tree(BestScore::new(best_file(test))),
        MockRenderer::new(),
        MockAudio::new(),
        Rect::new(0.0, 0.0, 800.0, 600.0),
    )
    .with_virtual_resolution(800.0, 600.0);
    assert!(l.init());
    l
}

fn step(l: &mut Game, input: Input) {
    l.set_input(input);
    l.process(1.0 / 60.0);
    l.set_input(Input::default());
}

fn press(l: &mut Game, key: Key) {
    step(
        l,
        Input {
            keys_pressed: vec![key],
            keys_down: vec![key],
            ..Input::default()
        },
    );
}

fn click_at(l: &mut Game, pos: Vec2) {
    step(
        l,
        Input {
            mouse_pos: pos,
            mouse_left_pressed: true,
            mouse_left_down: true,
            ..Input::default()
        },
    );
}

fn pagoda(l: &mut Game) -> &mut Pagoda {
    let root = l.tree.root().unwrap();
    l.tree
        .get_mut(root)
        .unwrap()
        .as_any_mut()
        .downcast_mut::<Pagoda>()
        .unwrap()
}

fn plays(l: &Game) -> usize {
    l.audio()
        .calls()
        .iter()
        .filter(|c| matches!(c, RecordedAudioCall::Play { .. }))
        .count()
}

/// Line the sliding block up with the top of the tower.
fn align(l: &mut Game) {
    let p = pagoda(l);
    let top = p.match_state.tower.last().unwrap().transform.position.x;
    p.match_state.sliding.x = top;
}

#[test]
fn ready_loads_both_fonts_and_the_sound_bank() {
    let mut l = game("ready");
    let fonts = l
        .rendering()
        .calls()
        .iter()
        .filter(|c| matches!(c, RecordedCall::CreateFont(_)))
        .count();
    assert_eq!(fonts, 2, "DejaVu + the CJK subset");
    assert!(
        l.rendering()
            .calls()
            .iter()
            .any(|c| matches!(c, RecordedCall::SetDefaultFont(Some(_))))
    );
    let clips = l
        .audio()
        .calls()
        .iter()
        .filter(|c| matches!(c, RecordedAudioCall::LoadClip(_)))
        .count();
    assert_eq!(clips, 11);
    assert!(pagoda(&mut l).sounds.is_some());
}

#[test]
fn a_click_starts_with_a_chime_and_a_drop_lands() {
    let mut l = game("start");
    click_at(&mut l, Vec2::new(400.0, 400.0));
    assert_eq!(pagoda(&mut l).app.state, AppState::Playing);
    assert_eq!(plays(&l), 1, "start chime");

    align(&mut l);
    press(&mut l, Key::Space);
    let p = pagoda(&mut l);
    assert_eq!(p.match_state.tower.len(), 2);
    assert_eq!(p.app.score, 1);
    assert!(plays(&l) >= 2, "landing sounds");
}

#[test]
fn the_speaker_mutes_and_swallows_the_click() {
    let mut l = game("mute");
    let speaker = MuteButton::default().center;
    click_at(&mut l, speaker);
    assert!(pagoda(&mut l).is_muted());
    assert_eq!(pagoda(&mut l).app.state, AppState::MainMenu, "no start");
    assert!(l.audio().calls().contains(&RecordedAudioCall::StopAll));

    // Muted: starting plays nothing.
    click_at(&mut l, Vec2::new(400.0, 400.0));
    assert_eq!(pagoda(&mut l).app.state, AppState::Playing);
    assert_eq!(plays(&l), 0);
}

#[test]
fn a_tap_drops_but_a_tap_on_the_speaker_mutes() {
    let mut l = game("touch");
    let tap = |pos| Input {
        touch_points: vec![TouchPoint::new(1, pos, TouchPhase::Started)],
        ..Input::default()
    };
    step(&mut l, tap(MuteButton::default().center));
    assert!(pagoda(&mut l).is_muted());
    assert_eq!(pagoda(&mut l).app.state, AppState::MainMenu);

    step(&mut l, tap(Vec2::new(400.0, 400.0)));
    assert_eq!(pagoda(&mut l).app.state, AppState::Playing);
}

#[test]
fn a_finished_run_saves_its_new_best() {
    let mut l = game("best");
    click_at(&mut l, Vec2::new(400.0, 400.0));
    align(&mut l);
    press(&mut l, Key::Space);
    // Miss: the sliding block entirely off the tower.
    pagoda(&mut l).match_state.sliding.x = 0.0;
    press(&mut l, Key::Space);

    let p = pagoda(&mut l);
    assert_eq!(p.app.state, AppState::GameOver);
    assert_eq!(p.app.high_score, 1);
    assert_eq!(p.best_score().load(), Some(1));
}

#[test]
fn draws_on_the_letterboxed_canvas_with_the_punch_zoom() {
    let mut l = game("draw");
    l.resize(1600.0, 600.0);
    click_at(&mut l, Vec2::new(400.0, 400.0));
    pagoda(&mut l).match_state.camera_punch = 1.0;
    l.rendering_mut().clear_calls();
    l.draw();
    l.rendering().assert_frame_closed();

    let letterbox = Camera2d::letterbox(Vec2::new(1600.0, 600.0), Vec2::new(800.0, 600.0));
    let punched = letterbox.zoomed_about(Vec2::new(400.0, 300.0), 1.03);
    let calls = l.rendering().calls();
    assert!(calls.contains(&RecordedCall::SetCamera(punched)));
    assert!(
        calls.contains(&RecordedCall::SetCamera(letterbox)),
        "restored"
    );
    // The background, the tower and the speaker are all drawn.
    let rects = calls
        .iter()
        .filter(|c| matches!(c, RecordedCall::DrawRect { .. }))
        .count();
    assert!(rects > 50, "{rects} rects");
    assert!(
        calls
            .iter()
            .any(|c| matches!(c, RecordedCall::DrawArc { .. })),
        "speaker waves"
    );
}
