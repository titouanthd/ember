//! The audio trait. Games never call `macroquad::audio` directly.

use crate::{AudioClipId, AudioFeature, ServerError, SoundId};

pub trait AudioServer {
    /// Play `clip` at `volume` (0..1) and `pitch` (1.0 = normal). Returns
    /// a `SoundId` which can be passed to `stop`. Finished sounds are
    /// auto-removed; `stop` on a finished sound is a no-op.
    fn play(&mut self, clip: AudioClipId, volume: f32, pitch: f32) -> SoundId;

    fn stop(&mut self, sound: SoundId);
    fn stop_all(&mut self);

    /// Master volume, applied to every subsequent `play`. Range 0..1.
    fn set_master_volume(&mut self, volume: f32);
    fn master_volume(&self) -> f32;

    /// Flush any buffered playback to the device. Called by `MainLoop`
    /// once per frame after `process`/`draw`.
    fn flush(&mut self);

    /// Load a WAV/OGG from raw bytes. Path resolution is `ember_assets`.
    fn load_clip(&mut self, bytes: &[u8]) -> Result<AudioClipId, ServerError>;
    fn destroy_clip(&mut self, clip: AudioClipId);

    fn supports(&self, feature: AudioFeature) -> bool;
}
