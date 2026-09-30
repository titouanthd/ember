//! Base types: the six jades, a tile, and its metadata.
//!
//! No `macroquad` here — this module is pure data and can be tested
//! headless.

use serde::Deserialize;

pub const GRID_W: usize = 8;
pub const GRID_H: usize = 8;
pub const JADE_TYPES: usize = 6;

/// Silhouette of a jade. Each type has a distinct shape so it reads
/// at a glance even without colour — big accessibility win.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JadeShape {
    /// Round polished pebble.
    Round,
    /// Hexagonal cut (flat-top).
    Hexagon,
    /// Diamond cut (points up/down/left/right).
    Diamond,
    /// Octagonal cut.
    Octagon,
    /// Pentagonal cut (points up).
    Pentagon,
    /// Rounded square (raw block).
    Square,
}

/// The six jade types, each with a cultural meaning.
///
/// The variant order determines `Jade::index()` — do not reorder
/// without updating tests and palettes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
pub enum Jade {
    /// 白玉 — white jade, purity.
    Bai,
    /// 碧玉 — green jade, harmony.
    Bi,
    /// 青玉 — blue jade, spirit.
    Qing,
    /// 红玉 — red jade, celebration.
    Hong,
    /// 黄玉 — yellow jade, prosperity.
    Huang,
    /// 墨玉 — black jade, mystery.
    Mo,
}

impl Jade {
    pub const ALL: [Jade; JADE_TYPES] = [
        Jade::Bai,
        Jade::Bi,
        Jade::Qing,
        Jade::Hong,
        Jade::Huang,
        Jade::Mo,
    ];

    /// Distinct silhouette for this jade type.
    pub fn shape(self) -> JadeShape {
        match self {
            Jade::Bai => JadeShape::Round,
            Jade::Bi => JadeShape::Hexagon,
            Jade::Qing => JadeShape::Diamond,
            Jade::Hong => JadeShape::Octagon,
            Jade::Huang => JadeShape::Pentagon,
            Jade::Mo => JadeShape::Square,
        }
    }

    /// Stable index 0..6, used for colour tables, seeds and tests.
    pub fn index(self) -> usize {
        self as usize
    }

    /// Lookup by index. Returns `None` if out of bounds.
    pub fn from_index(i: usize) -> Option<Jade> {
        Jade::ALL.get(i).copied()
    }

    /// RGB colour (0-255) for procedural rendering.
    pub fn base_rgb(self) -> (u8, u8, u8) {
        match self {
            Jade::Bai => (0xF5, 0xEF, 0xE0),
            Jade::Bi => (0x3D, 0x9B, 0x6A),
            Jade::Qing => (0x4A, 0x7B, 0xC4),
            Jade::Hong => (0xB2, 0x3A, 0x38),
            Jade::Huang => (0xD4, 0xA5, 0x4E),
            Jade::Mo => (0x2A, 0x2A, 0x2E),
        }
    }

    /// Traditional Chinese name (白玉, 碧玉, etc.).
    pub fn chinese_name(self) -> &'static str {
        match self {
            Jade::Bai => "白玉",
            Jade::Bi => "碧玉",
            Jade::Qing => "青玉",
            Jade::Hong => "红玉",
            Jade::Huang => "黄玉",
            Jade::Mo => "墨玉",
        }
    }

    /// Latin name (HUD fallback if the CJK font is unavailable).
    pub fn latin_name(self) -> &'static str {
        match self {
            Jade::Bai => "Bai",
            Jade::Bi => "Bi",
            Jade::Qing => "Qing",
            Jade::Hong => "Hong",
            Jade::Huang => "Huang",
            Jade::Mo => "Mo",
        }
    }
}

/// A jade tile in the grid.
///
/// The logical position (`row`, `col`) is stable throughout the tile's
/// lifetime. The visual position (`visual_row`, `visual_col`) is
/// interpolated by animations during swaps and falls.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tile {
    pub jade: Jade,
    /// Logical row, 0 = top.
    pub row: usize,
    /// Logical column, 0 = left.
    pub col: usize,
    /// Visual vertical position (f32 for animation).
    pub visual_row: f32,
    /// Visual horizontal position (f32 for animation).
    pub visual_col: f32,
    /// Stable seed for generating the jade's procedural veins.
    pub vein_seed: u32,
}

