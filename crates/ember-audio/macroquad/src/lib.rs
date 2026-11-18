//! Reference `AudioServer` implementation on macroquad.
//!
//! See `crates/ember-audio/macroquad/src/audio.rs` for behaviour notes.

mod audio;

pub use audio::MacroquadAudio;
pub use ember_servers as servers;

pub use macroquad::audio as mq_audio;
