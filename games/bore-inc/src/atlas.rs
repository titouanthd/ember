//! Sprite atlas. Sprites are drawn into `Canvas` buffers at startup,
//! then baked into a single RGBA texture. Rendering is a series of
//! `draw_texture_rec` calls — one texture binding, thousands of sprites
//! per frame if needed.
//!
//! **Layout**: fixed grid, cells of 32×32 pixels, 16 columns. Sprites
//! smaller than a cell are top-left aligned inside their cell. The
//! texture is `Nearest`-filtered so pixels stay crisp.

use std::collections::HashMap;

use macroquad::prelude::*;

use crate::pixel::Canvas;

/// Every sprite is padded to this cell size in the atlas.
const CELL: u32 = 48;
/// Number of columns in the grid.
const COLS: u32 = 16;

/// Position of one sprite inside the atlas texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpriteRect {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

impl SpriteRect {
    pub fn as_rect(self) -> Rect {
        Rect::new(self.x as f32, self.y as f32, self.w as f32, self.h as f32)
    }
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// Accumulates sprites. Call `add` for each one, then `bake` to get a
/// pure pixel buffer, or `bake().upload()` to get a GPU texture.
pub struct AtlasBuilder {
    sprites: Vec<(String, Canvas)>,
    seen: HashMap<String, ()>,
}

impl AtlasBuilder {
    pub fn new() -> Self {
        Self {
            sprites: Vec::new(),
            seen: HashMap::new(),
        }
    }

    /// Add a sprite. Returns the `SpriteRect` it will occupy in the
    /// baked atlas.
    ///
    /// Panics if the name is a duplicate or the canvas doesn't fit in
    /// a cell.
    pub fn add(&mut self, name: &str, canvas: Canvas) -> SpriteRect {
        assert!(
            canvas.width as u32 <= CELL && canvas.height as u32 <= CELL,
            "sprite {name} too large: {}×{} (max {CELL}×{CELL})",
            canvas.width,
            canvas.height,
        );
        assert!(!self.seen.contains_key(name), "duplicate sprite name: {name}");

        let i = self.sprites.len() as u32;
        let col = i % COLS;
        let row = i / COLS;
        let rect = SpriteRect {
            x: (col * CELL) as u16,
            y: (row * CELL) as u16,
            w: canvas.width,
            h: canvas.height,
        };

        self.seen.insert(name.to_string(), ());
        self.sprites.push((name.to_string(), canvas));
        rect
    }

    /// Bake into a flat pixel buffer. Pure — no macroquad needed, so
    /// it can be tested headless.
    pub fn bake(self) -> BakedAtlas {
        let n = self.sprites.len() as u32;
        let cols = COLS.min(n.max(1));
        let rows = n.div_ceil(cols).max(1);
        let width = cols * CELL;
        let height = rows * CELL;

        let mut pixels = vec![0u8; (width * height * 4) as usize];
        let mut rects = HashMap::new();

        for (i, (name, canvas)) in self.sprites.iter().enumerate() {
            let i = i as u32;
            let col = i % COLS;
            let row = i / COLS;
            let cell_x = col * CELL;
            let cell_y = row * CELL;

            for cy in 0..canvas.height as u32 {
                for cx in 0..canvas.width as u32 {
                    let src = ((cy * canvas.width as u32 + cx) * 4) as usize;
                    let dst = (((cell_y + cy) * width + (cell_x + cx)) * 4) as usize;
                    pixels[dst..dst + 4].copy_from_slice(&canvas.pixels[src..src + 4]);
                }
            }

            rects.insert(
                name.clone(),
                SpriteRect {
                    x: cell_x as u16,
                    y: cell_y as u16,
                    w: canvas.width,
                    h: canvas.height,
                },
            );
        }

        BakedAtlas {
            width: width as u16,
            height: height as u16,
            pixels,
            rects,
        }
    }
}

impl Default for AtlasBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Baked (CPU-side) atlas
// ---------------------------------------------------------------------------

/// A CPU-side atlas, ready to be uploaded as a `Texture2D`.
#[derive(Debug, Clone)]
pub struct BakedAtlas {
    pub width: u16,
    pub height: u16,
    pub pixels: Vec<u8>,
    pub rects: HashMap<String, SpriteRect>,
}

impl BakedAtlas {
    /// Upload to the GPU. **Requires a macroquad context.**
    pub fn upload(self) -> Atlas {
        let texture = Texture2D::from_rgba8(self.width, self.height, &self.pixels);
        texture.set_filter(FilterMode::Nearest);
        Atlas {
            texture,
            width: self.width,
            height: self.height,
            rects: self.rects,
        }
    }
}

// ---------------------------------------------------------------------------
// GPU-side atlas
// ---------------------------------------------------------------------------

/// A loaded atlas texture plus a name → rect lookup.
pub struct Atlas {
    pub texture: Texture2D,
    pub width: u16,
    pub height: u16,
    pub rects: HashMap<String, SpriteRect>,
}

impl Atlas {
    pub fn rect(&self, name: &str) -> Option<SpriteRect> {
        self.rects.get(name).copied()
    }

    pub fn has(&self, name: &str) -> bool {
        self.rects.contains_key(name)
    }

    /// Draw a sprite at native size, top-left anchored.
    pub fn draw(&self, name: &str, x: f32, y: f32) {
        if let Some(r) = self.rect(name) {
            draw_texture_ex(
                &self.texture,
                x,
                y,
                WHITE,
                DrawTextureParams {
                    source: Some(r.as_rect()),
                    ..Default::default()
                },
            );
        }
    }

