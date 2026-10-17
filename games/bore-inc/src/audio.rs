//! Music playback.
//!
//! On **wasm**, `macroquad::audio::load_sound` performs an HTTP fetch,
//! so tracks must be preloaded **before** the main loop. On native,
//! the same async API resolves from disk quickly.
//!
//! The starter kit keeps it simple: one `MusicPlayer` built on
//! `macroquad::audio` for both targets. If the native platform needs
//! the `afplay`/`ffplay` subprocess fallback (e.g. MP3 on macOS),
//! copy `jade-garden/src/audio.rs` — it has the hybrid backend.

use macroquad::audio::{load_sound, play_sound, stop_sound, PlaySoundParams, Sound};

use crate::paths;

/// Default playlist. Empty for now — drop files in `assets/` and add
/// entries here.
pub const DEFAULT_TRACKS: &[(&str, f32)] = &[
    // ("ambient-loop.ogg", 92.0),
];

/// Volume applied to every track (0.0–1.0).
const MUSIC_VOLUME: f32 = 0.4;

pub struct MusicPlayer {
    /// One slot per track. `None` if the track failed to load.
    sounds: Vec<Option<Sound>>,
    /// Parallel to `sounds`: duration in seconds.
    durations: Vec<f32>,
    current: usize,
    /// Seconds elapsed on the current track. We track it manually
    /// because `Instant::now()` is unavailable on `wasm32-unknown-unknown`.
    elapsed: f32,
    playing: bool,
}

impl MusicPlayer {
    pub fn new() -> Self {
        Self {
            sounds: Vec::new(),
            durations: Vec::new(),
            current: 0,
            elapsed: 0.0,
            playing: false,
        }
    }

    /// Preload every track. **Must be awaited before `start()` on
    /// web** (it fetches the files). Cheap no-op on native.
    pub async fn preload(&mut self, tracks: &[(&str, f32)]) {
        for (path, duration) in tracks {
            let full = paths::asset_str(path);
            match load_sound(&full).await {
                Ok(snd) => {
                    eprintln!("[audio] loaded {path}");
                    self.sounds.push(Some(snd));
                }
                Err(e) => {
                    eprintln!("[audio] failed to load {path}: {e:?}");
                    self.sounds.push(None);
                }
            }
            self.durations.push(*duration);
        }
    }

    /// Starts the playlist from the first track.
    pub fn start(&mut self) {
        if self.sounds.is_empty() {
            return;
        }
        self.playing = true;
        self.current = 0;
        self.elapsed = 0.0;
        self.play_current();
    }

    pub fn stop(&mut self) {
        if let Some(Some(snd)) = self.sounds.get(self.current) {
            stop_sound(snd);
        }
        self.playing = false;
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    /// Advances the playlist. Call once per frame.
    pub fn tick(&mut self, dt: f32) {
        if !self.playing || self.sounds.is_empty() {
            return;
        }
        self.elapsed += dt;
        let duration = self.durations.get(self.current).copied().unwrap_or(0.0);
        if duration > 0.0 && self.elapsed >= duration {
            self.advance();
        }
    }

    fn advance(&mut self) {
        if self.sounds.is_empty() {
            return;
        }
        if let Some(Some(snd)) = self.sounds.get(self.current) {
            stop_sound(snd);
        }
        self.current = (self.current + 1) % self.sounds.len();
        self.elapsed = 0.0;
        self.play_current();
    }

    fn play_current(&mut self) {
        match self.sounds.get(self.current) {
            Some(Some(snd)) => {
                play_sound(
                    snd,
                    PlaySoundParams {
                        looped: false,
                        volume: MUSIC_VOLUME,
                    },
                );
            }
            _ => {
                eprintln!("[audio] track {} unavailable", self.current);
            }
        }
    }
}

impl Default for MusicPlayer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_player_is_stopped_and_empty() {
        let p = MusicPlayer::new();
        assert!(!p.is_playing());
        assert!(p.sounds.is_empty());
        assert!(p.durations.is_empty());
    }

    #[test]
    fn default_tracks_are_a_slice() {
        // Structural sanity — the constant exists and is a slice.
        let _ = DEFAULT_TRACKS.len();
    }

    #[test]
    fn start_with_empty_playlist_is_noop() {
        let mut p = MusicPlayer::new();
        p.start();
        assert!(!p.is_playing());
    }

    #[test]
    fn tick_with_no_tracks_does_not_panic() {
        let mut p = MusicPlayer::new();
        p.tick(1.0);
        assert!(!p.is_playing());
    }
}