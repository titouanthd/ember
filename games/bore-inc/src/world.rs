//! The world of one level: a 2D grid of tiles that Archie and the
//! truck move through.
//!
//! **Coordinate convention**: the underlying `ember_stdlib::grid::Grid`
//! is indexed `(col, row)`, with `col` first.
//!
//! **Pixel vs tile**: `World` stores only tiles. Pixel-to-tile
//! conversion is done by callers via the `pixel_to_col/row` helpers.
//!
//! **Delta**: every `set` that changes a cell's kind appends to
//! `delta`. On level load, the saved delta is reapplied to a freshly
//! generated world so the tunnel survives a trip through the HQ.

use ember_stdlib::grid::Grid;

use crate::components::{Tile, TileKind, TILE_SIZE};

/// One recorded cell change: `(col, row, new_kind)`. `None` = cell
/// mined out. `TileKind` is stored (not full `Tile`) because a
/// re-mined tile always starts at full HP.
pub type DeltaEntry = (u16, u16, Option<TileKind>);

/// The world of one level.
#[derive(Debug, Clone)]
pub struct World {
    grid: Grid<Option<Tile>>,
    pub surface_row: usize,
    pub seed: u32,
    /// Every cell mutation since the level was generated, in order.
    /// Replayed on load via `apply_delta`. Bounded only by the number
    /// of tiles in the level.
    pub delta: Vec<DeltaEntry>,
}

impl World {
    pub fn from_cells(
        width: usize,
        height: usize,
        cells: Vec<Option<Tile>>,
        surface_row: usize,
        seed: u32,
    ) -> Self {
        Self {
            grid: Grid::from_vec(width, height, cells),
            surface_row,
            seed,
            delta: Vec::new(),
        }
    }

    /// A small hand-authored world for tests.
    pub fn test_world() -> Self {
        const W: usize = 48;
        const H: usize = 24;
        const SURFACE: usize = 12;

        let mut cells = Vec::with_capacity(W * H);
        for row in 0..H {
            for col in 0..W {
                cells.push(test_tile_kind(col, row, SURFACE).map(Tile::new));
            }
        }
        Self::from_cells(W, H, cells, SURFACE, 0)
    }

    pub fn width(&self) -> usize {
        self.grid.width()
    }

    pub fn height(&self) -> usize {
        self.grid.height()
    }

    /// Column the truck must reach to complete the level.
    pub fn finish_col(&self) -> usize {
        self.width().saturating_sub(2)
    }

    // ---------- Tile queries ----------

    pub fn tile(&self, col: usize, row: usize) -> Option<&Tile> {
        self.grid.get(col, row).and_then(|c| c.as_ref())
    }

    pub fn tile_mut(&mut self, col: usize, row: usize) -> Option<&mut Tile> {
        self.grid.get_mut(col, row).and_then(|c| c.as_mut())
    }

    /// Replace a cell. Records the change in `delta` if the tile
    /// kind actually differs from what was there.
    pub fn set(&mut self, col: usize, row: usize, cell: Option<Tile>) {
        let new_kind = cell.as_ref().map(|t| t.kind);
        if let Some(c) = self.grid.get_mut(col, row) {
            let old_kind = c.as_ref().map(|t| t.kind);
            if old_kind != new_kind {
                self.delta.push((col as u16, row as u16, new_kind));
            }
            *c = cell;
        }
    }

    /// Replace this world's `delta` with `delta`, then replay every
    /// entry against the grid. Call once after generation to bring a
    /// level back to its saved state.
    ///
    /// After this call, subsequent `set` calls append to `delta` — so
    /// new edits made this session accumulate on top of the loaded
    /// history.
    pub fn apply_delta(&mut self, delta: &[DeltaEntry]) {
        self.delta = delta.to_vec();
        for &(col, row, kind) in delta {
            let cell = kind.map(Tile::new);
            // Direct write, bypassing recording.
            if let Some(c) = self.grid.get_mut(col as usize, row as usize) {
                *c = cell;
            }
        }
    }

