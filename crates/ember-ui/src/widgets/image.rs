//! `Image`: a texture drawn in the control's rect (spec §5.5).

use crate::{Control, ControlBase, Measure, UiDrawContext};
use ember_core::Color;
use ember_core::math::Rect;
use ember_servers::TextureId;
use glam::Vec2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImageMode {
    /// Stretch to the rect; may distort.
    Fill,
    /// Largest size that fits, aspect kept, centred (letterbox).
    #[default]
    AspectFit,
    /// Cover the whole rect, aspect kept, overflow cropped.
    AspectFill,
}

/// Takes a backend `TextureId` and the texture's pixel size, e.g. from
/// an `ember_assets::Texture` (ADR-034: `ember-ui` does not depend on
/// `ember-assets`). Its desired size is the texture size.
pub struct Image {
    name: String,
    base: ControlBase,
    pub texture: TextureId,
    pub texture_size: Vec2,
    pub mode: ImageMode,
    pub tint: Color,
}

impl Image {
    pub fn new(texture: TextureId, texture_size: Vec2) -> Self {
        Self {
            name: "Image".into(),
            base: ControlBase::new(),
            texture,
            texture_size,
            mode: ImageMode::AspectFit,
            tint: Color::WHITE,
        }
    }

    pub fn with_mode(mut self, mode: ImageMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_tint(mut self, tint: Color) -> Self {
        self.tint = tint;
        self
    }

    /// Destination rect on screen and source rect in the texture (in
    /// pixels; `None` = whole texture) for drawing into `rect`.
    pub fn placement(&self, rect: Rect) -> (Rect, Option<Rect>) {
        let (tw, th) = (self.texture_size.x, self.texture_size.y);
        if tw <= 0.0 || th <= 0.0 || rect.w <= 0.0 || rect.h <= 0.0 {
            return (rect, None);
        }
        match self.mode {
            ImageMode::Fill => (rect, None),
            ImageMode::AspectFit => {
                let scale = (rect.w / tw).min(rect.h / th);
                let (w, h) = (tw * scale, th * scale);
                let dst = Rect::new(
                    rect.x + (rect.w - w) / 2.0,
                    rect.y + (rect.h - h) / 2.0,
                    w,
                    h,
                );
                (dst, None)
            }
            ImageMode::AspectFill => {
                let scale = (rect.w / tw).max(rect.h / th);
                let (vw, vh) = (rect.w / scale, rect.h / scale);
                let src = Rect::new((tw - vw) / 2.0, (th - vh) / 2.0, vw, vh);
                (rect, Some(src))
            }
        }
    }
}

crate::widgets::ui_object!(Image, "Image");

impl Control for Image {
    fn base(&self) -> &ControlBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ControlBase {
        &mut self.base
    }

    fn measure(&self, _available: Vec2, _m: &Measure) -> Vec2 {
        self.texture_size.max(self.min_size())
    }

    fn draw_control(&self, ui: &mut UiDrawContext) {
        let (dst, src) = self.placement(self.rect());
        ui.render().draw_sprite(self.texture, dst, src, self.tint);
    }
}

crate::control_node!(Image);

#[cfg(test)]
mod tests {
    use super::*;

    fn img(mode: ImageMode) -> Image {
        Image::new(TextureId::new(1), Vec2::new(200.0, 100.0)).with_mode(mode)
    }

    const BOX: Rect = Rect::new(0.0, 0.0, 100.0, 100.0);

    #[test]
    fn fill_uses_the_whole_rect() {
        assert_eq!(img(ImageMode::Fill).placement(BOX), (BOX, None));
    }

    #[test]
    fn aspect_fit_letterboxes() {
        let (dst, src) = img(ImageMode::AspectFit).placement(BOX);
        assert_eq!(dst, Rect::new(0.0, 25.0, 100.0, 50.0));
        assert_eq!(src, None);
    }

    #[test]
    fn aspect_fill_crops_the_centre() {
        let (dst, src) = img(ImageMode::AspectFill).placement(BOX);
        assert_eq!(dst, BOX);
        assert_eq!(src, Some(Rect::new(50.0, 0.0, 100.0, 100.0)));
    }

    #[test]
    fn empty_texture_draws_in_the_rect() {
        let i = Image::new(TextureId::new(1), Vec2::ZERO);
        assert_eq!(i.placement(BOX), (BOX, None));
    }
}
