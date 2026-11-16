//! Opaque handle types and capability enums.
//!
//! All IDs are backend-allocated and opaque to game code. Comparing two
//! textures for equality is `id1 == id2`; nothing else is meaningful.

/// A handle to a GPU texture, allocated by the rendering backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureId(u64);

impl TextureId {
    pub fn new(raw: u64) -> Self {
        Self(raw)
    }
    pub fn raw(self) -> u64 {
        self.0
    }
}

/// A handle to a loaded font, allocated by the rendering backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FontId(u64);

impl FontId {
    pub fn new(raw: u64) -> Self {
        Self(raw)
    }
    pub fn raw(self) -> u64 {
        self.0
    }
}

/// A handle to a live playback instance returned by `AudioServer::play`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SoundId(u64);

impl SoundId {
    pub fn new(raw: u64) -> Self {
        Self(raw)
    }
    pub fn raw(self) -> u64 {
        self.0
    }
}

/// A handle to a loaded audio clip, allocated by the audio backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AudioClipId(u64);

impl AudioClipId {
    pub fn new(raw: u64) -> Self {
        Self(raw)
    }
    pub fn raw(self) -> u64 {
        self.0
    }
}

/// Metrics returned by `RenderingServer::measure_text`.
///
/// `offset_y` is the baseline offset the backend applies when drawing at
/// a given Y. Matches macroquad's `TextDimensions` shape so the adapter
/// is a direct field copy.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TextMetrics {
    pub width: f32,
    pub height: f32,
    pub offset_y: f32,
}

/// Rendering capability flags, probed with `RenderingServer::supports`.
///
/// A game that queries a feature the backend lacks takes a fallback path
/// (see `12-servers.md §8.1`). New variants are added over time; match
/// arms must include `_`.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderFeature {
    AlphaBlending,
    CustomShaders,
    TextureArray,
    Instancing,
}

/// Audio capability flags, probed with `AudioServer::supports`.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AudioFeature {
    PitchShift,
    Streaming,
    PositionalAudio,
}
