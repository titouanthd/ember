//! The rendering trait. Every draw call in every game goes through this.
//!
//! ## Naming
//!
//! Per `02-principles.md §3.2`:
//! - `draw_*` — immediate; resets at `end_frame`.
//! - `set_*` — persistent state (camera, z, clear color, default font).
//! - `create_*` / `destroy_*` — resource lifecycle.
//! - `supports` — capability probe.
//!
//! ## Method set
//!
//! The list in `12-servers.md §4.1` is the floor. Three methods were
//! added when reviewing the games (see `50-design-decisions.md` when
//! Phase 6+ lands):
//! - `draw_triangle` — Pagoda's `CircleButton` tessellates manually to
//!   avoid the fill/border mismatch in macroquad's built-in circle draw.
//! - `draw_circle_outline` — shockwave rings, arc-style UI decorations.
//! - `draw_rect_rotated` — Pagoda's falling slice pieces (juice) rotate
//!   as they fall.

use crate::{Camera2d, FontId, RenderFeature, ServerError, TextMetrics, TextureId};
use ember_core::math::Rect;
use ember_core::{Color, Image};
use glam::Vec2;

pub trait RenderingServer {
    // --- Frame lifecycle ---

    /// Open a batching window for one frame. Called once per frame by
    /// `MainLoop` before any `draw_*` call.
    fn begin_frame(&mut self, viewport: Rect);

    /// Close the batching window and flush to the display.
    fn end_frame(&mut self);

    /// Background color for pixels not drawn over this frame.
    fn set_clear_color(&mut self, color: Color);

    /// The viewport passed to the current frame's `begin_frame`.
    fn viewport(&self) -> Rect;

    // --- Camera ---

    /// The camera for all subsequent draws until changed (ADR-040): a
    /// point at world `p` lands at `(p - offset) * zoom` on screen, and
    /// sizes, radii, thicknesses and font sizes scale by `zoom`
    /// (`Camera2d::world_to_screen`). Clip rects ignore it.
    fn set_camera(&mut self, camera: Camera2d);
    fn camera(&self) -> Camera2d;

    // --- Z ordering ---

    /// Z used by all subsequent draws until changed. `draw_*` calls
    /// preserve call order within the same z.
    fn set_z(&mut self, z: f32);
    fn current_z(&self) -> f32;

    // --- Clipping (ADR-031) ---

    /// Restrict drawing to `rect` until the matching
    /// [`pop_clip_rect`](Self::pop_clip_rect). `rect` is in **screen
    /// pixels** (top-left origin) and is not moved by the camera. Nested
    /// pushes intersect with the current clip; a push outside it clips
    /// everything. `begin_frame` clears the stack. Rules: [`crate::ClipStack`].
    fn push_clip_rect(&mut self, rect: Rect);

    /// Restore the clip that was active before the matching push.
    /// Popping an empty stack is ignored.
    fn pop_clip_rect(&mut self);

    /// The clip in effect, `None` when drawing is unclipped.
    fn clip_rect(&self) -> Option<Rect>;

    // --- Primitives ---

    fn draw_rect(&mut self, rect: Rect, color: Color);
    fn draw_rect_outline(&mut self, rect: Rect, thickness: f32, color: Color);
    fn draw_rect_rotated(&mut self, rect: Rect, rotation_radians: f32, color: Color);
    fn draw_circle(&mut self, center: Vec2, radius: f32, color: Color);
    fn draw_circle_outline(&mut self, center: Vec2, radius: f32, thickness: f32, color: Color);
    fn draw_line(&mut self, a: Vec2, b: Vec2, thickness: f32, color: Color);
    fn draw_triangle(&mut self, a: Vec2, b: Vec2, c: Vec2, color: Color);

    // --- Sprites ---

    /// Draw `texture` into `dst`. `src = None` means "the whole texture".
    /// `tint` multiplies every channel (use `Color::WHITE` for no tint).
    fn draw_sprite(&mut self, texture: TextureId, dst: Rect, src: Option<Rect>, tint: Color);

    // --- Text ---

    /// Draw text using the current default font.
    fn draw_text(&mut self, text: &str, pos: Vec2, size: f32, color: Color);

    /// Draw text using a specific font.
    fn draw_text_with_font(&mut self, font: FontId, text: &str, pos: Vec2, size: f32, color: Color);

    /// Measure text with the current default font.
    fn measure_text(&self, text: &str, size: f32) -> TextMetrics;

    /// Measure text with a specific font.
    fn measure_text_with_font(&self, font: FontId, text: &str, size: f32) -> TextMetrics;

    /// Use `font` for future `draw_text` / `measure_text` calls until
    /// changed. `None` reverts to the backend's built-in default.
    fn set_default_font(&mut self, font: Option<FontId>);

    // --- Textures and fonts ---

    fn create_texture(&mut self, image: &Image) -> Result<TextureId, ServerError>;
    fn destroy_texture(&mut self, id: TextureId);
    fn create_font(&mut self, bytes: &[u8]) -> Result<FontId, ServerError>;
    fn destroy_font(&mut self, id: FontId);

    // --- Capability probing ---

    fn supports(&self, feature: RenderFeature) -> bool;
}
