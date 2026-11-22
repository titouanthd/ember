//! `RenderingServer` implementation on macroquad.
//!
//! Every method is a thin shim around a macroquad call. The few with
//! non-trivial logic are documented inline.
//!
//! ## Frame model
//!
//! `begin_frame` / `end_frame` are **markers only** — macroquad is
//! immediate-mode, so there's no batching to open or flush. The
//! caller (the `MainLoop`) is responsible for calling
//! `macroquad::window::next_frame().await` after `end_frame`.
//!
//! ## Font loading
//!
//! `create_font` blocks on `load_ttf_font_from_bytes` on native
//! targets. On wasm it returns `ServerError::NotSupported` — games
//! should load fonts before entering the loop (via macroquad directly)
//! and pass the loaded handle in. See ADR-020.

use std::collections::HashMap;

use ember_core::math::Rect;
use ember_core::{Color, Image};
use ember_servers::{
    ClipStack, FontId, RenderFeature, RenderingServer, ServerError, TextMetrics, TextureId,
};
use glam::Vec2;
use macroquad::prelude::*;

use crate::convert::{color_to_mq, rect_to_mq};

pub struct MacroquadRenderer {
    viewport: Rect,
    camera_offset: Vec2,
    current_z: f32,
    textures: HashMap<TextureId, Texture2D>,
    fonts: HashMap<FontId, Font>,
    next_texture: u64,
    next_font: u64,
    default_font: Option<FontId>,
    capabilities: Vec<RenderFeature>,
    clip: ClipStack,
}

impl MacroquadRenderer {
    pub fn new() -> Self {
        Self {
            viewport: Rect::default(),
            camera_offset: Vec2::ZERO,
            current_z: 0.0,
            textures: HashMap::new(),
            fonts: HashMap::new(),
            next_texture: 0,
            next_font: 0,
            default_font: None,
            capabilities: vec![RenderFeature::AlphaBlending, RenderFeature::CustomShaders],
            clip: ClipStack::new(),
        }
    }

    /// Adopt a font loaded externally (e.g., before the loop on wasm).
    /// See ADR-020.
    pub fn adopt_font(&mut self, font: Font) -> FontId {
        self.next_font += 1;
        let id = FontId::new(self.next_font);
        self.fonts.insert(id, font);
        id
    }

    fn texture(&self, id: TextureId) -> Result<&Texture2D, ServerError> {
        self.textures.get(&id).ok_or(ServerError::UnknownHandle)
    }

    fn font(&self, id: FontId) -> Result<&Font, ServerError> {
        self.fonts.get(&id).ok_or(ServerError::UnknownHandle)
    }

    /// Apply `clip` (screen pixels, top-left origin) as the GL scissor;
    /// `None` disables it. Same call as jade-garden's help modal.
    fn apply_scissor(clip: Option<Rect>) {
        let scissor = clip.map(|r| {
            let x0 = r.x.floor();
            let y0 = r.y.floor();
            let x1 = (r.x + r.w).ceil();
            let y1 = (r.y + r.h).ceil();
            (x0 as i32, y0 as i32, (x1 - x0) as i32, (y1 - y0) as i32)
        });
        // SAFETY: `get_internal_gl` is unsafe because it exposes macroquad's
        // internal GL state. We only set the scissor between draw calls, on
        // the main thread, and never hold the reference past this call.
        let gl = unsafe { get_internal_gl() };
        gl.quad_gl.scissor(scissor);
    }
}

impl Default for MacroquadRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderingServer for MacroquadRenderer {
    fn begin_frame(&mut self, viewport: Rect) {
        self.viewport = viewport;
        self.clip.clear();
        Self::apply_scissor(None);
    }

    fn end_frame(&mut self) {
        // The MainLoop calls next_frame().await. An unbalanced push must
        // not leak into the next frame or into macroquad's own drawing.
        if self.clip.depth() > 0 {
            self.clip.clear();
            Self::apply_scissor(None);
        }
    }

    fn set_clear_color(&mut self, color: Color) {
        clear_background(color_to_mq(color));
    }

    fn viewport(&self) -> Rect {
        self.viewport
    }

    fn set_camera(&mut self, offset: Vec2) {
        self.camera_offset = offset;
    }

    fn camera_offset(&self) -> Vec2 {
        self.camera_offset
    }

    fn set_z(&mut self, z: f32) {
        self.current_z = z;
    }

    fn current_z(&self) -> f32 {
        self.current_z
    }

    fn push_clip_rect(&mut self, rect: Rect) {
        let effective = self.clip.push(rect);
        Self::apply_scissor(Some(effective));
    }

    fn pop_clip_rect(&mut self) {
        let current = self.clip.pop();
        Self::apply_scissor(current);
    }

    fn clip_rect(&self) -> Option<Rect> {
        self.clip.current()
    }

    fn draw_rect(&mut self, rect: Rect, color: Color) {
        let o = self.camera_offset;
        draw_rectangle(
            rect.x - o.x,
            rect.y - o.y,
            rect.w,
            rect.h,
            color_to_mq(color),
        );
    }

    fn draw_rect_outline(&mut self, rect: Rect, thickness: f32, color: Color) {
        let o = self.camera_offset;
        draw_rectangle_lines(
            rect.x - o.x,
            rect.y - o.y,
            rect.w,
            rect.h,
            thickness,
            color_to_mq(color),
        );
    }

