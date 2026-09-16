//! Minimal audio: load WAV files and play them.
//!
//! Wraps macroquad's audio API. `AudioClip` owns the loaded buffer; `play()`
//! starts a new playback instance. Multiple simultaneous playbacks of the
//! same clip are allowed.
//!
//! Lifetime note: macroquad's `Sound` must outlive any playbacks. We keep
//! the loaded buffer alive for the duration of the game.

use std::path::Path;

use macroquad::audio::{PlaySoundParams, Sound, load_sound, play_sound};

/// A loaded sound. Cheap to play repeatedly.
pub struct AudioClip {
    sound: Sound,
}

impl AudioClip {
    /// Load a WAV file from disk. Async because macroquad's loader is async.
    pub async fn load(path: &Path) -> Result<Self, String> {
        let path_str = path
            .to_str()
            .ok_or_else(|| format!("invalid path: {path:?}"))?;
        let sound = load_sound(path_str)
            .await
            .map_err(|e| format!("load {path:?}: {e}"))?;
        Ok(Self { sound })
    }

    /// Play once at full volume. Non-blocking.
    pub fn play(&self) {
        play_sound(
            &self.sound,
            PlaySoundParams {
                looped: false,
                volume: 1.0,
            },
        );
    }

    /// Play once at a specific volume (0.0 = silent, 1.0 = full).
    pub fn play_at(&self, volume: f32) {
        play_sound(
            &self.sound,
            PlaySoundParams {
                looped: false,
                volume: volume.clamp(0.0, 1.0),
            },
        );
    }
}
