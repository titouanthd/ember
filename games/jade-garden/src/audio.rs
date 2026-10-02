//! Background music: hybrid player.
//!
//! **Native** (macOS, Linux, Windows):
//! 1. **soloud** (bundled, dlopens the system audio library).
//! 2. **OS-native player subprocess** (`afplay`, `ffplay`, `mpv`).
//!
//! **Web** (WASM):
//! Uses `macroquad::audio` (backed by quad-snd → WebAudio).
//! Tracks are preloaded asynchronously before the main loop via
//! [`MusicPlayer::preload`] — required because on web
//! `macroquad::audio::load_sound` fetches the file. Playback is then
//! synchronous (`play_sound` / `stop_sound`).
//!
//! No `wasm-bindgen`-based crates are used: miniquad (via macroquad)
//! has its own import system, incompatible with wasm-bindgen's
//! `__wbindgen_placeholder__` imports.
//!
//! Time tracking: `std::time::Instant` is unavailable on
//! `wasm32-unknown-unknown`, so the wasm backend accumulates `dt` in a
//! plain `f32` (fed by `MusicPlayer::tick`). The native backend keeps
//! using `Instant`.

#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

// ─── Shared ────────────────────────────────────────────────────────

/// Volume applied to every track (0.0–1.0).
const MUSIC_VOLUME: f32 = 0.4;

/// One entry in the background music playlist.
pub struct TrackSpec {
    /// Path relative to the assets directory (e.g. `foo.mp3`).
    pub path: &'static str,
    /// Duration in seconds.
    pub duration: f32,
    /// `true` → plays once, then advances. `false` → part of the loop.
    pub is_intro: bool,
}

// ─── Native backend ────────────────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
mod backend {
    use super::*;
    use std::process::{Child, Command, Stdio};
    use std::time::Instant;
    use soloud::*;

    enum Backend {
        Soloud {
            soloud: Soloud,
            loaded: Option<audio::Wav>,
        },
        Subprocess {
            player: &'static str,
            child: Option<Child>,
        },
    }

    pub struct MusicBackend {
        inner: Backend,
        started_at: Instant,
        playing: bool,
    }

    impl MusicBackend {
        pub fn new() -> Self {
            match Soloud::default() {
                Ok(soloud) => {
                    eprintln!("🎵 Music backend: soloud (native)");
                    Self {
                        inner: Backend::Soloud { soloud, loaded: None },
                        started_at: Instant::now(),
                        playing: false,
                    }
                }
                Err(e) => {
                    eprintln!("⚠️  soloud unavailable ({e:?}), trying OS player...");
                    match detect_player() {
                        Some(player) => {
                            eprintln!("🎵 Music backend: {player} (subprocess)");
                            Self {
                                inner: Backend::Subprocess { player, child: None },
                                started_at: Instant::now(),
                                playing: false,
                            }
                        }
                        None => {
                            eprintln!("⚠️  No audio backend available. Music disabled.");
                            Self {
                                inner: Backend::Subprocess { player: "", child: None },
                                started_at: Instant::now(),
                                playing: false,
                            }
                        }
                    }
                }
            }
        }

        /// No-op on native: loading is lazy, done by `play`.
        pub async fn preload(&mut self, _specs: &[TrackSpec]) {}

        pub fn play(&mut self, path: &PathBuf) -> bool {
            match &mut self.inner {
                Backend::Soloud { soloud, loaded } => {
                    soloud.stop_all();
                    *loaded = None;
                    let mut wav = audio::Wav::default();
                    if let Err(e) = wav.load(path) {
                        eprintln!("⚠️  soloud: failed to load {path:?}: {e:?}");
                        return false;
                    }
                    soloud.set_global_volume(MUSIC_VOLUME);
                    soloud.play(&wav);
                    *loaded = Some(wav);
                    self.started_at = Instant::now();
                    true
                }
                Backend::Subprocess { player, child } => {
                    if let Some(mut c) = child.take() {
                        let _ = c.kill();
                        let _ = c.wait();
                    }
                    match spawn_player(player, path) {
                        Some(c) => {
                            *child = Some(c);
                            self.started_at = Instant::now();
                            true
                        }
                        None => {
                            eprintln!("⚠️  {player} failed to start for {path:?}");
                            false
                        }
                    }
                }
            }
        }

        pub fn stop(&mut self) {
            match &mut self.inner {
                Backend::Soloud { soloud, loaded } => {
                    soloud.stop_all();
                    *loaded = None;
                }
                Backend::Subprocess { child, .. } => {
                    if let Some(mut c) = child.take() {
                        let _ = c.kill();
                        let _ = c.wait();
                    }
                }
            }
        }

        /// Native uses `Instant` internally; `dt` is ignored.
        pub fn advance_elapsed(&mut self, _dt: f32) {}

        /// Seconds elapsed since the current track started.
        pub fn elapsed(&self) -> f32 {
            self.started_at.elapsed().as_secs_f32()
        }

