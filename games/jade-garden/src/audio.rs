//! Background music: hybrid player.
//!
//! Tries, in order:
//!
//! 1. **soloud** (bundled miniaudio backend, dlopens the system audio
//!    library). Works out of the box on most setups, no external
//!    runtime dependency.
//! 2. **OS-native player subprocess**:
//!    - macOS: `afplay` (part of the base system)
//!    - Linux / Windows: `ffplay` (ffmpeg) or `mpv`
//!
//! If neither works, music is disabled and the game keeps running.
//!
//! Why not rodio / cpal / kira? They collide with macroquad at the
//! Cargo level: both end up declaring `links = "alsa"` and Cargo
//! refuses to resolve the graph — even on macOS, because Cargo
//! resolves for all targets.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Instant;
use crate::paths;
use soloud::*;

/// Volume applied to every track (0.0–1.0).
const MUSIC_VOLUME: f32 = 0.4;

/// One entry in the background music playlist.
pub struct TrackSpec {
    /// Filename relative to the assets directory (e.g. `foo.mp3`).
    pub path: &'static str,
    /// Duration in seconds.
    pub duration: f32,
    /// `true` → plays once, then advances. `false` → part of the loop.
    pub is_intro: bool,
}

enum Backend {
    /// Native soloud engine, kept alive for the player's lifetime.
    Soloud {
        soloud: Soloud,
        loaded: Option<audio::Wav>,
    },
    /// External player subprocess.
    Subprocess {
        /// Name of the executable we detected, for reference.
        player: &'static str,
        child: Option<Child>,
    },
}

pub struct MusicPlayer {
    backend: Backend,
    tracks: Vec<TrackSpec>,
    current: usize,
    started_at: Instant,
    playing: bool,
}

