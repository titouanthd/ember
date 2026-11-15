//! Sound effects.
//!
//! Every clip is optional: a missing or malformed WAV makes that event
//! silent instead of crashing, so the game stays playable from a fresh
//! checkout even if the generated assets were not built yet.
//!
//! Files live in `assets/sfx/` and are produced by
//! `tools/generate-sounds.py`.
//!
//! macroquad exposes no per-playback pitch control, so the perfect
//! chime is a set of pre-generated notes on a pentatonic scale. The
//! combo selects the note, which makes the escalation audible without
//! any runtime DSP.

use ember_stdlib::audio::AudioClip;

use crate::paths;
use crate::systems::GameEvent;

/// Number of pre-generated perfect notes. Combos above this cap stay
/// on the highest note.
pub const PERFECT_NOTES: usize = 6;

/// Playback volumes, mixed against each other.
const VOL_LAND: f32 = 0.50;
const VOL_SLICE: f32 = 0.30;
/// A cut at least this wide plays at full slice volume. Smaller shaves
/// scale down towards [`SLICE_MIN_FRACTION`].
const SLICE_FULL_WIDTH_PX: f32 = 120.0;
const SLICE_MIN_FRACTION: f32 = 0.30;
/// The landing thud keeps a little body under the perfect chime.
const VOL_PERFECT_LAND: f32 = 0.18;
const VOL_PERFECT: f32 = 0.70;
const VOL_GAME_OVER: f32 = 0.90;
const VOL_START: f32 = 0.45;
const VOL_NEW_BEST: f32 = 0.65;

pub struct SoundBank {
    land: Option<AudioClip>,
    slice: Option<AudioClip>,
    perfect: Vec<Option<AudioClip>>,
    game_over: Option<AudioClip>,
    start: Option<AudioClip>,
    new_best: Option<AudioClip>,
    muted: bool,
}

impl SoundBank {
    /// Preload every clip. Must be awaited before the loop on the web,
    /// where each load performs an HTTP fetch.
    pub async fn load() -> Self {
        async fn clip(name: &str) -> Option<AudioClip> {
            let path = paths::asset(&format!("sfx/{name}.wav"));
            match AudioClip::load(&path).await {
                Ok(clip) => Some(clip),
                Err(e) => {
                    eprintln!("[audio] failed to load {}: {e}", path.display());
                    None
                }
            }
        }

        let mut perfect = Vec::with_capacity(PERFECT_NOTES);
        for i in 1..=PERFECT_NOTES {
            perfect.push(clip(&format!("perfect_{i}")).await);
        }

        Self {
            land: clip("drop").await,
            slice: clip("slice").await,
            perfect,
            game_over: clip("game_over").await,
            start: clip("start").await,
            new_best: clip("new_best").await,
            muted: false,
        }
    }

    /// Translate one simulation event into sound.
    pub fn on_event(&self, event: GameEvent) {
        match event {
            GameEvent::Land => self.play(self.land.as_ref(), VOL_LAND),
            GameEvent::Slice { cut_width } => {
                self.play(self.slice.as_ref(), slice_volume(cut_width));
            }
            GameEvent::Perfect { combo } => {
                // A soft thud under the chime keeps the landing physical.
                self.play(self.land.as_ref(), VOL_PERFECT_LAND);
                let note = self
                    .perfect
                    .get(perfect_index(combo))
                    .and_then(Option::as_ref);
                self.play(note, VOL_PERFECT);
            }
            GameEvent::Miss => self.play(self.game_over.as_ref(), VOL_GAME_OVER),
        }
    }

    /// Menu start and restart.
    pub fn start(&self) {
        self.play(self.start.as_ref(), VOL_START);
    }

    /// Short fanfare when a run beats the record. The WAV itself starts
    /// with silence so it does not collide with the game-over thud.
    pub fn new_best(&self) {
        self.play(self.new_best.as_ref(), VOL_NEW_BEST);
    }

    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
        if self.muted {
            self.stop_all();
        }
    }

    pub fn is_muted(&self) -> bool {
        self.muted
    }

    /// Cut whatever is currently ringing so muting is immediate, not
    /// "from the next sound onwards".
    fn stop_all(&self) {
        for clip in [
            self.land.as_ref(),
            self.slice.as_ref(),
            self.game_over.as_ref(),
            self.start.as_ref(),
            self.new_best.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            clip.stop();
        }
        for clip in self.perfect.iter().flatten() {
            clip.stop();
        }
    }

    fn play(&self, clip: Option<&AudioClip>, volume: f32) {
        if self.muted {
            return;
        }
        if let Some(clip) = clip {
            clip.play_at(volume);
        }
    }
}

/// Note index for a perfect combo. Combo 1 plays the first note; the
/// scale plateaus at [`PERFECT_NOTES`].
pub fn perfect_index(combo: u32) -> usize {
    let index = combo.saturating_sub(1) as usize;
    index.min(PERFECT_NOTES.saturating_sub(1))
}