        pub fn set_playing(&mut self, v: bool) {
            self.playing = v;
        }

        pub fn is_playing(&self) -> bool {
            self.playing
        }
    }

    impl Drop for MusicBackend {
        fn drop(&mut self) {
            if let Backend::Subprocess { child, .. } = &mut self.inner
                && let Some(mut c) = child.take()
            {
                let _ = c.kill();
                let _ = c.wait();
            }
        }
    }

    pub fn detect_player() -> Option<&'static str> {
        #[cfg(target_os = "macos")]
        const CANDIDATES: &[&str] = &["afplay", "ffplay", "mpv"];
        #[cfg(not(target_os = "macos"))]
        const CANDIDATES: &[&str] = &["ffplay", "mpv", "afplay"];

        for &name in CANDIDATES {
            let ok = Command::new(name)
                .arg("-version")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|_| true)
                .unwrap_or(false);
            if ok {
                return Some(name);
            }
        }
        None
    }

    pub fn spawn_player(player: &str, path: &std::path::Path) -> Option<Child> {
        let path_str = path.to_string_lossy().into_owned();
        let mut cmd = Command::new(player);
        match player {
            "afplay" => {
                cmd.arg(&path_str);
            }
            "ffplay" => {
                cmd.args(["-nodisp", "-autoexit", "-loglevel", "quiet", &path_str]);
            }
            "mpv" => {
                cmd.args(["--no-video", "--really-quiet", &path_str]);
            }
            _ => {
                cmd.arg(&path_str);
            }
        }
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok()
    }
}

// ─── Web backend ───────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
mod backend {
    use super::*;
    use macroquad::audio::{
        load_sound, play_sound, stop_sound, PlaySoundParams, Sound,
    };

    pub struct MusicBackend {
        /// One slot per track. `None` if that track failed to load.
        sounds: Vec<Option<Sound>>,
        current: usize,
        /// Seconds elapsed since the current track started. We track
        /// this manually because `std::time::Instant` is unavailable
        /// on `wasm32-unknown-unknown`.
        elapsed: f32,
        playing: bool,
    }

    impl MusicBackend {
        pub fn new() -> Self {
            eprintln!("🎵 Music backend: macroquad::audio (WebAudio)");
            Self {
                sounds: Vec::new(),
                current: 0,
                elapsed: 0.0,
                playing: false,
            }
        }

        /// Preload every track. Async because on web
        /// `macroquad::audio::load_sound` fetches the file via HTTP.
        /// Called from `main` before the loop starts.
        pub async fn preload(&mut self, specs: &[TrackSpec]) {
            self.sounds.reserve(specs.len());
            for spec in specs {
                let path = crate::paths::asset_str(spec.path);
                match load_sound(&path).await {
                    Ok(snd) => {
                        eprintln!("🎵 loaded {}", spec.path);
                        self.sounds.push(Some(snd));
                    }
                    Err(e) => {
                        eprintln!(
                            "⚠️  web audio: failed to load {}: {e:?}",
                            spec.path
                        );
                        self.sounds.push(None);
                    }
                }
            }
        }

        /// Play the track at `idx` (looping). Stops the previous one
        /// first. Returns `false` if that track failed to load.
        pub fn play(&mut self, idx: usize) -> bool {
            if let Some(Some(prev)) = self.sounds.get(self.current) {
                stop_sound(prev);
            }
            self.current = idx;
            self.elapsed = 0.0;
            match self.sounds.get(idx) {
                Some(Some(snd)) => {
                    play_sound(
                        snd,
                        PlaySoundParams {
                            looped: true,
                            volume: MUSIC_VOLUME,
                        },
                    );
                    true
                }
                _ => false,
            }
        }

        pub fn stop(&mut self) {
            if let Some(Some(snd)) = self.sounds.get(self.current) {
                stop_sound(snd);
            }
        }

        /// Called from `MusicPlayer::tick` with the frame delta.
        pub fn advance_elapsed(&mut self, dt: f32) {
            self.elapsed += dt;
        }

        /// Seconds elapsed since the current track started.
        pub fn elapsed(&self) -> f32 {
            self.elapsed
        }

        pub fn set_playing(&mut self, v: bool) {
            self.playing = v;
        }

        pub fn is_playing(&self) -> bool {
            self.playing
        }
    }
}

// ─── Public wrapper ────────────────────────────────────────────────

pub struct MusicPlayer {
    backend: backend::MusicBackend,
    tracks: Vec<TrackSpec>,
    current: usize,
}

impl MusicPlayer {
    pub fn load(specs: Vec<TrackSpec>) -> Self {
        Self {
            backend: backend::MusicBackend::new(),
            tracks: specs,
            current: 0,
        }
    }

    /// Preloads every track. **Must be awaited before `start()` on
    /// web** (it fetches the files). No-op on native.
    pub async fn preload(&mut self) {
        self.backend.preload(&self.tracks).await;
    }

