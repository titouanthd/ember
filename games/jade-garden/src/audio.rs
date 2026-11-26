//! Background music and the Halloween teaser's two shrieks.
//!
//! **Music** plays through the engine's `AudioServer` (ADR-043): each
//! track is a clip played on a loop, and [`MusicPlayer`] moves through
//! the playlist on a timer (an intro once, then the other tracks in a
//! loop). The tracks are MP3s; the native backend decodes them and the
//! browser decodes them itself. Native reads them from `assets/`; the web
//! build has them built in. (Was soloud or an OS player natively and
//! `macroquad::audio` on the web.)
//!
//! **The teaser's shrieks** are native-only one-shots played by an OS
//! player subprocess (`afplay`, `ffplay`, `mpv`), as before: the scare
//! falls back to a macOS system sound in AIFF, which the engine does not
//! read.

use ember_servers::{AudioClipId, AudioServer, SoundId};

/// Volume applied to every track (0.0–1.0).
const MUSIC_VOLUME: f32 = 0.4;

/// One playlist entry (was `ember_stdlib::music::TrackSpec`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrackSpec {
    /// File name under `assets/`.
    pub path: &'static str,
    /// Seconds before moving to the next track.
    pub duration: f32,
    /// Played once at the start, then skipped when the playlist loops.
    pub is_intro: bool,
}

/// What `start` / `stop` asked for, applied on the next `tick`, which has
/// the audio server. Lets the app's state machine stay audio-free.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Request {
    Start,
    Stop,
}

/// The playlist, on the engine's audio (was `ember_stdlib::music`).
pub struct MusicPlayer {
    tracks: Vec<TrackSpec>,
    clips: Vec<Option<AudioClipId>>,
    current: usize,
    elapsed: f32,
    playing: bool,
    /// The current track's clip is still decoding (the web); it starts,
    /// and its timer runs, once it is ready.
    waiting: bool,
    sound: Option<SoundId>,
    request: Option<Request>,
}

impl MusicPlayer {
    /// A player for `tracks`; nothing is loaded until [`load`](Self::load).
    pub fn new(tracks: Vec<TrackSpec>) -> Self {
        Self {
            tracks,
            clips: Vec::new(),
            current: 0,
            elapsed: 0.0,
            playing: false,
            waiting: false,
            sound: None,
            request: None,
        }
    }

    /// Load every track's clip. A track that fails is skipped with a
    /// warning, and the playlist stops when it reaches it, as before.
    pub fn load(&mut self, audio: &mut dyn AudioServer) {
        self.clips = self
            .tracks
            .iter()
            .map(|t| match track_bytes(t.path) {
                Some(bytes) => match audio.load_clip(&bytes) {
                    Ok(clip) => Some(clip),
                    Err(e) => {
                        eprintln!("⚠️  music: failed to load {}: {e}", t.path);
                        None
                    }
                },
                None => {
                    eprintln!("⚠️  Music file not found: {}", t.path);
                    None
                }
            })
            .collect();
    }

    /// Start from the first track (on the next `tick`).
    pub fn start(&mut self) {
        self.request = Some(Request::Start);
    }

    /// Stop the music (on the next `tick`).
    pub fn stop(&mut self) {
        self.request = Some(Request::Stop);
    }

    pub fn is_playing(&self) -> bool {
        match self.request {
            Some(Request::Start) => true,
            Some(Request::Stop) => false,
            None => self.playing,
        }
    }

    /// The track playing now, if any.
    pub fn current_track(&self) -> Option<usize> {
        (self.playing && !self.waiting).then_some(self.current)
    }

    /// Apply `start` / `stop`, then move the playlist on by `dt`.
    pub fn tick(&mut self, dt: f32, audio: &mut dyn AudioServer) {
        match self.request.take() {
            Some(Request::Start) if !self.tracks.is_empty() => {
                self.current = 0;
                self.playing = true;
                self.play_current(audio);
            }
            Some(Request::Stop) => {
                self.silence(audio);
                self.playing = false;
                self.waiting = false;
            }
            _ => {}
        }
        if !self.playing || self.tracks.is_empty() {
            return;
        }
        if self.waiting {
            self.resume(audio);
            if self.waiting {
                return;
            }
        }
        self.elapsed += dt;
        let duration = self.tracks[self.current].duration;
        if duration > 0.0 && self.elapsed >= duration {
            let loop_start = self.tracks.iter().position(|t| !t.is_intro).unwrap_or(0);
            self.current = if self.current + 1 < self.tracks.len() {
                self.current + 1
            } else {
                loop_start
            };
            self.play_current(audio);
        }
    }

    fn play_current(&mut self, audio: &mut dyn AudioServer) {
        self.silence(audio);
        self.elapsed = 0.0;
        self.waiting = false;
        match self.clips.get(self.current).copied().flatten() {
            Some(_) => {
                self.waiting = true;
                self.resume(audio);
            }
            None => {
                let path = self.tracks[self.current].path;
                eprintln!("[music] track {} unavailable: {path}", self.current);
                self.playing = false;
            }
        }
    }

    /// Start the waiting track if its clip is ready.
    fn resume(&mut self, audio: &mut dyn AudioServer) {
        if let Some(clip) = self.clips.get(self.current).copied().flatten()
            && audio.is_clip_ready(clip)
        {
            self.sound = Some(audio.play_looped(clip, MUSIC_VOLUME));
            self.waiting = false;
        }
    }

    fn silence(&mut self, audio: &mut dyn AudioServer) {
        if let Some(sound) = self.sound.take() {
            audio.stop(sound);
        }
    }
}

/// A track's bytes: read from `assets/` natively.
#[cfg(not(target_arch = "wasm32"))]
fn track_bytes(path: &str) -> Option<Vec<u8>> {
    std::fs::read(crate::paths::asset(path)).ok()
}

/// A track's bytes: built in on the web, which has no file system.
#[cfg(target_arch = "wasm32")]
fn track_bytes(path: &str) -> Option<Vec<u8>> {
    let bytes: &[u8] = match path {
        "sonican-modern-chinese-trip-asian-music-loop-499913.mp3" => {
            include_bytes!("../assets/sonican-modern-chinese-trip-asian-music-loop-499913.mp3")
        }
        "sonican-modern-trip-asian-phonk-loop-507321.mp3" => {
            include_bytes!("../assets/sonican-modern-trip-asian-phonk-loop-507321.mp3")
        }
        "sonican-asian-phonk-loop-2-modern-trip-508779.mp3" => {
            include_bytes!("../assets/sonican-asian-phonk-loop-2-modern-trip-508779.mp3")
        }
        _ => return None,
    };
    Some(bytes.to_vec())
}

// ─── OS player (native shrieks only) ────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
mod os_player {
    use std::process::{Child, Command, Stdio};

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
        if let Some(player) = os_player::detect_player() {
            let _ = os_player::spawn_player(player, &path);
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
        if let Some(player) = os_player::detect_player() {
            let _ = os_player::spawn_player(player, &path);
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
