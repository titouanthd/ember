//! Archie's sprites: CraftPix sheet loader.
//!
//! `assets/archie.png` is 256×320 — 8 columns × 10 rows of 32×32
//! cells. Row meanings (0-indexed), corrected from the pack README
//! after visual inspection:
//!
//! ```text
//!   0 = idle front   1 = idle profile   2 = lick paw   3 = clean
//!   4 = walk         5 = run            6 = sleep      7 = play (hit)
//!   8 = jump         9 = scare
//! ```
//!
//! We use rows 1 (idle), 7 (mine), 4 (walk), 8 (jump). L variants
//! are mirrored at bake time. Frame counts are auto-detected: the
//! loader walks each row from column 0 and stops at the first fully
//! transparent cell (alpha == 0, RGB ignored).
//!
//! The bag is baked separately (`sprites::bag`) and exposed here as
//! `bag_r` / `bag_l` so `Game::draw` can find it next to the cat
//! sprites without extra plumbing.

use macroquad::texture::Image;

use crate::atlas::{AtlasBuilder, SpriteRect};
use crate::pixel::Canvas;
use crate::sprites::bag;

/// Square cell size in the sheet, in pixels.
pub const SHEET_CELL: usize = 32;

/// Offset added to Archie's physics position when drawing the sheet
/// sprite. Bottom-aligns the 32×32 cell to the 12×14 physics box.
pub const SHEET_OFFSET_X: f32 = (SHEET_CELL as f32 - crate::player::ARCHIE_WIDTH) * -0.5;
pub const SHEET_OFFSET_Y: f32 = crate::player::ARCHIE_HEIGHT - SHEET_CELL as f32;

// ---------- Baked sprite set ----------

#[derive(Debug, Clone)]
pub struct ArchieSprites {
    pub idle_r: Vec<SpriteRect>,
    pub idle_l: Vec<SpriteRect>,
    pub walk_r: Vec<SpriteRect>,
    pub walk_l: Vec<SpriteRect>,
    pub jump_r: Vec<SpriteRect>,
    pub jump_l: Vec<SpriteRect>,
    pub mine_r: Vec<SpriteRect>,
    pub mine_l: Vec<SpriteRect>,

    /// Bag sprites, indexed by fill level. `bag_r[i]` is the R-facing
    /// bag at level `i` of `bag::LEVELS`.
    pub bag_r: Vec<SpriteRect>,
    pub bag_l: Vec<SpriteRect>,

    /// (x, y) added to Archie's physics position when drawing.
    pub offset: (f32, f32),
}

// ---------- Sheet loading ----------

/// Load `assets/<filename>` into an `Image`. Async on wasm (fetch).
pub async fn load_sheet(filename: &str) -> Result<Image, String> {
    let path = crate::paths::asset_str(filename);
    macroquad::texture::load_image(&path)
        .await
        .map_err(|e| format!("{e:?}"))
}

/// Extract one 32×32 cell from `image` at `(col, row)`.
fn extract_cell(image: &Image, col: usize, row: usize) -> Canvas {
    let mut canvas = Canvas::new(SHEET_CELL as u16, SHEET_CELL as u16);
    let stride = image.width() * 4;
    let bytes = &image.bytes;
    let x0 = col * SHEET_CELL;
    let y0 = row * SHEET_CELL;
    for y in 0..SHEET_CELL {
        for x in 0..SHEET_CELL {
            let src = (y0 + y) * stride + (x0 + x) * 4;
            let rgba = [bytes[src], bytes[src + 1], bytes[src + 2], bytes[src + 3]];
            canvas.set(x as i32, y as i32, rgba);
        }
    }
    canvas
}