impl Tile {
    /// Creates a tile in its "resting" state: visual == logical.
    pub fn new(jade: Jade, row: usize, col: usize, vein_seed: u32) -> Self {
        Self {
            jade,
            row,
            col,
            visual_row: row as f32,
            visual_col: col as f32,
            vein_seed,
        }
    }

    /// True if the tile is visually at its logical position
    /// (no animation in progress).
    pub fn is_visually_idle(&self) -> bool {
        (self.visual_row - self.row as f32).abs() < 1e-3
            && (self.visual_col - self.col as f32).abs() < 1e-3
    }

    /// Visual distance (in tile units) between the current position
    /// and the logical position. Useful to modulate juice intensity.
    pub fn visual_offset(&self) -> (f32, f32) {
        (
            self.visual_row - self.row as f32,
            self.visual_col - self.col as f32,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jade_shapes_are_distinct() {
        let mut shapes: Vec<JadeShape> =
            Jade::ALL.iter().map(|j| j.shape()).collect();
        shapes.sort_by_key(|s| *s as u8);
        shapes.dedup();
        assert_eq!(shapes.len(), JADE_TYPES, "every jade needs a unique shape");
    }

    #[test]
    fn jade_all_has_six_entries() {
        assert_eq!(Jade::ALL.len(), JADE_TYPES);
        assert_eq!(JADE_TYPES, 6);
    }

    #[test]
    fn jade_index_roundtrip() {
        for (i, j) in Jade::ALL.iter().enumerate() {
            assert_eq!(j.index(), i);
            assert_eq!(Jade::from_index(i), Some(*j));
        }
    }

    #[test]
    fn jade_from_index_out_of_bounds() {
        assert!(Jade::from_index(6).is_none());
        assert!(Jade::from_index(usize::MAX).is_none());
    }

    #[test]
    fn jade_indices_are_unique() {
        let mut indices: Vec<usize> = Jade::ALL.iter().map(|j| j.index()).collect();
        indices.sort_unstable();
        indices.dedup();
        assert_eq!(indices.len(), JADE_TYPES);
    }

    #[test]
    fn jade_colors_are_distinct() {
        // Verify that no two colours are identical.
        for (i, a) in Jade::ALL.iter().enumerate() {
            for (k, b) in Jade::ALL.iter().enumerate() {
                if i == k {
                    continue;
                }
                assert_ne!(a.base_rgb(), b.base_rgb(), "{a:?} == {b:?}");
            }
        }
    }

    #[test]
    fn jade_chinese_names_are_distinct() {
        let mut names: Vec<&str> = Jade::ALL.iter().map(|j| j.chinese_name()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), JADE_TYPES);
    }

    #[test]
    fn jade_latin_names_are_distinct() {
        let mut names: Vec<&str> = Jade::ALL.iter().map(|j| j.latin_name()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), JADE_TYPES);
    }

    #[test]
    fn tile_new_visual_matches_logical() {
        let t = Tile::new(Jade::Bi, 3, 5, 42);
        assert_eq!(t.row, 3);
        assert_eq!(t.col, 5);
        assert!((t.visual_row - 3.0).abs() < 1e-6);
        assert!((t.visual_col - 5.0).abs() < 1e-6);
        assert_eq!(t.vein_seed, 42);
    }

    #[test]
    fn tile_is_visually_idle_after_new() {
        let t = Tile::new(Jade::Hong, 0, 0, 1);
        assert!(t.is_visually_idle());
    }

    #[test]
    fn tile_is_visually_idle_when_animated_position_equals_logical() {
        let mut t = Tile::new(Jade::Mo, 4, 4, 7);
        t.visual_row = 4.0;
        t.visual_col = 4.0;
        assert!(t.is_visually_idle());
    }

    #[test]
    fn tile_is_not_idle_when_offset() {
        let mut t = Tile::new(Jade::Qing, 2, 2, 3);
        t.visual_row = 1.5;
        assert!(!t.is_visually_idle());
        let (dr, dc) = t.visual_offset();
        assert!((dr - (-0.5)).abs() < 1e-6);
        assert!((dc - 0.0).abs() < 1e-6);
    }

    #[test]
    fn tile_copy_preserves_data() {
        let a = Tile::new(Jade::Huang, 6, 7, 99);
        let b = a;
        assert_eq!(a, b);
    }
}