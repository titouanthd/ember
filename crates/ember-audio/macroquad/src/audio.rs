//! `AudioServer` implementation on macroquad.
//!
//! ## Async loading
//!
//! `load_clip` blocks on `load_sound_from_bytes` on native. On wasm it
//! returns `ServerError::NotSupported` — games on wasm load audio
//! before entering the loop. See ADR-020.
//!
//! ## Playback
//!
//! macroquad's `play_sound` takes `PlaySoundParams`. Volume is clamped
//! 0..1 per macroquad's contract; pitch is not supported by macroquad
//! (no per-playback pitch control), so it is accepted and ignored.
//! `supports(PitchShift)` returns `false` to advertise this.

use std::collections::HashMap;

use ember_servers::{AudioClipId, AudioFeature, AudioServer, ServerError, SoundId};
use macroquad::audio::{PlaySoundParams, Sound, load_sound_from_bytes, play_sound, stop_sound};

pub struct MacroquadAudio {
    clips: HashMap<AudioClipId, Sound>,
    sound_to_clip: HashMap<SoundId, AudioClipId>,
    next_clip: u64,
    next_sound: u64,
    master_volume: f32,
    capabilities: Vec<AudioFeature>,
}

impl MacroquadAudio {
    pub fn new() -> Self {
        Self {
            clips: HashMap::new(),
            sound_to_clip: HashMap::new(),
            next_clip: 0,
            next_sound: 0,
            master_volume: 1.0,
            capabilities: vec![],
        }
    }

    /// Adopt a sound loaded externally (e.g., before the loop on wasm).
    /// See ADR-020.
    pub fn adopt_clip(&mut self, sound: Sound) -> AudioClipId {
        self.next_clip += 1;
        let id = AudioClipId::new(self.next_clip);
        self.clips.insert(id, sound);
        id
    }
}

impl Default for MacroquadAudio {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioServer for MacroquadAudio {
    fn play(&mut self, clip: AudioClipId, volume: f32, _pitch: f32) -> SoundId {
        self.next_sound += 1;
        let sound_id = SoundId::new(self.next_sound);

        if let Some(sound) = self.clips.get(&clip) {
            play_sound(
                sound,
                PlaySoundParams {
                    looped: false,
                    volume: volume.clamp(0.0, 1.0) * self.master_volume,
                },
            );
            self.sound_to_clip.insert(sound_id, clip);
        }
        sound_id
    }

    fn stop(&mut self, sound: SoundId) {
        if let Some(clip_id) = self.sound_to_clip.remove(&sound)
            && let Some(clip) = self.clips.get(&clip_id)
        {
            stop_sound(clip);
        }
    }

    fn stop_all(&mut self) {
        for sound in self.clips.values() {
            stop_sound(sound);
        }
        self.sound_to_clip.clear();
    }

    fn set_master_volume(&mut self, volume: f32) {
        self.master_volume = volume.clamp(0.0, 1.0);
    }

    fn master_volume(&self) -> f32 {
        self.master_volume
    }

    fn flush(&mut self) {
        // macroquad flushes automatically on `next_frame`. No-op.
    }

    fn load_clip(&mut self, bytes: &[u8]) -> Result<AudioClipId, ServerError> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = bytes;
            return Err(ServerError::NotSupported);
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let sound = futures::executor::block_on(load_sound_from_bytes(bytes))
                .map_err(|e| ServerError::Backend(format!("audio parse: {e}")))?;
            self.next_clip += 1;
            let id = AudioClipId::new(self.next_clip);
            self.clips.insert(id, sound);
            Ok(id)
        }
    }

    fn destroy_clip(&mut self, clip: AudioClipId) {
        self.clips.remove(&clip);
        self.sound_to_clip.retain(|_, c| *c != clip);
    }

    fn supports(&self, feature: AudioFeature) -> bool {
        self.capabilities.contains(&feature)
    }
}
