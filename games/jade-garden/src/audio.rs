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
//! Playlist logic (intro, loop, timed advance) lives in
//! `ember_stdlib::music`; this file only supplies the native backend.
//! Time tracking: the native backend keeps its own `Instant` clock; on
//! wasm (no `Instant`) the player accumulates `dt` itself.

#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

// ─── Shared ────────────────────────────────────────────────────────

/// Volume applied to every track (0.0–1.0).
const MUSIC_VOLUME: f32 = 0.4;

pub use ember_stdlib::music::{MacroquadMusic, MusicBackend, MusicPlayer as Player, TrackSpec};

// ─── Native backend ────────────────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
mod backend {
    use super::*;
    use soloud::*;
    use std::process::{Child, Command, Stdio};
    use std::time::Instant;

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

    pub struct NativeMusic {
        inner: Backend,
        started_at: Instant,
    }

    impl NativeMusic {
        pub fn new() -> Self {
            match Soloud::default() {
                Ok(soloud) => {
                    eprintln!("🎵 Music backend: soloud (native)");
                    Self {
                        inner: Backend::Soloud {
                            soloud,
                            loaded: None,
                        },
                        started_at: Instant::now(),
                    }
                }
                Err(e) => {
                    eprintln!("⚠️  soloud unavailable ({e:?}), trying OS player...");
                    match detect_player() {
                        Some(player) => {
                            eprintln!("🎵 Music backend: {player} (subprocess)");
                            Self {
                                inner: Backend::Subprocess {
                                    player,
                                    child: None,
                                },
                                started_at: Instant::now(),
                            }
                        }
                        None => {
                            eprintln!("⚠️  No audio backend available. Music disabled.");
                            Self {
                                inner: Backend::Subprocess {
                                    player: "",
                                    child: None,
                                },
                                started_at: Instant::now(),
                            }
                        }
                    }
                }
            }
        }

        fn play_file(&mut self, path: &PathBuf) -> bool {
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
    }

    impl MusicBackend for NativeMusic {
        /// Loading is lazy: done by `play`.
        fn play(&mut self, _idx: usize, track: &TrackSpec) -> bool {
            let full = crate::paths::asset(track.path);
            if !full.exists() {
                eprintln!("⚠️  Music file not found: {full:?}");
                return false;
            }
            self.play_file(&full)
        }

        fn stop(&mut self) {
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

        /// Native keeps its own wall clock (audio keeps playing even if
        /// frames stall).
        fn elapsed(&self) -> Option<f32> {
            Some(self.started_at.elapsed().as_secs_f32())
        }
    }

    impl Drop for NativeMusic {
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

// ─── Public player ─────────────────────────────────────────────────

/// Native: soloud or OS player. Web: `macroquad::audio` (WebAudio).
#[cfg(not(target_arch = "wasm32"))]
type Backend = backend::NativeMusic;
#[cfg(target_arch = "wasm32")]
type Backend = MacroquadMusic;

pub type MusicPlayer = Player<Backend>;

/// Build the music player for `specs`. On web, `preload().await` must
/// run before `start()`.
pub fn new_player(specs: Vec<TrackSpec>) -> MusicPlayer {
    #[cfg(not(target_arch = "wasm32"))]
    let backend = backend::NativeMusic::new();
    #[cfg(target_arch = "wasm32")]
    let backend = MacroquadMusic::new(MUSIC_VOLUME, crate::paths::asset_str);
    Player::new(backend, specs)
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
