//! Music playback.
//!
//! The playlist logic is `ember_stdlib::music::MusicPlayer`; this file
//! gives it an engine backend. `EngineMusic` cannot reach the
//! `AudioServer` from inside the player, so `play` / `stop` are queued
//! and the root node applies them each frame with
//! [`EngineMusic::apply`] (ADR-043: each track is a looped clip). Native
//! reads tracks from `assets/`; the web build needs them built in (none
//! yet).

use ember_servers::{AudioClipId, AudioServer, SoundId};
use ember_stdlib::music::{MusicBackend, MusicPlayer as Player, TrackSpec};

use crate::paths;

/// Default playlist. Empty for now — drop files in `assets/` and add
/// entries here, e.g. `TrackSpec::looped("ambient-loop.ogg", 92.0)`.
pub const DEFAULT_TRACKS: &[TrackSpec] = &[];

/// Volume applied to every track (0.0–1.0).
const MUSIC_VOLUME: f32 = 0.4;

pub type MusicPlayer = Player<EngineMusic>;

/// Build the game's music player over [`DEFAULT_TRACKS`].
pub fn new_player() -> MusicPlayer {
    Player::new(EngineMusic::default(), DEFAULT_TRACKS.to_vec())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Request {
    Play(usize),
    Stop,
}

/// A `MusicBackend` on the engine's `AudioServer`.
#[derive(Default)]
pub struct EngineMusic {
    clips: Vec<Option<AudioClipId>>,
    requests: Vec<Request>,
    /// A track asked for while its clip was still decoding (the web).
    waiting: Option<usize>,
    sound: Option<SoundId>,
}

impl EngineMusic {
    /// Load every track of `tracks` as a clip. A track that fails is
    /// skipped with a warning; the player stops when it reaches it.
    pub fn load(&mut self, tracks: &[TrackSpec], audio: &mut dyn AudioServer) {
        self.clips = tracks
            .iter()
            .map(|t| {
                let bytes = std::fs::read(paths::asset(t.path)).ok()?;
                audio
                    .load_clip(&bytes)
                    .map_err(|e| eprintln!("[music] failed to load {}: {e}", t.path))
                    .ok()
            })
            .collect();
    }

    /// Apply the queued plays and stops. Call once per frame.
    pub fn apply(&mut self, audio: &mut dyn AudioServer) {
        for request in std::mem::take(&mut self.requests) {
            if let Some(sound) = self.sound.take() {
                audio.stop(sound);
            }
            self.waiting = match request {
                Request::Play(idx) => Some(idx),
                Request::Stop => None,
            };
        }
        if let Some(idx) = self.waiting
            && let Some(Some(clip)) = self.clips.get(idx).copied()
            && audio.is_clip_ready(clip)
        {
            self.sound = Some(audio.play_looped(clip, MUSIC_VOLUME));
            self.waiting = None;
        }
    }
}

impl MusicBackend for EngineMusic {
    fn play(&mut self, idx: usize, _track: &TrackSpec) -> bool {
        if !matches!(self.clips.get(idx), Some(Some(_))) {
            return false;
        }
        self.requests.push(Request::Play(idx));
        true
    }

    fn stop(&mut self) {
        self.requests.push(Request::Stop);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_player_is_stopped() {
        assert!(!new_player().is_playing());
    }

    #[test]
    fn start_with_empty_playlist_is_noop() {
        let mut p = new_player();
        p.start();
        assert!(!p.is_playing());
    }

    #[test]
    fn tick_with_no_tracks_does_not_panic() {
        let mut p = new_player();
        p.tick(1.0);
        assert!(!p.is_playing());
    }
}