/// Read one animation row: from column 0, stop at the first empty
/// cell. Produces R and mirrored L rects under the same name
/// convention used across the codebase.
fn bake_sheet_anim(
    builder: &mut AtlasBuilder,
    image: &Image,
    prefix: &str,
    row: usize,
    max_frames: usize,
) -> (Vec<SpriteRect>, Vec<SpriteRect>) {
    let cols = image.width() / SHEET_CELL;
    let limit = max_frames.min(cols);
    let mut r = Vec::new();
    let mut l = Vec::new();

    // Diagnostic : par frame, on imprime le nombre de pixels opaques
    // et le bounding box. Un frame avec `px` très bas ou un `y0`
    // anormal est un candidat au flash.
    println!("[bore-inc] {prefix} row={row}  col px bbox(x0,y0)..(x1,y1):");

    for col in 0..limit {
        let canvas = extract_cell(image, col, row);
        if canvas.is_empty() {
            break;
        }

        let mut px = 0usize;
        let (mut x0, mut y0) = (SHEET_CELL as i32, SHEET_CELL as i32);
        let (mut x1, mut y1) = (-1i32, -1i32);
        for y in 0..SHEET_CELL as i32 {
            for x in 0..SHEET_CELL as i32 {
                if canvas.get(x, y).is_some_and(|p| p[3] > 0) {
                    px += 1;
                    x0 = x0.min(x);
                    y0 = y0.min(y);
                    x1 = x1.max(x);
                    y1 = y1.max(y);
                }
            }
        }
        println!("    col={col}  px={px:4}  bbox=({x0},{y0})..({x1},{y1})");

        r.push(builder.add(&format!("archie_{prefix}_{col}_R"), canvas.clone()));
        l.push(builder.add(&format!("archie_{prefix}_{col}_L"), canvas.mirrored_x()));
    }
    (r, l)
}

/// Bake sprites from the loaded sheet image.
pub fn bake_from_image(builder: &mut AtlasBuilder, image: &Image) -> ArchieSprites {
    let (idle_r, idle_l) = bake_sheet_anim(builder, image, "idle", 1, 8);
    let (walk_r, walk_l) = bake_sheet_anim(builder, image, "walk", 4, 8);
    let (jump_r, jump_l) = bake_sheet_anim(builder, image, "jump", 8, 8);
    let (mine_r, mine_l) = bake_sheet_anim(builder, image, "mine", 7, 8);

    let (bag_r, bag_l) = bag::bake_into(builder);

    println!(
        "[bore-inc] archie frames: idle={} walk={} jump={} mine={} bag={}",
        idle_r.len(),
        walk_r.len(),
        jump_r.len(),
        mine_r.len(),
        bag_r.len(),
    );

    ArchieSprites {
        idle_r,
        idle_l,
        walk_r,
        walk_l,
        jump_r,
        jump_l,
        mine_r,
        mine_l,
        bag_r,
        bag_l,
        offset: (SHEET_OFFSET_X, SHEET_OFFSET_Y),
    }
}

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sheet_cell_is_32() {
        assert_eq!(SHEET_CELL, 32);
    }

    #[test]
    fn offsets_are_negative() {
        const { assert!(SHEET_OFFSET_X < 0.0) };
        const { assert!(SHEET_OFFSET_Y < 0.0) };
    }

    #[test]
    fn extract_cell_fully_transparent_image_reads_empty() {
        let img = Image {
            bytes: vec![0; SHEET_CELL * SHEET_CELL * 4],
            width: SHEET_CELL as u16,
            height: SHEET_CELL as u16,
        };
        let cell = extract_cell(&img, 0, 0);
        assert_eq!(cell.width, SHEET_CELL as u16);
        assert!(cell.is_empty());
    }

    #[test]
    fn extract_cell_white_rgb_alpha_zero_reads_empty() {
        let mut bytes = vec![0u8; SHEET_CELL * SHEET_CELL * 4];
        for px in bytes.chunks_exact_mut(4) {
            px[0] = 255;
            px[1] = 255;
            px[2] = 255;
            px[3] = 0;
        }
        let img = Image {
            bytes,
            width: SHEET_CELL as u16,
            height: SHEET_CELL as u16,
        };
        let cell = extract_cell(&img, 0, 0);
        assert!(cell.is_empty());
    }

    #[test]
    fn extract_cell_copies_pixels() {
        let mut bytes = vec![0u8; SHEET_CELL * SHEET_CELL * 4];
        bytes[0] = 0xFF;
        bytes[1] = 0x80;
        bytes[2] = 0x40;
        bytes[3] = 0xFF;
        let img = Image {
            bytes,
            width: SHEET_CELL as u16,
            height: SHEET_CELL as u16,
        };
        let cell = extract_cell(&img, 0, 0);
        assert_eq!(cell.get(0, 0), Some([0xFF, 0x80, 0x40, 0xFF]));
    }
}