    fn draw_rect_rotated(&mut self, rect: Rect, rotation_radians: f32, color: Color) {
        let o = self.camera_offset;
        draw_rectangle_ex(
            rect.x - o.x,
            rect.y - o.y,
            rect.w,
            rect.h,
            DrawRectangleParams {
                offset: vec2(0.5, 0.5),
                rotation: rotation_radians,
                color: color_to_mq(color),
            },
        );
    }

    fn draw_circle(&mut self, center: Vec2, radius: f32, color: Color) {
        let o = self.camera_offset;
        draw_circle(center.x - o.x, center.y - o.y, radius, color_to_mq(color));
    }

    fn draw_circle_outline(&mut self, center: Vec2, radius: f32, thickness: f32, color: Color) {
        let o = self.camera_offset;
        draw_circle_lines(
            center.x - o.x,
            center.y - o.y,
            radius,
            thickness,
            color_to_mq(color),
        );
    }

    fn draw_line(&mut self, a: Vec2, b: Vec2, thickness: f32, color: Color) {
        let o = self.camera_offset;
        draw_line(
            a.x - o.x,
            a.y - o.y,
            b.x - o.x,
            b.y - o.y,
            thickness,
            color_to_mq(color),
        );
    }

    fn draw_triangle(&mut self, a: Vec2, b: Vec2, c: Vec2, color: Color) {
        let o = self.camera_offset;
        draw_triangle(
            vec2(a.x - o.x, a.y - o.y),
            vec2(b.x - o.x, b.y - o.y),
            vec2(c.x - o.x, c.y - o.y),
            color_to_mq(color),
        );
    }

    fn draw_sprite(&mut self, texture: TextureId, dst: Rect, src: Option<Rect>, tint: Color) {
        let Ok(tex) = self.texture(texture) else {
            return;
        };
        let o = self.camera_offset;
        let dst_mq = Rect::new(dst.x - o.x, dst.y - o.y, dst.w, dst.h);
        let src_mq = src.map(rect_to_mq);
        draw_texture_ex(
            tex,
            dst_mq.x,
            dst_mq.y,
            color_to_mq(tint),
            DrawTextureParams {
                dest_size: Some(vec2(dst_mq.w, dst_mq.h)),
                source: src_mq,
                ..Default::default()
            },
        );
    }

    fn draw_text(&mut self, text: &str, pos: Vec2, size: f32, color: Color) {
        let o = self.camera_offset;
        if let Some(font_id) = self.default_font
            && let Ok(font) = self.font(font_id)
        {
            draw_text_ex(
                text,
                pos.x - o.x,
                pos.y - o.y,
                TextParams {
                    font: Some(font),
                    font_size: size as u16,
                    color: color_to_mq(color),
                    ..Default::default()
                },
            );
            return;
        }
        draw_text(text, pos.x - o.x, pos.y - o.y, size, color_to_mq(color));
    }

    fn draw_text_with_font(
        &mut self,
        font: FontId,
        text: &str,
        pos: Vec2,
        size: f32,
        color: Color,
    ) {
        let o = self.camera_offset;
        let Ok(font_ref) = self.font(font) else {
            return;
        };
        draw_text_ex(
            text,
            pos.x - o.x,
            pos.y - o.y,
            TextParams {
                font: Some(font_ref),
                font_size: size as u16,
                color: color_to_mq(color),
                ..Default::default()
            },
        );
    }

    fn measure_text(&self, text: &str, size: f32) -> TextMetrics {
        let dims = match self.default_font.and_then(|id| self.fonts.get(&id)) {
            Some(font) => measure_text(text, Some(font), size as u16, 1.0),
            None => measure_text(text, None, size as u16, 1.0),
        };
        TextMetrics {
            width: dims.width,
            height: dims.height,
            offset_y: dims.offset_y,
        }
    }

    fn measure_text_with_font(&self, font: FontId, text: &str, size: f32) -> TextMetrics {
        let Ok(font_ref) = self.font(font) else {
            return self.measure_text(text, size);
        };
        let dims = measure_text(text, Some(font_ref), size as u16, 1.0);
        TextMetrics {
            width: dims.width,
            height: dims.height,
            offset_y: dims.offset_y,
        }
    }

    fn set_default_font(&mut self, font: Option<FontId>) {
        self.default_font = font;
    }

    fn create_texture(&mut self, image: &Image) -> Result<TextureId, ServerError> {
        if image.width() == 0 || image.height() == 0 {
            return Err(ServerError::Backend("empty image".into()));
        }
        let texture = Texture2D::from_rgba8(
            image.width() as u16,
            image.height() as u16,
            image.as_bytes(),
        );
        self.next_texture += 1;
        let id = TextureId::new(self.next_texture);
        self.textures.insert(id, texture);
        Ok(id)
    }

    fn destroy_texture(&mut self, id: TextureId) {
        self.textures.remove(&id);
    }

    fn create_font(&mut self, bytes: &[u8]) -> Result<FontId, ServerError> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = bytes;
            return Err(ServerError::NotSupported);
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let font = load_ttf_font_from_bytes(bytes)
                .map_err(|e| ServerError::Backend(format!("font loading failed: {e}")))?;
            self.next_font += 1;
            let id = FontId::new(self.next_font);
            self.fonts.insert(id, font);
            Ok(id)
        }
    }

    fn destroy_font(&mut self, id: FontId) {
        self.fonts.remove(&id);
        if self.default_font == Some(id) {
            self.default_font = None;
        }
    }

    fn supports(&self, feature: RenderFeature) -> bool {
        self.capabilities.contains(&feature)
    }
}
