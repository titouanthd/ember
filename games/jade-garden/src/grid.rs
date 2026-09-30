//! 8×8 grid logic and match-3 detection.
//!
//! The grid is **always full** logically (64 tiles). Visual positions
//! of tiles may differ from their logical position during an animation
//! (swap, fall), but the `tiles[row][col]` structure stays coherent at
//! all times.
//!
//! Every function in this module is **pure** or nearly pure (only
//! `Rng` is mutable). They can be tested in isolation.

use std::collections::HashSet;

use ember_core::rng::Rng;

use crate::components::{Jade, Tile, GRID_H, GRID_W, JADE_TYPES};

/// The game grid: 8 rows × 8 columns of tiles.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grid {
    pub tiles: [[Tile; GRID_W]; GRID_H],
}

impl Grid {
    /// Generates an initial grid with no match and at least one
    /// playable move. Retries up to `MAX_ATTEMPTS` times.
    pub fn new(seed: u32) -> Self {
        let mut rng = Rng::new(seed);
        for _ in 0..MAX_ATTEMPTS {
            let grid = Self::generate(&mut rng);
            if grid.find_matches().is_empty() && grid.has_any_valid_move() {
                return grid;
            }
        }
        // Fallback: the last generated grid, matched or not.
        Self::generate(&mut rng)
    }

    /// Generates a random grid while avoiding immediate matches (top
    /// and left). May still produce blocked grids — `new` retries.
    fn generate(rng: &mut Rng) -> Self {
        let mut tiles: [[Tile; GRID_W]; GRID_H] =
            [[Tile::new(Jade::Bai, 0, 0, 0); GRID_W]; GRID_H];

        for row in 0..GRID_H {
            for col in 0..GRID_W {
                let jade = pick_safe_jade(&tiles, row, col, rng);
                let vein_seed = rng.next_u32();
                tiles[row][col] = Tile::new(jade, row, col, vein_seed);
            }
        }

        Self { tiles }
    }

    /// Read-only access to a tile by logical coordinates.
    pub fn get(&self, row: usize, col: usize) -> Option<&Tile> {
        if row < GRID_H && col < GRID_W {
            Some(&self.tiles[row][col])
        } else {
            None
        }
    }

    /// Mutable access.
    pub fn get_mut(&mut self, row: usize, col: usize) -> Option<&mut Tile> {
        if row < GRID_H && col < GRID_W {
            Some(&mut self.tiles[row][col])
        } else {
            None
        }
    }

    /// Swaps two adjacent tiles **logically**. Updates logical and
    /// visual positions (commit).
    ///
    /// Call this once a swap animation has been validated by the
    /// player. During the animation, the caller must interpolate
    /// visually without calling this method.
    pub fn swap(&mut self, a: (usize, usize), b: (usize, usize)) {
        if a.0 >= GRID_H || a.1 >= GRID_W || b.0 >= GRID_H || b.1 >= GRID_W {
            return;
        }
        let ta = self.tiles[a.0][a.1];
        let tb = self.tiles[b.0][b.1];

        let mut new_at_a = tb;
        new_at_a.row = a.0;
        new_at_a.col = a.1;
        new_at_a.visual_row = a.0 as f32;
        new_at_a.visual_col = a.1 as f32;

        let mut new_at_b = ta;
        new_at_b.row = b.0;
        new_at_b.col = b.1;
        new_at_b.visual_row = b.0 as f32;
        new_at_b.visual_col = b.1 as f32;

        self.tiles[a.0][a.1] = new_at_a;
        self.tiles[b.0][b.1] = new_at_b;
    }

