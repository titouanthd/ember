//! Recording backend. Records every `draw_*` call in order with the
//! z-level active at the moment of the call. Every other method is
//! a no-op or returns a default.
//!
//! This is the test harness for `ember_scene`, `ember_ui`, and any
//! crate that wants to verify "what would this render look like"
//! without a window.

use crate::*;
use ember_core::math::Rect;
use ember_core::{Color, Image};
use glam::Vec2;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq)]
pub enum RecordedCall {
    SetClearColor(Color),
    SetCamera(Vec2),
    SetZ(f32),
    SetDefaultFont(Option<FontId>),
    /// `rect` as requested; `effective` after intersecting with the parent.
    PushClipRect {
        rect: Rect,
        effective: Rect,
    },
    PopClipRect,
    DrawRect {
        rect: Rect,
        color: Color,
        z: f32,
    },
    DrawRectOutline {
        rect: Rect,
        thickness: f32,
        color: Color,
        z: f32,
    },
    DrawRectRotated {
        rect: Rect,
        rotation: f32,
        color: Color,
        z: f32,
    },
    DrawCircle {
        center: Vec2,
        radius: f32,
        color: Color,
        z: f32,
    },
    DrawCircleOutline {
        center: Vec2,
        radius: f32,
        thickness: f32,
        color: Color,
        z: f32,
    },
    DrawLine {
        a: Vec2,
        b: Vec2,
        thickness: f32,
        color: Color,
        z: f32,
    },
    DrawTriangle {
        a: Vec2,
        b: Vec2,
        c: Vec2,
        color: Color,
        z: f32,
    },
    DrawSprite {
        texture: TextureId,
        dst: Rect,
        src: Option<Rect>,
        tint: Color,
        z: f32,
    },
    DrawText {
        text: String,
        pos: Vec2,
        size: f32,
        color: Color,
        z: f32,
    },
    DrawTextWithFont {
        font: FontId,
        text: String,
        pos: Vec2,
        size: f32,
        color: Color,
        z: f32,
    },
    CreateTexture(TextureId),
    DestroyTexture(TextureId),
    CreateFont(FontId),
    DestroyFont(FontId),
}

#[derive(Debug, Clone, Default)]
pub struct MockRenderer {
    calls: Vec<RecordedCall>,
    viewport: Rect,
    camera_offset: Vec2,
    current_z: f32,
    default_font: Option<FontId>,
    next_texture: u64,
    next_font: u64,
    capabilities: HashSet<RenderFeature>,
    in_frame: bool,
    clip: ClipStack,
    /// Fixed text width returned by `measure_text`, scaled by `size`.
    pub text_width_per_size: f32,
    /// Fixed text height returned by `measure_text`, scaled by `size`.
    pub text_height_per_size: f32,
}

impl MockRenderer {
    pub fn new() -> Self {
        Self {
            text_width_per_size: 0.6,
            text_height_per_size: 1.0,
            capabilities: HashSet::from([RenderFeature::AlphaBlending]),
            ..Default::default()
        }
    }

    pub fn calls(&self) -> &[RecordedCall] {
        &self.calls
    }
    pub fn take_calls(&mut self) -> Vec<RecordedCall> {
        std::mem::take(&mut self.calls)
    }
    pub fn clear_calls(&mut self) {
        self.calls.clear();
    }
    pub fn grant(&mut self, feature: RenderFeature) {
        self.capabilities.insert(feature);
    }

    /// Panics unless every call was preceded by `begin_frame` and
    /// followed by `end_frame`.
    pub fn assert_frame_closed(&self) {
        assert!(!self.in_frame, "end_frame was never called");
    }
}

impl RenderingServer for MockRenderer {
    fn begin_frame(&mut self, viewport: Rect) {
        assert!(!self.in_frame, "begin_frame called twice without end_frame");
        self.viewport = viewport;
        self.in_frame = true;
        self.clip.clear();
    }

