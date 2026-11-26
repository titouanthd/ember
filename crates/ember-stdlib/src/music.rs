//! Background-music playlist.
//!
//! [`MusicPlayer`] owns the playlist logic (intro tracks, looping, timed
//! advance) and delegates actual playback to a [`MusicBackend`]. Games pick
//! the backend: [`MacroquadMusic`] works everywhere macroquad does (and is
//! the only option on wasm); a game can plug a native one (e.g. soloud or an
//! OS player subprocess) for formats macroquad cannot decode.
//!
//! Tracks are advanced by duration, not by "playback ended" callbacks,
//! because neither macroquad nor subprocess players expose one.

use macroquad::audio::{PlaySoundParams, Sound, load_sound, play_sound, stop_sound};

/// One entry in a playlist.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrackSpec {
    /// Path relative to the game's assets directory.
    pub path: &'static str,
    /// Duration in seconds. `0.0` means "never auto-advance".
    pub duration: f32,
    /// `true` → plays once, then never again. `false` → part of the loop.
    pub is_intro: bool,
}

impl TrackSpec {
    /// A looping-playlist track.
    pub const fn looped(path: &'static str, duration: f32) -> Self {
        Self {
            path,
            duration,
            is_intro: false,
        }
    }

    /// A track played once at the start.
    pub const fn intro(path: &'static str, duration: f32) -> Self {
        Self {
            path,
            duration,
            is_intro: true,
        }
    }
}

/// Playback strategy used by [`MusicPlayer`].
#[allow(async_fn_in_trait)] // single-threaded: games never `Send` a backend
pub trait MusicBackend {
    /// Load tracks ahead of time. Required before `play` on web, where
    /// loading is an HTTP fetch; a no-op by default.
    async fn preload(&mut self, _tracks: &[TrackSpec]) {}

    /// Stop the current track and start track `idx`. Returns `false` if
    /// it could not be played.
    fn play(&mut self, idx: usize, track: &TrackSpec) -> bool;

    /// Stop whatever is playing.
    fn stop(&mut self);

    /// Seconds since the current track started, if the backend keeps its
    /// own clock. `None` makes the player accumulate `dt` itself — needed
    /// on wasm, where `std::time::Instant` is unavailable.
    fn elapsed(&self) -> Option<f32> {
        None
    }
}

/// Playlist driver. Call [`tick`](Self::tick) once per frame.
pub struct MusicPlayer<B: MusicBackend> {
    backend: B,
    tracks: Vec<TrackSpec>,
    current: usize,
    elapsed: f32,
    playing: bool,
}

impl<B: MusicBackend> MusicPlayer<B> {
    pub fn new(backend: B, tracks: Vec<TrackSpec>) -> Self {
        Self {
            backend,
            tracks,
            current: 0,
            elapsed: 0.0,
            playing: false,
        }
    }

    /// Preload every track. **Must be awaited before [`start`](Self::start)
    /// on web.** Cheap no-op for backends that load lazily.
    pub async fn preload(&mut self) {
        self.backend.preload(&self.tracks).await;
    }

    /// Start the playlist from the first track. No-op on an empty playlist.
    pub fn start(&mut self) {
        if self.tracks.is_empty() {
            return;
        }
        self.current = 0;
        self.playing = true;
        self.play_current();
    }

    pub fn stop(&mut self) {
        self.backend.stop();
        self.playing = false;
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    /// The backend, for a backend that needs per-frame work outside the
    /// player (an engine-audio backend applying queued plays).
    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }

    /// Advance the playlist clock by `dt` seconds.
    pub fn tick(&mut self, dt: f32) {
        if !self.playing || self.tracks.is_empty() {
            return;
        }
        self.elapsed += dt;
        let elapsed = self.backend.elapsed().unwrap_or(self.elapsed);
        let duration = self.tracks[self.current].duration;
        if duration > 0.0 && elapsed >= duration {
            self.advance();
        }
    }

    fn advance(&mut self) {
        let loop_start = self.tracks.iter().position(|t| !t.is_intro).unwrap_or(0);
        self.current = if self.current + 1 < self.tracks.len() {
            self.current + 1
        } else {
            loop_start
        };
        self.play_current();
    }

    fn play_current(&mut self) {
        self.elapsed = 0.0;
        let track = self.tracks[self.current];
        if !self.backend.play(self.current, &track) {
            eprintln!("[music] track {} unavailable: {}", self.current, track.path);
            self.playing = false;
        }
    }
}