    /// Returns the coordinates of every tile that is part of a run of
    /// 3+ identical jades (horizontal or vertical), deduplicated.
    pub fn find_matches(&self) -> Vec<(usize, usize)> {
        let mut set: HashSet<(usize, usize)> = HashSet::new();

        // Horizontal runs.
        for row in 0..GRID_H {
            let mut col = 0;
            while col < GRID_W {
                let jade = self.tiles[row][col].jade;
                let start = col;
                while col < GRID_W && self.tiles[row][col].jade == jade {
                    col += 1;
                }
                if col - start >= 3 {
                    for c in start..col {
                        set.insert((row, c));
                    }
                }
            }
        }

        // Vertical runs.
        for col in 0..GRID_W {
            let mut row = 0;
            while row < GRID_H {
                let jade = self.tiles[row][col].jade;
                let start = row;
                while row < GRID_H && self.tiles[row][col].jade == jade {
                    row += 1;
                }
                if row - start >= 3 {
                    for r in start..row {
                        set.insert((r, col));
                    }
                }
            }
        }

        let mut out: Vec<(usize, usize)> = set.into_iter().collect();
        out.sort();
        out
    }

    /// True if the tile at `(row, col)` is part of a run of 3+
    /// (horizontal or vertical). Used after a swap to validate it.
    pub fn has_match_at(&self, row: usize, col: usize) -> bool {
        if row >= GRID_H || col >= GRID_W {
            return false;
        }
        let jade = self.tiles[row][col].jade;

        // Horizontal run.
        let mut count = 1;
        let mut c = col;
        while c > 0 && self.tiles[row][c - 1].jade == jade {
            count += 1;
            c -= 1;
        }
        let mut c = col;
        while c + 1 < GRID_W && self.tiles[row][c + 1].jade == jade {
            count += 1;
            c += 1;
        }
        if count >= 3 {
            return true;
        }

        // Vertical run.
        let mut count = 1;
        let mut r = row;
        while r > 0 && self.tiles[r - 1][col].jade == jade {
            count += 1;
            r -= 1;
        }
        let mut r = row;
        while r + 1 < GRID_H && self.tiles[r + 1][col].jade == jade {
            count += 1;
            r += 1;
        }
        count >= 3
    }

    /// True if at least one adjacent swap is possible and would create
    /// a match. Used to detect dead-locked grids.
    pub fn has_any_valid_move(&self) -> bool {
        // Horizontal swaps.
        for row in 0..GRID_H {
            for col in 0..(GRID_W - 1) {
                if self.would_match_after_swap((row, col), (row, col + 1)) {
                    return true;
                }
            }
        }
        // Vertical swaps.
        for row in 0..(GRID_H - 1) {
            for col in 0..GRID_W {
                if self.would_match_after_swap((row, col), (row + 1, col)) {
                    return true;
                }
            }
        }
        false
    }

    /// Non-mutating test: the grid is copied, swapped, tested, and
    /// thrown away. `Tile` and `Grid` are `Copy`, so this is cheap
    /// (2 KB).
    fn would_match_after_swap(&self, a: (usize, usize), b: (usize, usize)) -> bool {
        let mut test = *self;
        test.swap(a, b);
        test.has_match_at(a.0, a.1) || test.has_match_at(b.0, b.1)
    }

    /// Removes the tiles at the given coordinates, drops the tiles
    /// above to fill the holes, and refills the top of the column with
    /// new jades.
    ///
    /// Surviving tiles keep their old `visual_row` (for the fall
    /// animation). New tiles are created with a negative `visual_row`
    /// (above the grid) to animate their appearance.
    pub fn remove_and_collapse(
        &mut self,
        coords: &[(usize, usize)],
        rng: &mut Rng,
    ) {
        let coord_set: HashSet<(usize, usize)> = coords.iter().copied().collect();

        for col in 0..GRID_W {
            // Compact downward.
            let mut write_row = GRID_H;
            for read_row in (0..GRID_H).rev() {
                if coord_set.contains(&(read_row, col)) {
                    continue;
                }
                write_row -= 1;
                if write_row != read_row {
                    self.tiles[write_row][col] = self.tiles[read_row][col];
                    self.tiles[write_row][col].row = write_row;
                }
            }

            // Fill the top with new tiles.
            for r in 0..write_row {
                let jade = pick_safe_jade(&self.tiles, r, col, rng);
                let vein_seed = rng.next_u32();
                let mut tile = Tile::new(jade, r, col, vein_seed);
                // Start above the grid (negative).
                tile.visual_row = r as f32 - write_row as f32;
                self.tiles[r][col] = tile;
            }
        }
    }