    fn end_frame(&mut self) {
        assert!(self.in_frame, "end_frame called without begin_frame");
        self.in_frame = false;
    }

    fn set_clear_color(&mut self, color: Color) {
        self.calls.push(RecordedCall::SetClearColor(color));
    }

    fn viewport(&self) -> Rect {
        self.viewport
    }

    fn set_camera(&mut self, offset: Vec2) {
        self.camera_offset = offset;
        self.calls.push(RecordedCall::SetCamera(offset));
    }

    fn camera_offset(&self) -> Vec2 {
        self.camera_offset
    }

    fn set_z(&mut self, z: f32) {
        self.current_z = z;
        self.calls.push(RecordedCall::SetZ(z));
    }

    fn current_z(&self) -> f32 {
        self.current_z
    }

    fn push_clip_rect(&mut self, rect: Rect) {
        let effective = self.clip.push(rect);
        self.calls
            .push(RecordedCall::PushClipRect { rect, effective });
    }

    fn pop_clip_rect(&mut self) {
        self.clip.pop();
        self.calls.push(RecordedCall::PopClipRect);
    }

    fn clip_rect(&self) -> Option<Rect> {
        self.clip.current()
    }

    fn draw_rect(&mut self, rect: Rect, color: Color) {
        self.calls.push(RecordedCall::DrawRect {
            rect,
            color,
            z: self.current_z,
        });
    }

    fn draw_rect_outline(&mut self, rect: Rect, thickness: f32, color: Color) {
        self.calls.push(RecordedCall::DrawRectOutline {
            rect,
            thickness,
            color,
            z: self.current_z,
        });
    }

    fn draw_rect_rotated(&mut self, rect: Rect, rotation_radians: f32, color: Color) {
        self.calls.push(RecordedCall::DrawRectRotated {
            rect,
            rotation: rotation_radians,
            color,
            z: self.current_z,
        });
    }

    fn draw_circle(&mut self, center: Vec2, radius: f32, color: Color) {
        self.calls.push(RecordedCall::DrawCircle {
            center,
            radius,
            color,
            z: self.current_z,
        });
    }

    fn draw_circle_outline(&mut self, center: Vec2, radius: f32, thickness: f32, color: Color) {
        self.calls.push(RecordedCall::DrawCircleOutline {
            center,
            radius,
            thickness,
            color,
            z: self.current_z,
        });
    }

    fn draw_line(&mut self, a: Vec2, b: Vec2, thickness: f32, color: Color) {
        self.calls.push(RecordedCall::DrawLine {
            a,
            b,
            thickness,
            color,
            z: self.current_z,
        });
    }

    fn draw_triangle(&mut self, a: Vec2, b: Vec2, c: Vec2, color: Color) {
        self.calls.push(RecordedCall::DrawTriangle {
            a,
            b,
            c,
            color,
            z: self.current_z,
        });
    }

    fn draw_sprite(&mut self, texture: TextureId, dst: Rect, src: Option<Rect>, tint: Color) {
        self.calls.push(RecordedCall::DrawSprite {
            texture,
            dst,
            src,
            tint,
            z: self.current_z,
        });
    }

    fn draw_text(&mut self, text: &str, pos: Vec2, size: f32, color: Color) {
        self.calls.push(RecordedCall::DrawText {
            text: text.into(),
            pos,
            size,
            color,
            z: self.current_z,
        });
    }

    fn draw_text_with_font(
        &mut self,
        font: FontId,
        text: &str,
        pos: Vec2,
        size: f32,
        color: Color,
    ) {
        self.calls.push(RecordedCall::DrawTextWithFont {
            font,
            text: text.into(),
            pos,
            size,
            color,
            z: self.current_z,
        });
    }

    fn measure_text(&self, text: &str, size: f32) -> TextMetrics {
        TextMetrics {
            width: text.chars().count() as f32 * self.text_width_per_size * size,
            height: self.text_height_per_size * size,
            offset_y: self.text_height_per_size * size,
        }
    }