    pub fn start(&mut self) {
        if self.tracks.is_empty() {
            return;
        }
        self.current = 0;
        self.backend.set_playing(true);
        self.play_current();
    }

    pub fn stop(&mut self) {
        self.backend.stop();
        self.backend.set_playing(false);
    }

    pub fn is_playing(&self) -> bool {
        self.backend.is_playing()
    }

    /// Advances the playlist. Call once per frame.
    pub fn tick(&mut self, dt: f32) {
        if !self.backend.is_playing() || self.tracks.is_empty() {
            return;
        }
        self.backend.advance_elapsed(dt);
        let duration = self.tracks[self.current].duration;
        if self.backend.elapsed() >= duration {
            self.advance();
        }
    }

    fn advance(&mut self) {
        let loop_start = self
            .tracks
            .iter()
            .position(|t| !t.is_intro)
            .unwrap_or(0);
        let next = if self.current + 1 < self.tracks.len() {
            self.current + 1
        } else {
            loop_start
        };
        self.current = next;
        self.play_current();
    }

    /// Native: synchronous load + play.
    #[cfg(not(target_arch = "wasm32"))]
    fn play_current(&mut self) {
        let spec = &self.tracks[self.current];
        let full = crate::paths::asset(spec.path);

        if !full.exists() {
            eprintln!("⚠️  Music file not found: {full:?}");
            self.backend.set_playing(false);
            return;
        }

        if !self.backend.play(&full) {
            self.backend.set_playing(false);
        }
    }

    /// Web: play a preloaded track. No async needed here — preload
    /// already fetched everything.
    #[cfg(target_arch = "wasm32")]
    fn play_current(&mut self) {
        let idx = self.current;
        if !self.backend.play(idx) {
            eprintln!("⚠️  web audio: track {idx} unavailable");
            self.backend.set_playing(false);
        }
    }
}

// ─── SFX (teaser Halloween) ────────────────────────────────────────

/// Plays a piercing one-shot scare sound for the Halloween teaser.
///
/// On native, prefers `assets/scare.wav` then macOS system sound.
/// On web, silent (the teaser is disabled on web anyway — see
/// `is_spooky_season` in main.rs).
pub fn play_scare() {
    #[cfg(target_arch = "wasm32")]
    {
        // Silent on web for now — the Halloween teaser is disabled.
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let custom_wav = manifest.join("assets/scare.wav");
        let custom_ogg = manifest.join("assets/scare.ogg");

        let target: Option<std::path::PathBuf> = if custom_wav.exists() {
            Some(custom_wav)
        } else if custom_ogg.exists() {
            Some(custom_ogg)
        } else {
            #[cfg(target_os = "macos")]
            {
                Some(std::path::PathBuf::from(
                    "/System/Library/Sounds/Glass.aiff",
                ))
            }
            #[cfg(not(target_os = "macos"))]
            {
                None
            }
        };

        let Some(path) = target else { return };
        if let Some(player) = backend::detect_player() {
            let _ = backend::spawn_player(player, &path);
        }
    }
}

/// Plays the second-stage scare sound (final jumpscare).
pub fn play_scream() {
    #[cfg(target_arch = "wasm32")]
    {
        // Silent on web for now.
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let custom_wav = manifest.join("assets/scream.wav");
        let custom_ogg = manifest.join("assets/scream.ogg");

        let target: Option<std::path::PathBuf> = if custom_wav.exists() {
            Some(custom_wav)
        } else if custom_ogg.exists() {
            Some(custom_ogg)
        } else {
            #[cfg(target_os = "macos")]
            {
                Some(std::path::PathBuf::from(
                    "/System/Library/Sounds/Sosumi.aiff",
                ))
            }
            #[cfg(not(target_os = "macos"))]
            {
                None
            }
        };

        let Some(path) = target else { return };
        if let Some(player) = backend::detect_player() {
            let _ = backend::spawn_player(player, &path);
        }
    }
}

// ─── Playlist ─────────────────────────────────────────────────────

pub fn default_playlist() -> Vec<TrackSpec> {
    vec![
        TrackSpec {
            path: "sonican-modern-chinese-trip-asian-music-loop-499913.mp3",
            duration: 92.03,
            is_intro: true,
        },
        TrackSpec {
            path: "sonican-modern-trip-asian-phonk-loop-507321.mp3",
            duration: 88.03,
            is_intro: false,
        },
        TrackSpec {
            path: "sonican-asian-phonk-loop-2-modern-trip-508779.mp3",
            duration: 68.05,
            is_intro: false,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_playlist_has_three_tracks() {
        assert_eq!(default_playlist().len(), 3);
    }

    #[test]
    fn default_playlist_starts_with_one_intro() {
        let list = default_playlist();
        assert!(list[0].is_intro);
        assert!(!list[1].is_intro);
        assert!(!list[2].is_intro);
    }

    #[test]
    fn default_playlist_has_positive_durations() {
        for t in default_playlist() {
            assert!(t.duration > 0.0, "{} has non-positive duration", t.path);
        }
    }
}