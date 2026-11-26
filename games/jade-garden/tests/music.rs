//! The playlist on the engine's audio (ADR-043), driven on `MockAudio`.

use ember_servers::{MockAudio, RecordedAudioCall};
use jade_garden::audio::{MusicPlayer, TrackSpec, default_playlist};

/// Short tracks so a test can run the playlist round.
fn short_playlist() -> Vec<TrackSpec> {
    // Real file names, so `load` finds them in `assets/`.
    default_playlist()
        .into_iter()
        .map(|t| TrackSpec { duration: 1.0, ..t })
        .collect()
}

fn loops(audio: &MockAudio) -> Vec<(u64, f32)> {
    audio
        .calls()
        .iter()
        .filter_map(|c| match c {
            RecordedAudioCall::PlayLooped { clip, volume, .. } => Some((clip.raw(), *volume)),
            _ => None,
        })
        .collect()
}

fn loaded(tracks: Vec<TrackSpec>) -> (MusicPlayer, MockAudio) {
    let mut audio = MockAudio::new();
    let mut music = MusicPlayer::new(tracks);
    music.load(&mut audio);
    (music, audio)
}

#[test]
fn every_track_is_loaded_as_a_clip() {
    let (_, audio) = loaded(default_playlist());
    let clips = audio
        .calls()
        .iter()
        .filter(|c| matches!(c, RecordedAudioCall::LoadClip(_)))
        .count();
    assert_eq!(clips, 3);
}

#[test]
fn start_plays_the_intro_on_a_loop_at_music_volume() {
    let (mut music, mut audio) = loaded(default_playlist());
    music.start();
    assert!(music.is_playing());
    music.tick(1.0 / 60.0, &mut audio);
    assert_eq!(loops(&audio), vec![(1, 0.4)]);
    assert_eq!(music.current_track(), Some(0));
}

#[test]
fn the_intro_plays_once_then_the_other_tracks_loop() {
    let (mut music, mut audio) = loaded(short_playlist());
    music.start();
    let mut order = Vec::new();
    for _ in 0..60 {
        music.tick(0.1, &mut audio);
        if order.last() != music.current_track().as_ref() {
            order.extend(music.current_track());
        }
    }
    assert_eq!(&order[..5], &[0, 1, 2, 1, 2]);
    // Each change stops the previous track first.
    let stops = audio
        .calls()
        .iter()
        .filter(|c| matches!(c, RecordedAudioCall::Stop(_)))
        .count();
    assert_eq!(stops, loops(&audio).len() - 1);
}

#[test]
fn a_track_still_decoding_starts_when_ready_and_its_timer_waits() {
    let (mut music, mut audio) = loaded(short_playlist());
    let first = ember_servers::AudioClipId::new(1);
    audio.set_clip_ready(first, false);
    music.start();
    for _ in 0..30 {
        music.tick(0.1, &mut audio);
    }
    // Three seconds went by while decoding: nothing played, no skipping.
    assert!(loops(&audio).is_empty());
    assert_eq!(music.current_track(), None);
    audio.set_clip_ready(first, true);
    music.tick(0.1, &mut audio);
    assert_eq!(loops(&audio), vec![(1, 0.4)]);
    assert_eq!(music.current_track(), Some(0));
}

#[test]
fn stop_silences_the_track() {
    let (mut music, mut audio) = loaded(default_playlist());
    music.start();
    music.tick(0.1, &mut audio);
    music.stop();
    assert!(!music.is_playing());
    music.tick(0.1, &mut audio);
    assert!(matches!(
        audio.calls().last(),
        Some(RecordedAudioCall::Stop(_))
    ));
    assert_eq!(music.current_track(), None);
    // Stopped music no longer advances.
    let before = audio.calls().len();
    for _ in 0..2_000 {
        music.tick(0.1, &mut audio);
    }
    assert_eq!(audio.calls().len(), before);
}