    fn measure_text_with_font(&self, _font: FontId, text: &str, size: f32) -> TextMetrics {
        self.measure_text(text, size)
    }

    fn set_default_font(&mut self, font: Option<FontId>) {
        self.default_font = font;
        self.calls.push(RecordedCall::SetDefaultFont(font));
    }

    fn create_texture(&mut self, _image: &Image) -> Result<TextureId, ServerError> {
        self.next_texture += 1;
        let id = TextureId::new(self.next_texture);
        self.calls.push(RecordedCall::CreateTexture(id));
        Ok(id)
    }

    fn destroy_texture(&mut self, id: TextureId) {
        self.calls.push(RecordedCall::DestroyTexture(id));
    }

    fn create_font(&mut self, _bytes: &[u8]) -> Result<FontId, ServerError> {
        self.next_font += 1;
        let id = FontId::new(self.next_font);
        self.calls.push(RecordedCall::CreateFont(id));
        Ok(id)
    }

    fn destroy_font(&mut self, id: FontId) {
        self.calls.push(RecordedCall::DestroyFont(id));
    }

    fn supports(&self, feature: RenderFeature) -> bool {
        self.capabilities.contains(&feature)
    }
}

// ---------------------------------------------------------------------------
// MockAudio
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum RecordedAudioCall {
    Play {
        clip: AudioClipId,
        volume: f32,
        pitch: f32,
        sound: SoundId,
    },
    Stop(SoundId),
    StopAll,
    SetMasterVolume(f32),
    Flush,
    LoadClip(AudioClipId),
    DestroyClip(AudioClipId),
}

#[derive(Debug, Clone)]
pub struct MockAudio {
    calls: Vec<RecordedAudioCall>,
    next_clip: u64,
    next_sound: u64,
    master_volume: f32,
    capabilities: HashSet<AudioFeature>,
}

impl MockAudio {
    pub fn new() -> Self {
        Self {
            calls: Vec::new(),
            next_clip: 0,
            next_sound: 0,
            master_volume: 1.0,
            capabilities: HashSet::new(),
        }
    }

    pub fn calls(&self) -> &[RecordedAudioCall] {
        &self.calls
    }

    pub fn take_calls(&mut self) -> Vec<RecordedAudioCall> {
        std::mem::take(&mut self.calls)
    }

    pub fn grant(&mut self, feature: AudioFeature) {
        self.capabilities.insert(feature);
    }
}

impl Default for MockAudio {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::AudioServer for MockAudio {
    fn play(&mut self, clip: AudioClipId, volume: f32, pitch: f32) -> SoundId {
        self.next_sound += 1;
        let sound = SoundId::new(self.next_sound);
        self.calls.push(RecordedAudioCall::Play {
            clip,
            volume,
            pitch,
            sound,
        });
        sound
    }

    fn stop(&mut self, sound: SoundId) {
        self.calls.push(RecordedAudioCall::Stop(sound));
    }

    fn stop_all(&mut self) {
        self.calls.push(RecordedAudioCall::StopAll);
    }

    fn set_master_volume(&mut self, volume: f32) {
        self.master_volume = volume.clamp(0.0, 1.0);
        self.calls
            .push(RecordedAudioCall::SetMasterVolume(self.master_volume));
    }

    fn master_volume(&self) -> f32 {
        self.master_volume
    }

    fn flush(&mut self) {
        self.calls.push(RecordedAudioCall::Flush);
    }

    fn load_clip(&mut self, _bytes: &[u8]) -> Result<AudioClipId, crate::ServerError> {
        self.next_clip += 1;
        let id = AudioClipId::new(self.next_clip);
        self.calls.push(RecordedAudioCall::LoadClip(id));
        Ok(id)
    }

    fn destroy_clip(&mut self, clip: AudioClipId) {
        self.calls.push(RecordedAudioCall::DestroyClip(clip));
    }

    fn supports(&self, feature: AudioFeature) -> bool {
        self.capabilities.contains(&feature)
    }
}