impl MusicPlayer {
    pub fn load(specs: Vec<TrackSpec>) -> Self {
        // Try soloud first.
        match Soloud::default() {
            Ok(soloud) => {
                eprintln!("🎵 Music backend: soloud (native)");
                Self {
                    backend: Backend::Soloud {
                        soloud,
                        loaded: None,
                    },
                    tracks: specs,
                    current: 0,
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
                            backend: Backend::Subprocess {
                                player,
                                child: None,
                            },
                            tracks: specs,
                            current: 0,
                            started_at: Instant::now(),
                            playing: false,
                        }
                    }
                    None => {
                        eprintln!(
                            "⚠️  No audio backend available. Music disabled.\n\
                             Install one of: afplay (macOS, built-in), \
                             ffplay (ffmpeg), or mpv."
                        );
                        // Fall back to soloud anyway so the API stays
                        // consistent; `start()` will just no-op.
                        Self {
                            backend: Backend::Subprocess {
                                player: "",
                                child: None,
                            },
                            tracks: specs,
                            current: 0,
                            started_at: Instant::now(),
                            playing: false,
                        }
                    }
                }
            }
        }
    }

    pub fn stop(&mut self) {
        match &mut self.backend {
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
        self.playing = false;
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    /// Advances the playlist. Call once per frame.
    pub fn tick(&mut self, _dt: f32) {
        if !self.playing || self.tracks.is_empty() {
            return;
        }

        // Subprocess: rely on the OS process exit to know when a track
        // ended. Soloud: use wall-clock duration.
        let finished = match &mut self.backend {
            Backend::Soloud { .. } => {
                let spec = &self.tracks[self.current];
                self.started_at.elapsed().as_secs_f32() >= spec.duration
            }
            Backend::Subprocess { child, .. } => match child.as_mut() {
                Some(c) => match c.try_wait() {
                    Ok(Some(_)) | Err(_) => {
                        *child = None;
                        true
                    }
                    Ok(None) => false,
                },
                None => true,
            },
        };

        if finished {
            self.advance();
        }
    }

    pub fn start(&mut self) {
        if self.tracks.is_empty() {
            return;
        }
        if let Backend::Subprocess { player, .. } = &self.backend
            && player.is_empty()
        {
            return;
        }
        self.current = 0;
        self.playing = true;
        self.play_current_with_skip();
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
        self.play_current_with_skip();
    }

    /// Tries to play the current track. On failure, tries every other
    /// track once. If none work, disables music.
    fn play_current_with_skip(&mut self) {
        let original = self.current;
        for offset in 0..self.tracks.len() {
            self.current = (original + offset) % self.tracks.len();
            if self.play_current_inner() {
                return;
            }
        }
        eprintln!(
            "⚠️  All {} music tracks failed to play; disabling music.",
            self.tracks.len()
        );
        self.playing = false;
    }

    /// Attempts to play the current track. Returns `true` on success.
    fn play_current_inner(&mut self) -> bool {
        let spec = &self.tracks[self.current];
        let full = paths::asset(spec.path);
        let label = spec.path;

        if !full.exists() {
            eprintln!("⚠️  Music file not found: {}", full.display());
            return false;
        }

        match &mut self.backend {
            Backend::Soloud { soloud, loaded } => {
                soloud.stop_all();
                *loaded = None;
                let mut wav = audio::Wav::default();
                if let Err(e) = wav.load(&full) {
                    eprintln!("⚠️  soloud: failed to load {label}: {e:?}");
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
                match spawn_player(player, &full) {
                    Some(c) => {
                        *child = Some(c);
                        self.started_at = Instant::now();
                        true
                    }
                    None => {
                        eprintln!("⚠️  {player} failed to start for {label}");
                        false
                    }
                }
            }
        }
    }
}

impl Drop for MusicPlayer {
    fn drop(&mut self) {
        if let Backend::Subprocess { child, .. } = &mut self.backend
            && let Some(mut c) = child.take()
        {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

/// Tries `--version` on a list of candidate players, returns the first
/// one whose executable exists and runs.
fn detect_player() -> Option<&'static str> {
    // On macOS afplay is guaranteed by the system.
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

/// Spawns the given player with arguments tuned to `player`.
fn spawn_player(player: &str, path: &Path) -> Option<Child> {
    let path_str = path.to_string_lossy().into_owned();
    let mut cmd = Command::new(player);
    match player {
        "afplay" => {
            // afplay has no volume flag; leave default.
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

/// Convenience: the jade-garden default playlist.
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

pub fn play_scare() {
    let custom_wav = paths::asset("scare.wav");
    let custom_ogg = paths::asset("scare.ogg");

    let target: Option<PathBuf> = if custom_wav.exists() {
        Some(custom_wav)
    } else if custom_ogg.exists() {
        Some(custom_ogg)
    } else {
        #[cfg(target_os = "macos")]
        {
            Some(PathBuf::from("/System/Library/Sounds/Glass.aiff"))
        }
        #[cfg(not(target_os = "macos"))]
        {
            None
        }
    };

    let Some(path) = target else { return };
    if let Some(player) = detect_player() {
        let _ = spawn_player(player, &path);
    }
}

pub fn play_scream() {
    let custom_wav = paths::asset("scream.wav");
    let custom_ogg = paths::asset("scream.ogg");

    let target: Option<PathBuf> = if custom_wav.exists() {
        Some(custom_wav)
    } else if custom_ogg.exists() {
        Some(custom_ogg)
    } else {
        #[cfg(target_os = "macos")]
        {
            Some(PathBuf::from("/System/Library/Sounds/Sosumi.aiff"))
        }
        #[cfg(not(target_os = "macos"))]
        {
            None
        }
    };

    let Some(path) = target else { return };
    if let Some(player) = detect_player() {
        let _ = spawn_player(player, &path);
    }
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

    #[test]
    fn spawn_player_returns_none_for_missing_binary() {
        let path = std::path::Path::new("/dev/null");
        let result = spawn_player("this-binary-does-not-exist-jade-xyz", path);
        assert!(result.is_none(), "unknown player must return None");
    }

    #[test]
    fn detect_player_does_not_panic() {
        // Result depends on the host; we only check it doesn't panic.
        let _ = detect_player();
    }

    #[test]
    fn play_scare_and_play_scream_do_not_panic() {
        // Fire-and-forget; must never panic even without system sounds.
        play_scare();
        play_scream();
    }
}