    pub fn is_solid(&self, col: isize, row: isize) -> bool {
        if col < 0 || row < 0 {
            return true;
        }
        match self.grid.get(col as usize, row as usize) {
            Some(cell) => cell.is_some(),
            None => true,
        }
    }

    pub fn is_empty(&self, col: isize, row: isize) -> bool {
        if col < 0 || row < 0 {
            return false;
        }
        matches!(self.grid.get(col as usize, row as usize), Some(None))
    }

    pub fn pixel_to_col(x: f32) -> isize {
        (x / TILE_SIZE).floor() as isize
    }

    pub fn pixel_to_row(y: f32) -> isize {
        (y / TILE_SIZE).floor() as isize
    }
}

fn test_tile_kind(col: usize, row: usize, surface_row: usize) -> Option<TileKind> {
    if row < surface_row {
        if col >= 16 && row >= surface_row - 3 {
            return Some(TileKind::Dirt);
        }
        return None;
    }

    if row >= 22 {
        return Some(TileKind::Bedrock);
    }

    if (20..=23).contains(&col) && (16..=17).contains(&row) {
        return Some(TileKind::Coal);
    }
    if col == 30 && (18..=19).contains(&row) {
        return Some(TileKind::Iron);
    }
    if col == 32 && row == 20 {
        return Some(TileKind::Crystal);
    }

    if row <= surface_row + 2 {
        return Some(TileKind::Dirt);
    }
    Some(TileKind::Stone)
}

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_has_expected_dimensions() {
        let w = World::test_world();
        assert_eq!(w.width(), 48);
        assert_eq!(w.height(), 24);
        assert_eq!(w.surface_row, 12);
    }

    #[test]
    fn test_world_is_sky_above_surface() {
        let w = World::test_world();
        for row in 0..9 {
            for col in 0..48 {
                assert!(w.is_empty(col as isize, row as isize));
            }
        }
    }

    #[test]
    fn mountain_face_rises_three_rows() {
        let w = World::test_world();
        for col in 16..48 {
            assert!(w.tile(col, 9).is_some());
        }
    }

    #[test]
    fn test_world_has_bedrock_at_bottom() {
        let w = World::test_world();
        for col in 0..48 {
            assert_eq!(w.tile(col, 22).map(|t| t.kind), Some(TileKind::Bedrock));
            assert_eq!(w.tile(col, 23).map(|t| t.kind), Some(TileKind::Bedrock));
        }
    }

    #[test]
    fn test_world_has_a_crystal() {
        let w = World::test_world();
        assert_eq!(w.tile(32, 20).map(|t| t.kind), Some(TileKind::Crystal));
    }

    #[test]
    fn test_world_has_coal_and_iron_veins() {
        let w = World::test_world();
        assert_eq!(w.tile(20, 16).map(|t| t.kind), Some(TileKind::Coal));
        assert_eq!(w.tile(30, 18).map(|t| t.kind), Some(TileKind::Iron));
    }

    #[test]
    fn out_of_bounds_is_solid() {
        let w = World::test_world();
        assert!(w.is_solid(-1, 0));
        assert!(w.is_solid(0, -1));
        assert!(w.is_solid(48, 0));
        assert!(w.is_solid(0, 24));
    }

    #[test]
    fn in_bounds_air_is_not_solid() {
        let w = World::test_world();
        assert!(!w.is_solid(0, 0));
        assert!(!w.is_solid(15, 11));
    }

    #[test]
    fn in_bounds_tile_is_solid() {
        let w = World::test_world();
        assert!(w.is_solid(0, 12));
        assert!(w.is_solid(47, 23));
    }

    #[test]
    fn pixel_to_col_rounds_down() {
        assert_eq!(World::pixel_to_col(0.0), 0);
        assert_eq!(World::pixel_to_col(15.9), 0);
        assert_eq!(World::pixel_to_col(16.0), 1);
    }

    #[test]
    fn pixel_to_row_rounds_down() {
        assert_eq!(World::pixel_to_row(0.0), 0);
        assert_eq!(World::pixel_to_row(15.9), 0);
        assert_eq!(World::pixel_to_row(16.0), 1);
    }

    #[test]
    fn pixel_to_col_handles_negative() {
        assert_eq!(World::pixel_to_col(-1.0), -1);
        assert_eq!(World::pixel_to_col(-17.0), -2);
    }

    #[test]
    fn set_replaces_a_cell() {
        let mut w = World::test_world();
        assert!(w.tile(5, 12).is_some());
        w.set(5, 12, None);
        assert!(w.tile(5, 12).is_none());
        w.set(5, 12, Some(Tile::new(TileKind::Coal)));
        assert_eq!(w.tile(5, 12).map(|t| t.kind), Some(TileKind::Coal));
    }

    #[test]
    fn set_out_of_bounds_is_noop() {
        let mut w = World::test_world();
        w.set(100, 100, Some(Tile::new(TileKind::Stone)));
    }

    #[test]
    fn finish_col_is_near_the_right_edge() {
        let w = World::test_world();
        assert_eq!(w.finish_col(), w.width() - 2);
    }

    #[test]
    fn finish_col_is_zero_on_empty_world() {
        let w = World::from_cells(1, 1, vec![None], 0, 0);
        assert_eq!(w.finish_col(), 0);
    }

    // ---------- Delta ----------

    #[test]
    fn new_world_has_empty_delta() {
        let w = World::test_world();
        assert!(w.delta.is_empty());
    }

    #[test]
    fn set_records_delta_on_kind_change() {
        let mut w = World::test_world();
        assert!(w.delta.is_empty());
        w.set(5, 13, None);
        assert_eq!(w.delta.len(), 1);
        assert_eq!(w.delta[0], (5, 13, None));
    }

    #[test]
    fn set_same_kind_does_not_record() {
        let mut w = World::test_world();
        let kind = w.tile(5, 13).unwrap().kind;
        w.set(5, 13, Some(Tile::new(kind)));
        assert!(w.delta.is_empty());
    }

    #[test]
    fn set_records_replacement_kind() {
        let mut w = World::test_world();
        w.set(5, 13, Some(Tile::new(TileKind::Coal)));
        assert_eq!(w.delta.len(), 1);
        assert_eq!(w.delta[0], (5, 13, Some(TileKind::Coal)));
    }

    #[test]
    fn apply_delta_replays_cells() {
        let mut w = World::test_world();
        assert!(w.tile(5, 13).is_some());
        let delta: Vec<DeltaEntry> = vec![(5, 13, None)];
        w.apply_delta(&delta);
        assert!(w.tile(5, 13).is_none());
        assert_eq!(w.delta, delta);
    }

    #[test]
    fn apply_delta_does_not_double_record() {
        let mut w = World::test_world();
        let delta: Vec<DeltaEntry> = vec![(5, 13, None), (6, 13, None)];
        w.apply_delta(&delta);
        assert_eq!(w.delta.len(), 2);
    }

    #[test]
    fn mining_after_apply_appends_to_delta() {
        let mut w = World::test_world();
        w.apply_delta(&[(5, 13, None)]);
        assert_eq!(w.delta.len(), 1);
        w.set(10, 13, None);
        assert_eq!(w.delta.len(), 2);
    }

    #[test]
    fn delta_survives_regeneration() {
        // End-to-end: mine some cells, keep the delta, generate a
        // fresh copy of the same level, apply — the tunnel is back.
        let mut w1 = World::test_world();
        w1.set(5, 13, None);
        w1.set(6, 13, None);
        w1.set(7, 13, Some(Tile::new(TileKind::Coal)));
        let delta = w1.delta.clone();

        let mut w2 = World::test_world();
        assert!(w2.tile(5, 13).is_some());
        w2.apply_delta(&delta);
        assert!(w2.tile(5, 13).is_none());
        assert!(w2.tile(6, 13).is_none());
        assert_eq!(w2.tile(7, 13).map(|t| t.kind), Some(TileKind::Coal));
    }
}