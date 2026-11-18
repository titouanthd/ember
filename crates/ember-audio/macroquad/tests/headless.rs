//! Audio tests. `#[ignore]`'d — need a window for macroquad's audio
//! context. Run with `xvfb-run cargo test -p ember-audio-macroquad -- --ignored`.

use ember_audio_macroquad::MacroquadAudio;
use ember_servers::{AudioFeature, AudioServer, SoundId};

#[test]
#[ignore = "requires a macroquad window"]
fn master_volume_clamps() {
    let mut a = MacroquadAudio::new();
    a.set_master_volume(2.0);
    assert_eq!(a.master_volume(), 1.0);
    a.set_master_volume(-1.0);
    assert_eq!(a.master_volume(), 0.0);
}

#[test]
#[ignore = "requires a macroquad window"]
fn play_on_unknown_clip_returns_a_sound_id() {
    let mut a = MacroquadAudio::new();
    let fake = ember_servers::AudioClipId::new(999);
    let s = a.play(fake, 1.0, 1.0);
    assert_eq!(s.raw(), 1);
}

#[test]
#[ignore = "requires a macroquad window"]
fn stop_on_unknown_sound_is_noop() {
    let mut a = MacroquadAudio::new();
    a.stop(SoundId::new(999));
}

#[test]
#[ignore = "requires a macroquad window"]
fn stop_all_and_flush_do_not_panic_on_empty_bank() {
    let mut a = MacroquadAudio::new();
    a.stop_all();
    a.flush();
}

#[test]
#[ignore = "requires a macroquad window"]
fn pitch_shift_is_not_advertised() {
    let a = MacroquadAudio::new();
    assert!(!a.supports(AudioFeature::PitchShift));
}