/// Slice volume for a cut of `cut_width` virtual pixels.
///
/// A tiny shave (just outside the perfect threshold) stays near
/// [`SLICE_MIN_FRACTION`] of [`VOL_SLICE`]; a large chop reaches full
/// volume at [`SLICE_FULL_WIDTH_PX`].
pub fn slice_volume(cut_width: f32) -> f32 {
    let scale = (cut_width / SLICE_FULL_WIDTH_PX).clamp(0.0, 1.0);
    VOL_SLICE * (SLICE_MIN_FRACTION + (1.0 - SLICE_MIN_FRACTION) * scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn silent_bank() -> SoundBank {
        SoundBank {
            land: None,
            slice: None,
            perfect: (0..PERFECT_NOTES).map(|_| None).collect(),
            game_over: None,
            start: None,
            new_best: None,
            muted: false,
        }
    }

    #[test]
    fn perfect_index_starts_at_zero() {
        assert_eq!(perfect_index(1), 0);
        assert_eq!(perfect_index(2), 1);
    }

    #[test]
    fn perfect_index_clamps_at_the_last_note() {
        assert_eq!(perfect_index(PERFECT_NOTES as u32), PERFECT_NOTES - 1);
        assert_eq!(perfect_index(PERFECT_NOTES as u32 + 1), PERFECT_NOTES - 1);
        assert_eq!(perfect_index(999), PERFECT_NOTES - 1);
    }

    #[test]
    fn perfect_index_handles_combo_zero() {
        assert_eq!(perfect_index(0), 0);
    }

    #[test]
    fn every_event_is_handled_silently() {
        let bank = silent_bank();
        for event in [
            GameEvent::Land,
            GameEvent::Slice { cut_width: 0.0 },
            GameEvent::Slice { cut_width: 500.0 },
            GameEvent::Perfect { combo: 1 },
            GameEvent::Perfect { combo: 99 },
            GameEvent::Miss,
        ] {
            bank.on_event(event);
        }
        bank.start();
        bank.new_best();
    }

    #[test]
    fn mute_toggles() {
        let mut bank = silent_bank();
        assert!(!bank.is_muted());
        bank.toggle_mute();
        assert!(bank.is_muted());
        bank.toggle_mute();
        assert!(!bank.is_muted());
    }

    #[test]
    fn small_slices_are_quiet_and_large_ones_open_up() {
        let whisper = slice_volume(0.0);
        let full = slice_volume(SLICE_FULL_WIDTH_PX);
        let overshoot = slice_volume(SLICE_FULL_WIDTH_PX * 10.0);

        assert!((whisper - VOL_SLICE * SLICE_MIN_FRACTION).abs() < 1e-6);
        assert!((full - VOL_SLICE).abs() < 1e-6);
        assert!((overshoot - VOL_SLICE).abs() < 1e-6);
        assert!(slice_volume(60.0) > whisper);
        assert!(slice_volume(60.0) < full);
    }

    #[test]
    fn negative_slice_width_is_clamped() {
        assert!((slice_volume(-10.0) - slice_volume(0.0)).abs() < 1e-6);
    }

    // quad-snd (below macroquad) resamples every non-44.1 kHz WAV with
    // a nearest-neighbour filter, which crackles. Keep the whole bank
    // at 44.1 kHz / mono / 16-bit PCM.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn every_sfx_clip_uses_the_native_mix_format() {
        fn fmt_chunk(bytes: &[u8]) -> (u16, u16, u32, u16) {
            assert_eq!(&bytes[0..4], b"RIFF");
            assert_eq!(&bytes[8..12], b"WAVE");

            let mut offset = 12;
            while offset + 8 <= bytes.len() {
                let id = &bytes[offset..offset + 4];
                let size = u32::from_le_bytes(
                    bytes[offset + 4..offset + 8]
                        .try_into()
                        .expect("4-byte chunk size"),
                ) as usize;
                let data = offset + 8;
                if id == b"fmt " {
                    let audio_format =
                        u16::from_le_bytes(bytes[data..data + 2].try_into().unwrap());
                    let channels =
                        u16::from_le_bytes(bytes[data + 2..data + 4].try_into().unwrap());
                    let sample_rate =
                        u32::from_le_bytes(bytes[data + 4..data + 8].try_into().unwrap());
                    let bits = u16::from_le_bytes(bytes[data + 14..data + 16].try_into().unwrap());
                    return (audio_format, channels, sample_rate, bits);
                }
                offset = data + size + (size & 1);
            }
            panic!("no fmt chunk");
        }

        let mut clips: Vec<String> = vec![
            "drop.wav".to_owned(),
            "slice.wav".to_owned(),
            "game_over.wav".to_owned(),
            "start.wav".to_owned(),
            "new_best.wav".to_owned(),
        ];
        clips.extend((1..=PERFECT_NOTES).map(|i| format!("perfect_{i}.wav")));

        for name in clips {
            let path = paths::asset(&format!("sfx/{name}"));
            let label = path.display().to_string();
            let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("missing {label}: {e}"));
            let (format, channels, sample_rate, bits) = fmt_chunk(&bytes);
            assert_eq!(format, 1, "{name}: expected PCM");
            assert_eq!(channels, 1, "{name}: expected mono");
            assert_eq!(
                sample_rate, 44_100,
                "{name}: must be 44.1 kHz to bypass quad-snd's resampler"
            );
            assert_eq!(bits, 16, "{name}: expected 16-bit samples");
        }
    }
}