/// [`MusicBackend`] built on `macroquad::audio` (WebAudio on wasm).
pub struct MacroquadMusic {
    sounds: Vec<Option<Sound>>,
    current: Option<usize>,
    volume: f32,
    resolve: fn(&str) -> String,
}

impl MacroquadMusic {
    /// `resolve` maps a track's relative path to what `load_sound` expects
    /// (typically the game's `paths::asset_str`).
    pub fn new(volume: f32, resolve: fn(&str) -> String) -> Self {
        Self {
            sounds: Vec::new(),
            current: None,
            volume: volume.clamp(0.0, 1.0),
            resolve,
        }
    }
}

impl MusicBackend for MacroquadMusic {
    async fn preload(&mut self, tracks: &[TrackSpec]) {
        self.sounds.clear();
        for track in tracks {
            let full = (self.resolve)(track.path);
            match load_sound(&full).await {
                Ok(snd) => self.sounds.push(Some(snd)),
                Err(e) => {
                    eprintln!("[music] failed to load {}: {e:?}", track.path);
                    self.sounds.push(None);
                }
            }
        }
    }

    fn play(&mut self, idx: usize, _track: &TrackSpec) -> bool {
        self.stop();
        match self.sounds.get(idx) {
            Some(Some(snd)) => {
                play_sound(
                    snd,
                    PlaySoundParams {
                        looped: true,
                        volume: self.volume,
                    },
                );
                self.current = Some(idx);
                true
            }
            _ => false,
        }
    }

    fn stop(&mut self) {
        if let Some(idx) = self.current.take()
            && let Some(Some(snd)) = self.sounds.get(idx)
        {
            stop_sound(snd);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Recorder {
        played: Vec<usize>,
        stops: usize,
        fail: bool,
    }

    impl MusicBackend for Recorder {
        fn play(&mut self, idx: usize, _track: &TrackSpec) -> bool {
            self.played.push(idx);
            !self.fail
        }
        fn stop(&mut self) {
            self.stops += 1;
        }
    }

    fn playlist() -> Vec<TrackSpec> {
        vec![
            TrackSpec::intro("intro", 2.0),
            TrackSpec::looped("a", 3.0),
            TrackSpec::looped("b", 1.0),
        ]
    }

    #[test]
    fn empty_playlist_is_inert() {
        let mut p = MusicPlayer::new(Recorder::default(), Vec::new());
        p.start();
        p.tick(10.0);
        assert!(!p.is_playing());
        assert!(p.backend.played.is_empty());
    }

    #[test]
    fn start_plays_first_track() {
        let mut p = MusicPlayer::new(Recorder::default(), playlist());
        p.start();
        assert!(p.is_playing());
        assert_eq!(p.backend.played, [0]);
    }

    #[test]
    fn advances_when_duration_elapses() {
        let mut p = MusicPlayer::new(Recorder::default(), playlist());
        p.start();
        p.tick(1.9);
        assert_eq!(p.backend.played, [0]);
        p.tick(0.2);
        assert_eq!(p.backend.played, [0, 1]);
    }

    #[test]
    fn loop_skips_the_intro() {
        let mut p = MusicPlayer::new(Recorder::default(), playlist());
        p.start();
        for _ in 0..4 {
            p.tick(3.0);
        }
        // intro, a, b, then back to a (not the intro), then b.
        assert_eq!(p.backend.played, [0, 1, 2, 1, 2]);
    }

    #[test]
    fn all_loop_playlist_wraps_to_start() {
        let mut p = MusicPlayer::new(
            Recorder::default(),
            vec![TrackSpec::looped("a", 1.0), TrackSpec::looped("b", 1.0)],
        );
        p.start();
        p.tick(1.0);
        p.tick(1.0);
        assert_eq!(p.backend.played, [0, 1, 0]);
    }

    #[test]
    fn zero_duration_never_advances() {
        let mut p = MusicPlayer::new(Recorder::default(), vec![TrackSpec::looped("a", 0.0)]);
        p.start();
        p.tick(100.0);
        assert_eq!(p.backend.played, [0]);
    }

    #[test]
    fn failed_play_stops_the_player() {
        let backend = Recorder {
            fail: true,
            ..Default::default()
        };
        let mut p = MusicPlayer::new(backend, playlist());
        p.start();
        assert!(!p.is_playing());
    }

    #[test]
    fn stop_halts_the_clock() {
        let mut p = MusicPlayer::new(Recorder::default(), playlist());
        p.start();
        p.stop();
        p.tick(10.0);
        assert!(!p.is_playing());
        assert_eq!(p.backend.stops, 1);
        assert_eq!(p.backend.played, [0]);
    }
}