    /// Shuffles the grid while preserving the jade multiset and
    /// guaranteeing no immediate match and at least one valid move.
    /// Retries up to `MAX_ATTEMPTS` times, then regenerates a fresh
    /// grid as a last resort.
    pub fn reshuffle(&mut self, rng: &mut Rng) {
        // Extract every jade into a Vec.
        let mut jades: Vec<Jade> = Vec::with_capacity(GRID_W * GRID_H);
        for row in 0..GRID_H {
            for col in 0..GRID_W {
                jades.push(self.tiles[row][col].jade);
            }
        }

        for _ in 0..MAX_ATTEMPTS {
            // Fisher-Yates.
            let n = jades.len();
            for i in (1..n).rev() {
                let j = rng.next_range(i + 1);
                jades.swap(i, j);
            }

            // Write back into the grid.
            let mut idx = 0;
            for row in 0..GRID_H {
                for col in 0..GRID_W {
                    self.tiles[row][col].jade = jades[idx];
                    idx += 1;
                }
            }

            if self.find_matches().is_empty() && self.has_any_valid_move() {
                return;
            }
        }

        // Fallback: fully regenerate.
        *self = Self::new(rng.next_u32());
    }

    /// Counts the total number of distinct matched tiles (useful for
    /// scoring multi-match cascades).
    pub fn count_matches(&self) -> usize {
        self.find_matches().len()
    }
}

/// Maximum number of attempts to generate or reshuffle a valid grid
/// before falling back.
pub const MAX_ATTEMPTS: usize = 50;