    /// Draw with a colour tint (multiplied against the source).
    pub fn draw_tinted(&self, name: &str, x: f32, y: f32, tint: Color) {
        if let Some(r) = self.rect(name) {
            draw_texture_ex(
                &self.texture,
                x,
                y,
                tint,
                DrawTextureParams {
                    source: Some(r.as_rect()),
                    ..Default::default()
                },
            );
        }
    }

    /// Draw scaled. `scale = 2.0` doubles size; pixel art looks best
    /// with integer scales.
    pub fn draw_scaled(&self, name: &str, x: f32, y: f32, scale: f32) {
        if let Some(r) = self.rect(name) {
            draw_texture_ex(
                &self.texture,
                x,
                y,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(Vec2::new(r.w as f32 * scale, r.h as f32 * scale)),
                    source: Some(r.as_rect()),
                    ..Default::default()
                },
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: u16, h: u16, color: [u8; 4]) -> Canvas {
        let mut c = Canvas::new(w, h);
        c.fill(color);
        c
    }

    #[test]
    fn add_returns_a_rect_matching_the_canvas() {
        let mut b = AtlasBuilder::new();
        let r = b.add("test", solid(16, 16, [255, 0, 0, 255]));
        assert_eq!(r.x, 0);
        assert_eq!(r.y, 0);
        assert_eq!(r.w, 16);
        assert_eq!(r.h, 16);
    }

    #[test]
    fn add_places_sprites_in_a_grid() {
        let mut b = AtlasBuilder::new();
        let r0 = b.add("a", solid(8, 8, [255, 0, 0, 255]));
        let r1 = b.add("b", solid(8, 8, [0, 255, 0, 255]));
        assert_eq!(r1.x, r0.x + CELL as u16);
        assert_eq!(r1.y, r0.y);
    }

    #[test]
    fn add_wraps_to_the_next_row() {
        let mut b = AtlasBuilder::new();
        for i in 0..COLS {
            b.add(&format!("s{i}"), solid(8, 8, [0, 0, 0, 255]));
        }
        let r = b.add("wrapped", solid(8, 8, [0, 0, 0, 255]));
        assert_eq!(r.x, 0);
        assert_eq!(r.y, CELL as u16);
    }

    #[test]
    #[should_panic(expected = "duplicate")]
    fn duplicate_name_panics() {
        let mut b = AtlasBuilder::new();
        b.add("x", solid(8, 8, [0, 0, 0, 255]));
        b.add("x", solid(8, 8, [0, 0, 0, 255]));
    }

    #[test]
    #[should_panic(expected = "too large")]
    fn oversized_sprite_panics() {
        let mut b = AtlasBuilder::new();
        b.add("big", solid(64, 64, [0, 0, 0, 255]));
    }

    #[test]
    fn bake_produces_expected_dimensions() {
        let mut b = AtlasBuilder::new();
        for i in 0..COLS {
            b.add(&format!("s{i}"), solid(8, 8, [0, 0, 0, 255]));
        }
        let baked = b.bake();
        assert_eq!(baked.width, (COLS * CELL) as u16);
        assert_eq!(baked.height, CELL as u16);
    }

    #[test]
    fn bake_empty_atlas_is_one_cell() {
        let baked = AtlasBuilder::new().bake();
        assert_eq!(baked.width, CELL as u16);
        assert_eq!(baked.height, CELL as u16);
        assert!(baked.rects.is_empty());
        assert_eq!(baked.pixels.len(), (CELL * CELL * 4) as usize);
    }

    #[test]
    fn bake_blits_pixels_in_the_right_place() {
        let mut b = AtlasBuilder::new();
        b.add("red", solid(4, 4, [255, 0, 0, 255]));
        let baked = b.bake();
        let r = baked.rects.get("red").unwrap();
        let idx = ((r.y as u32 * baked.width as u32 + r.x as u32) * 4) as usize;
        assert_eq!(&baked.pixels[idx..idx + 4], &[255, 0, 0, 255]);
        // And one pixel past the 4×4 block should still be 0.
        let past = idx + 4 * 4;
        assert_eq!(&baked.pixels[past..past + 4], &[0, 0, 0, 0]);
    }

    #[test]
    fn bake_rects_do_not_overlap() {
        let mut b = AtlasBuilder::new();
        for i in 0..40 {
            b.add(&format!("s{i}"), solid(16, 16, [0, 0, 0, 255]));
        }
        let baked = b.bake();
        let rects: Vec<SpriteRect> = baked.rects.values().copied().collect();
        for (i, a) in rects.iter().enumerate() {
            for b in rects.iter().skip(i + 1) {
                let overlap_x = a.x < b.x + b.w && a.x + a.w > b.x;
                let overlap_y = a.y < b.y + b.h && a.y + a.h > b.y;
                assert!(
                    !(overlap_x && overlap_y),
                    "rects overlap: {a:?} and {b:?}",
                );
            }
        }
    }

    #[test]
    fn bake_rects_match_add_return_values() {
        let mut b = AtlasBuilder::new();
        let r0 = b.add("a", solid(12, 14, [0, 0, 0, 255]));
        let r1 = b.add("b", solid(20, 10, [0, 0, 0, 255]));
        let baked = b.bake();
        assert_eq!(baked.rects.get("a"), Some(&r0));
        assert_eq!(baked.rects.get("b"), Some(&r1));
    }
}