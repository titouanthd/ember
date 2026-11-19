//! Music playback.
//!
//! A thin configuration of `ember_stdlib::music`: the macroquad backend
//! works on native and web. On web, tracks must be preloaded **before**
//! the main loop (`load_sound` performs an HTTP fetch). For formats
//! macroquad cannot decode on native (e.g. MP3), see the soloud /
//! subprocess backend in `jade-garden/src/audio.rs`.

use ember_stdlib::music::{MacroquadMusic, MusicPlayer as Player, TrackSpec};

use crate::paths;

/// Default playlist. Empty for now — drop files in `assets/` and add
/// entries here, e.g. `TrackSpec::looped("ambient-loop.ogg", 92.0)`.
pub const DEFAULT_TRACKS: &[TrackSpec] = &[];

/// Volume applied to every track (0.0–1.0).
const MUSIC_VOLUME: f32 = 0.4;

pub type MusicPlayer = Player<MacroquadMusic>;

/// Build the game's music player over [`DEFAULT_TRACKS`].
pub fn new_player() -> MusicPlayer {
    Player::new(
        MacroquadMusic::new(MUSIC_VOLUME, paths::asset_str),
        DEFAULT_TRACKS.to_vec(),
    )
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