/// Picks a random jade while avoiding an immediate match with the two
/// neighbours above or to the left (already placed).
///
/// `tiles` is the grid being built. Neighbours `(row-1, col)`,
/// `(row-2, col)`, `(row, col-1)` and `(row, col-2)` are assumed to be
/// already filled.
pub fn pick_safe_jade(
    tiles: &[[Tile; GRID_W]; GRID_H],
    row: usize,
    col: usize,
    rng: &mut Rng,
) -> Jade {
    let mut forbidden = [false; JADE_TYPES];

    if row >= 2 && tiles[row - 1][col].jade == tiles[row - 2][col].jade {
        forbidden[tiles[row - 1][col].jade.index()] = true;
    }
    if col >= 2 && tiles[row][col - 1].jade == tiles[row][col - 2].jade {
        forbidden[tiles[row][col - 1].jade.index()] = true;
    }

    let allowed: Vec<Jade> = Jade::ALL
        .iter()
        .copied()
        .filter(|j| !forbidden[j.index()])
        .collect();

    if allowed.is_empty() {
        // Should not happen with 6 types and 2 checks, but we prefer
        // a deterministic fallback over a panic.
        Jade::ALL[rng.next_range(JADE_TYPES)]
    } else {
        allowed[rng.next_range(allowed.len())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds an 8×8 grid from a jade layout (one character per cell).
    /// Used to write readable test scenarios.
    ///
    /// Example: `"BBBC...."` etc. — 1 character per cell.
    /// `B` = Bai, `g` = Bi, `q` = Qing, `r` = Hong, `y` = Huang,
    /// `m` = Mo, `.` = Bai (placeholder, not interpreted).
    fn grid_from_str(rows: [&str; GRID_H]) -> Grid {
        let mut tiles: [[Tile; GRID_W]; GRID_H] =
            [[Tile::new(Jade::Bai, 0, 0, 0); GRID_W]; GRID_H];
        for (r, line) in rows.iter().enumerate() {
            assert_eq!(line.len(), GRID_W, "row {r} length");
            for (c, ch) in line.chars().enumerate() {
                let jade = match ch {
                    'B' => Jade::Bai,
                    'g' => Jade::Bi,
                    'q' => Jade::Qing,
                    'r' => Jade::Hong,
                    'y' => Jade::Huang,
                    'm' => Jade::Mo,
                    _ => Jade::Bai,
                };
                let seed = (r as u32) * 100 + (c as u32);
                tiles[r][c] = Tile::new(jade, r, c, seed);
            }
        }
        Grid { tiles }
    }

    // ---------- Construction ----------

    #[test]
    fn new_grid_has_no_initial_match() {
        for seed in 1..20 {
            let g = Grid::new(seed);
            assert!(
                g.find_matches().is_empty(),
                "seed {seed} produced initial matches"
            );
        }
    }

    #[test]
    fn new_grid_has_at_least_one_valid_move() {
        for seed in 1..20 {
            let g = Grid::new(seed);
            assert!(g.has_any_valid_move(), "seed {seed} has no valid move");
        }
    }

    #[test]
    fn new_grid_is_deterministic_for_same_seed() {
        let a = Grid::new(12345);
        let b = Grid::new(12345);
        assert_eq!(a, b);
    }

    #[test]
    fn new_grid_differs_for_different_seeds() {
        let a = Grid::new(1);
        let b = Grid::new(2);
        assert_ne!(a, b);
    }

    #[test]
    fn new_grid_is_full() {
        let g = Grid::new(1);
        for row in 0..GRID_H {
            for col in 0..GRID_W {
                let t = g.get(row, col).unwrap();
                assert_eq!(t.row, row);
                assert_eq!(t.col, col);
                assert!(t.is_visually_idle());
            }
        }
    }

    // ---------- get / get_mut ----------

    #[test]
    fn get_out_of_bounds_returns_none() {
        let g = Grid::new(1);
        assert!(g.get(GRID_H, 0).is_none());
        assert!(g.get(0, GRID_W).is_none());
        assert!(g.get(usize::MAX, usize::MAX).is_none());
    }

    #[test]
    fn get_mut_modifies_in_place() {
        let mut g = Grid::new(1);
        if let Some(t) = g.get_mut(3, 4) {
            t.jade = Jade::Hong;
        }
        assert_eq!(g.get(3, 4).unwrap().jade, Jade::Hong);
    }

    // ---------- swap ----------

    #[test]
    fn swap_updates_jades() {
        let mut g = Grid::new(1);
        let j_a = g.get(0, 0).unwrap().jade;
        let j_b = g.get(0, 1).unwrap().jade;
        g.swap((0, 0), (0, 1));
        assert_eq!(g.get(0, 0).unwrap().jade, j_b);
        assert_eq!(g.get(0, 1).unwrap().jade, j_a);
    }

    #[test]
    fn swap_updates_logical_positions() {
        let mut g = Grid::new(1);
        g.swap((2, 3), (2, 4));
        let t_left = g.get(2, 3).unwrap();
        let t_right = g.get(2, 4).unwrap();
        assert_eq!(t_left.row, 2);
        assert_eq!(t_left.col, 3);
        assert_eq!(t_right.row, 2);
        assert_eq!(t_right.col, 4);
    }

    #[test]
    fn swap_resets_visual_positions_to_logical() {
        let mut g = Grid::new(1);
        // Introduce an artificial visual offset.
        g.get_mut(0, 0).unwrap().visual_row = -2.0;
        g.swap((0, 0), (0, 1));
        assert!(g.get(0, 0).unwrap().is_visually_idle());
        assert!(g.get(0, 1).unwrap().is_visually_idle());
    }

    #[test]
    fn swap_out_of_bounds_is_noop() {
        let mut g = Grid::new(1);
        let before = g;
        g.swap((0, 0), (GRID_H, 0));
        assert_eq!(g, before);
    }

    // ---------- find_matches ----------

    #[test]
    fn find_matches_empty_on_idle_grid() {
        let g = grid_from_str([
            "BgyrmqBg",
            "gqrmBygm",
            "qrmBgyqr",
            "rmBgyqrm",
            "mBgyqrmB",
            "BgyqrmBg",
            "gyqrmBgy",
            "yqrmBgyq",
        ]);
        assert!(g.find_matches().is_empty());
    }

    #[test]
    fn find_matches_detects_horizontal_three() {
        let g = grid_from_str([
            "BBBgqrmy",
            "gqrmBgyq",
            "qrmBgyqr",
            "rmBgyqrm",
            "mBgyqrmB",
            "BgyqrmBg",
            "gyqrmBgy",
            "yqrmBgyq",
        ]);
        let m = g.find_matches();
        assert_eq!(m.len(), 3);
        assert!(m.contains(&(0, 0)));
        assert!(m.contains(&(0, 1)));
        assert!(m.contains(&(0, 2)));
    }

    #[test]
    fn find_matches_detects_vertical_three() {
        let g = grid_from_str([
            "BgyqrmBy",
            "BqrmBgyq",
            "BrmBgyqr",
            "rmBgyqrm",
            "mBgyqrmB",
            "BgyqrmBg",
            "gyqrmBgy",
            "yqrmBgyq",
        ]);
        let m = g.find_matches();
        assert!(m.contains(&(0, 0)));
        assert!(m.contains(&(1, 0)));
        assert!(m.contains(&(2, 0)));
    }

    #[test]
    fn find_matches_detects_horizontal_four() {
        let g = grid_from_str([
            "BBBBqrmy",
            "gqrmBgyq",
            "qrmBgyqr",
            "rmBgyqrm",
            "mBgyqrmB",
            "BgyqrmBg",
            "gyqrmBgy",
            "yqrmBgyq",
        ]);
        let m = g.find_matches();
        assert_eq!(m.len(), 4);
    }

    #[test]
    fn find_matches_detects_cross() {
        // Cross centered at (2, 3): Bi in row 2 cols 2,3,4 and in col 3
        // rows 2,3,4.
        let g = grid_from_str([
            "BgyqrmBy",
            "gqrmBgyq",
            "qBgggBrB",
            "rmBgBgyq",
            "mBggBgrm",
            "BgyqrmBg",
            "gyqrmBgy",
            "yqrmBgyq",
        ]);
        let m = g.find_matches();
        assert!(m.contains(&(2, 2)));
        assert!(m.contains(&(2, 3)));
        assert!(m.contains(&(2, 4)));
        assert!(m.contains(&(3, 3)));
        assert!(m.contains(&(4, 3)));
        assert_eq!(m.len(), 5, "exactly 5 unique tiles");
    }

    #[test]
    fn find_matches_deduplicates_shared_tiles() {
        // Same grid as above: (2, 3) is shared between the horizontal
        // run (row 2) and the vertical run (col 3).
        let g = grid_from_str([
            "BgyqrmBy",
            "gqrmBgyq",
            "qBgggBrB",
            "rmBgBgyq",
            "mBggBgrm",
            "BgyqrmBg",
            "gyqrmBgy",
            "yqrmBgyq",
        ]);
        let m = g.find_matches();
        let count_2_3 = m.iter().filter(|&&x| x == (2, 3)).count();
        assert_eq!(count_2_3, 1);
    }

    #[test]
    fn find_matches_are_sorted() {
        let g = grid_from_str([
            "BBBgqrmy",
            "gqrmBgyq",
            "qrmBgyqr",
            "rmBgyqrm",
            "mBgyqrmB",
            "BgyqrmBg",
            "gyqrmBgy",
            "yqrmBgyq",
        ]);
        let m = g.find_matches();
        let mut sorted = m.clone();
        sorted.sort();
        assert_eq!(m, sorted);
    }

    // ---------- has_match_at ----------

    #[test]
    fn has_match_at_center_of_three() {
        let g = grid_from_str([
            "BBBgqrmy",
            "gqrmBgyq",
            "qrmBgyqr",
            "rmBgyqrm",
            "mBgyqrmB",
            "BgyqrmBg",
            "gyqrmBgy",
            "yqrmBgyq",
        ]);
        assert!(g.has_match_at(0, 0));
        assert!(g.has_match_at(0, 1));
        assert!(g.has_match_at(0, 2));
        assert!(!g.has_match_at(0, 3));
    }

    #[test]
    fn has_match_at_out_of_bounds_is_false() {
        let g = Grid::new(1);
        assert!(!g.has_match_at(GRID_H, 0));
        assert!(!g.has_match_at(0, GRID_W));
    }

    // ---------- has_any_valid_move ----------

    #[test]
    fn has_any_valid_move_detects_swap_creating_three() {
        // Grid with an obvious swap at the top: swapping (0,0) and
        // (0,1) aligns 3 Bai in row 0.
        let g = grid_from_str([
            "BgBqrmyq",
            "BqrmBgyq",
            "BrmBgyqr",
            "rmBgyqrm",
            "mBgyqrmB",
            "BgyqrmBg",
            "gyqrmBgy",
            "yqrmBgyq",
        ]);
        assert!(g.has_any_valid_move());
    }

    #[test]
    fn would_match_after_swap_does_not_mutate() {
        let g = grid_from_str([
            "BgBqrmyq",
            "BqrmBgyq",
            "BrmBgyqr",
            "rmBgyqrm",
            "mBgyqrmB",
            "BgyqrmBg",
            "gyqrmBgy",
            "yqrmBgyq",
        ]);
        let before = g;
        let _ = g.has_any_valid_move();
        assert_eq!(g, before);
    }

    // ---------- remove_and_collapse ----------

    #[test]
    fn remove_and_collapse_removes_target_tiles() {
        let mut g = Grid::new(1);
        let mut rng = Rng::new(1);
        let coords = vec![(GRID_H - 1, 0), (GRID_H - 2, 0), (GRID_H - 3, 0)];
        let _ = coords; // (jades will differ)
        // Note: this test only verifies that the grid stays full and
        // that no coordinate is null. We can't predict the jades
        // without re-reading.
        g.remove_and_collapse(&coords, &mut rng);
        for row in 0..GRID_H {
            for col in 0..GRID_W {
                let t = g.get(row, col).unwrap();
                assert_eq!(t.row, row);
                assert_eq!(t.col, col);
            }
        }
    }

    #[test]
    fn remove_and_collapse_leaves_top_tiles_above_grid_visually() {
        let mut g = Grid::new(1);
        let mut rng = Rng::new(1);
        let coords = vec![(GRID_H - 1, 0), (GRID_H - 2, 0), (GRID_H - 3, 0)];
        g.remove_and_collapse(&coords, &mut rng);
        // 3 new tiles should have a negative visual_row.
        let mut off_grid = 0;
        for row in 0..GRID_H {
            if g.get(row, 0).unwrap().visual_row < 0.0 {
                off_grid += 1;
            }
        }
        assert_eq!(off_grid, 3);
    }

    #[test]
    fn remove_and_collapse_preserves_remaining_column_order() {
        // Tag the jades of column 0 with distinct values to trace
        // their positions after collapse.
        let mut g = grid_from_str([
            "BgyqrmBy",
            "gqrmBgyq",
            "qrmBgyqr",
            "rmBgyqrm",
            "mBgyqrmB",
            "BgyqrmBg",
            "gyqrmBgy",
            "yqrmBgyq",
        ]);
        let mut rng = Rng::new(1);
        // Destroy the bottom row and the middle row of column 0.
        let coords = vec![(7, 0), (5, 0)];
        // Save the column 0 jades, rows 0..5.
        let original_col: Vec<Jade> = (0..6).map(|r| g.get(r, 0).unwrap().jade).collect();
        g.remove_and_collapse(&coords, &mut rng);
        // After collapse: original jades rows 0..4 should have slid to
        // rows 2..6 (2 holes filled below them).
        for (offset, orig_row) in (0..5).enumerate() {
            let new_row = orig_row + 2;
            assert_eq!(
                g.get(new_row, 0).unwrap().jade,
                original_col[offset],
                "row {new_row} should hold original row {orig_row}"
            );
        }
    }

    #[test]
    fn remove_and_collapse_with_empty_coords_is_noop() {
        let mut g = Grid::new(1);
        let mut rng = Rng::new(1);
        let before = g;
        g.remove_and_collapse(&[], &mut rng);
        assert_eq!(g, before);
    }

    // ---------- reshuffle ----------

    #[test]
    fn reshuffle_preserves_multiset_of_jades() {
        let mut g = Grid::new(1);
        let mut rng = Rng::new(2);
        let mut before: Vec<Jade> = Vec::new();
        for row in 0..GRID_H {
            for col in 0..GRID_W {
                before.push(g.get(row, col).unwrap().jade);
            }
        }
        before.sort();
        g.reshuffle(&mut rng);
        let mut after: Vec<Jade> = Vec::new();
        for row in 0..GRID_H {
            for col in 0..GRID_W {
                after.push(g.get(row, col).unwrap().jade);
            }
        }
        after.sort();
        assert_eq!(before, after);
    }

    #[test]
    fn reshuffle_produces_grid_with_no_matches() {
        let mut g = Grid::new(1);
        let mut rng = Rng::new(3);
        for _ in 0..10 {
            g.reshuffle(&mut rng);
            assert!(g.find_matches().is_empty());
        }
    }

    #[test]
    fn reshuffle_produces_grid_with_valid_move() {
        let mut g = Grid::new(1);
        let mut rng = Rng::new(4);
        for _ in 0..10 {
            g.reshuffle(&mut rng);
            assert!(g.has_any_valid_move());
        }
    }

    // ---------- pick_safe_jade ----------

    #[test]
    fn pick_safe_jade_avoids_horizontal_match() {
        let mut tiles: [[Tile; GRID_W]; GRID_H] =
            [[Tile::new(Jade::Bai, 0, 0, 0); GRID_W]; GRID_H];
        tiles[0][0] = Tile::new(Jade::Bi, 0, 0, 0);
        tiles[0][1] = Tile::new(Jade::Bi, 0, 1, 0);
        // We want a jade for (0, 2): it must not be Bi.
        let mut rng = Rng::new(1);
        for _ in 0..20 {
            let j = pick_safe_jade(&tiles, 0, 2, &mut rng);
            assert_ne!(j, Jade::Bi);
        }
    }

    #[test]
    fn pick_safe_jade_avoids_vertical_match() {
        let mut tiles: [[Tile; GRID_W]; GRID_H] =
            [[Tile::new(Jade::Bai, 0, 0, 0); GRID_W]; GRID_H];
        tiles[0][0] = Tile::new(Jade::Hong, 0, 0, 0);
        tiles[1][0] = Tile::new(Jade::Hong, 1, 0, 0);
        // We want a jade for (2, 0): it must not be Hong.
        let mut rng = Rng::new(2);
        for _ in 0..20 {
            let j = pick_safe_jade(&tiles, 2, 0, &mut rng);
            assert_ne!(j, Jade::Hong);
        }
    }

    // ---------- count_matches ----------

    #[test]
    fn count_matches_matches_find_matches_len() {
        let g = grid_from_str([
            "BBBgqrmy",
            "gqrmBgyq",
            "qrmBgyqr",
            "rmBgyqrm",
            "mBgyqrmB",
            "BgyqrmBg",
            "gyqrmBgy",
            "yqrmBgyq",
        ]);
        assert_eq!(g.count_matches(), g.find_matches().len());
    }
}