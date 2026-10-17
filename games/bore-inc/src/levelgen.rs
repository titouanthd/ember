//! Procedural level generation. DESIGN §14.
//!
//! Public entry: `generate(seed, kind) -> World`. Pure — no `macroquad`,
//! no I/O — so worlds are byte-identical for a given `(seed, kind)` and
//! tests run headless.

use crate::components::{Tile, TileKind};
use crate::rng::Rng;
use crate::world::World;

/// The three V1 levels. One is the tutorial shape, Two introduces
/// crystal and HardRock, Three is the endgame — deeper, harder,
/// more of everything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelKind {
    One,
    Two,
    Three,
}

#[derive(Debug, Clone, Copy)]
pub struct VeinSpec {
    pub kind: TileKind,
    pub count: u32,
    pub size_min: u32,
    pub size_max: u32,
    pub row_min_offset: usize,
    pub row_max_offset: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct LevelParams {
    pub width: usize,
    pub height: usize,
    pub surface_row: usize,
    pub plain_end_col: usize,
    pub bedrock_rows: usize,
    pub veins: &'static [VeinSpec],
}

const LEVEL_ONE_VEINS: &[VeinSpec] = &[
    VeinSpec {
        kind: TileKind::Coal,
        count: 20,
        size_min: 6,
        size_max: 12,
        row_min_offset: 1,
        row_max_offset: 8,
    },
    VeinSpec {
        kind: TileKind::Iron,
        count: 15,
        size_min: 4,
        size_max: 8,
        row_min_offset: 4,
        row_max_offset: 13,
    },
];

const LEVEL_TWO_VEINS: &[VeinSpec] = &[
    VeinSpec {
        kind: TileKind::Coal,
        count: 30,
        size_min: 6,
        size_max: 12,
        row_min_offset: 1,
        row_max_offset: 12,
    },
    VeinSpec {
        kind: TileKind::Iron,
        count: 25,
        size_min: 4,
        size_max: 8,
        row_min_offset: 5,
        row_max_offset: 13,
    },
    VeinSpec {
        kind: TileKind::Crystal,
        count: 8,
        size_min: 2,
        size_max: 4,
        row_min_offset: 8,
        row_max_offset: 13,
    },
    VeinSpec {
        kind: TileKind::HardRock,
        count: 3,
        size_min: 5,
        size_max: 10,
        row_min_offset: 6,
        row_max_offset: 12,
    },
];

const LEVEL_THREE_VEINS: &[VeinSpec] = &[
    VeinSpec {
        kind: TileKind::Coal,
        count: 40,
        size_min: 6,
        size_max: 14,
        row_min_offset: 1,
        row_max_offset: 13,
    },
    VeinSpec {
        kind: TileKind::Iron,
        count: 30,
        size_min: 5,
        size_max: 10,
        row_min_offset: 5,
        row_max_offset: 13,
    },
    VeinSpec {
        kind: TileKind::Crystal,
        count: 12,
        size_min: 2,
        size_max: 5,
        row_min_offset: 8,
        row_max_offset: 13,
    },
    // Twice the HardRock of level Two. Real obstacles this time.
    VeinSpec {
        kind: TileKind::HardRock,
        count: 6,
        size_min: 6,
        size_max: 12,
        row_min_offset: 6,
        row_max_offset: 12,
    },
];

impl LevelKind {
    pub fn params(self) -> LevelParams {
        match self {
            LevelKind::One => LevelParams {
                width: 160,
                height: 25,
                surface_row: 8,
                plain_end_col: 16,
                bedrock_rows: 3,
                veins: LEVEL_ONE_VEINS,
            },
            LevelKind::Two => LevelParams {
                width: 250,
                height: 25,
                surface_row: 8,
                plain_end_col: 16,
                bedrock_rows: 3,
                veins: LEVEL_TWO_VEINS,
            },
            LevelKind::Three => LevelParams {
                width: 320,
                height: 25,
                surface_row: 8,
                plain_end_col: 16,
                bedrock_rows: 3,
                veins: LEVEL_THREE_VEINS,
            },
        }
    }

    /// Stable id used as a key in `Progress::levels`. Never change
    /// these strings without a migration — they identify a save slot.
    pub fn id(self) -> &'static str {
        match self {
            LevelKind::One => "level-one",
            LevelKind::Two => "level-two",
            LevelKind::Three => "level-three",
        }
    }

    /// Human-readable label, for the debug panel.
    pub fn label(self) -> &'static str {
        match self {
            LevelKind::One => "One",
            LevelKind::Two => "Two",
            LevelKind::Three => "Three",
        }
    }

    /// The next level in progression. Completing FinishLine on One
    /// moves the player to Two, and so on. Three loops to itself —
    /// endgame, no further progression yet.
    pub fn next(self) -> LevelKind {
        match self {
            LevelKind::One => LevelKind::Two,
            LevelKind::Two => LevelKind::Three,
            LevelKind::Three => LevelKind::Three,
        }
    }

    /// Cycle for the debug panel: 1 → 2 → 3 → 1.
    pub fn cycle(self) -> LevelKind {
        match self {
            LevelKind::One => LevelKind::Two,
            LevelKind::Two => LevelKind::Three,
            LevelKind::Three => LevelKind::One,
        }
    }
}

pub fn generate(seed: u32, kind: LevelKind) -> World {
    let p = kind.params();
    let mut rng = Rng::new(seed);
    let mut cells: Vec<Option<Tile>> = vec![None; p.width * p.height];

    generate_terrain(&mut cells, &p, &mut rng);
    for spec in p.veins {
        scatter_veins(&mut cells, &p, &mut rng, spec);
    }
    fill_bedrock(&mut cells, &p);

    World::from_cells(p.width, p.height, cells, p.surface_row, seed)
}

// ---------- Terrain ----------

fn generate_terrain(cells: &mut [Option<Tile>], p: &LevelParams, rng: &mut Rng) {
    for col in 0..p.plain_end_col {
        fill_column(cells, p, col, p.surface_row);
    }

    let lo = p.surface_row.saturating_sub(6) as i32;
    let hi = p.surface_row.saturating_sub(1) as i32;
    let mut top = p.surface_row.saturating_sub(3) as i32;

    let mut slow_v: f32 = 0.0;
    let mut fast_v: f32 = 0.0;

    for col in p.plain_end_col..p.width {
        slow_v = slow_v * 0.88 + rng.range(-1, 2) as f32 * 0.65;
        slow_v = slow_v.clamp(-1.4, 1.4);

        fast_v = fast_v * 0.35 + rng.range(-1, 2) as f32 * 0.55;
        fast_v = fast_v.clamp(-0.9, 0.9);

        let step = (slow_v + fast_v).round() as i32;
        top = (top + step).clamp(lo, hi);
        fill_column(cells, p, col, top as usize);
    }
}

fn fill_column(cells: &mut [Option<Tile>], p: &LevelParams, col: usize, top: usize) {
    let dirt_bottom = p.surface_row + 4;
    for row in top..p.height {
        let kind = if row <= dirt_bottom {
            TileKind::Dirt
        } else {
            TileKind::Stone
        };
        cells[row * p.width + col] = Some(Tile::new(kind));
    }
}

fn fill_bedrock(cells: &mut [Option<Tile>], p: &LevelParams) {
    let start = p.height - p.bedrock_rows;
    for row in start..p.height {
        for col in 0..p.width {
            cells[row * p.width + col] = Some(Tile::new(TileKind::Bedrock));
        }
    }
}

// ---------- Veins ----------

fn scatter_veins(
    cells: &mut [Option<Tile>],
    p: &LevelParams,
    rng: &mut Rng,
    spec: &VeinSpec,
) {
    let bottom_limit = p.height - p.bedrock_rows;
    let row_min = (p.surface_row + spec.row_min_offset).min(bottom_limit);
    let row_max = (p.surface_row + spec.row_max_offset + 1).min(bottom_limit);
    if row_max <= row_min {
        return;
    }

    for _ in 0..spec.count {
        let mut seed_pos = None;
        for _ in 0..8 {
            let col = rng.range(1, p.width as i32 - 1) as usize;
            let row = rng.range(row_min as i32, row_max as i32) as usize;
            if let Some(Some(t)) = cells.get(row * p.width + col)
                && matches!(t.kind, TileKind::Dirt | TileKind::Stone)
            {
                seed_pos = Some((col, row));
                break;
            }
        }
        let Some((col, row)) = seed_pos else {
            continue;
        };

        let size = rng.range(spec.size_min as i32, spec.size_max as i32 + 1) as u32;
        grow_vein(cells, p, rng, col, row, spec.kind, size, row_min, row_max);
    }
}

#[allow(clippy::too_many_arguments)]
fn grow_vein(
    cells: &mut [Option<Tile>],
    p: &LevelParams,
    rng: &mut Rng,
    col0: usize,
    row0: usize,
    kind: TileKind,
    size: u32,
    row_min: usize,
    row_max: usize,
) {
    let bottom_limit = p.height - p.bedrock_rows;
    let mut placed = 0u32;
    let mut frontier: Vec<(usize, usize)> = vec![(col0, row0)];

    while placed < size && !frontier.is_empty() {
        let i = rng.range(0, frontier.len() as i32) as usize;
        let (col, row) = frontier.swap_remove(i);

        if row < row_min || row >= row_max {
            continue;
        }
        if row < p.surface_row || row >= bottom_limit {
            continue;
        }

        match cells[row * p.width + col] {
            Some(t) if matches!(t.kind, TileKind::Dirt | TileKind::Stone) => {}
            _ => continue,
        }

        cells[row * p.width + col] = Some(Tile::new(kind));
        placed += 1;

        if col > 0 {
            frontier.push((col - 1, row));
        }
        if col + 1 < p.width {
            frontier.push((col + 1, row));
        }
        if row > 0 {
            frontier.push((col, row - 1));
        }
        if row + 1 < p.height {
            frontier.push((col, row + 1));
        }
    }
}

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;

    fn count_kind(w: &World, kind: TileKind) -> usize {
        let mut n = 0;
        for row in 0..w.height() {
            for col in 0..w.width() {
                if w.tile(col, row).map(|t| t.kind) == Some(kind) {
                    n += 1;
                }
            }
        }
        n
    }

    const ALL_LEVELS: [LevelKind; 3] = [LevelKind::One, LevelKind::Two, LevelKind::Three];

    #[test]
    fn generate_is_deterministic() {
        for kind in ALL_LEVELS {
            let a = generate(42, kind);
            let b = generate(42, kind);
            assert_eq!(a.width(), b.width());
            for row in 0..a.height() {
                for col in 0..a.width() {
                    assert_eq!(
                        a.tile(col, row).map(|t| t.kind),
                        b.tile(col, row).map(|t| t.kind),
                        "{kind:?} mismatch at ({col}, {row})",
                    );
                }
            }
        }
    }

    #[test]
    fn different_seeds_differ() {
        let a = generate(1, LevelKind::One);
        let b = generate(2, LevelKind::One);
        let mut diffs = 0;
        for row in 0..a.height() {
            for col in 0..a.width() {
                if a.tile(col, row).map(|t| t.kind)
                    != b.tile(col, row).map(|t| t.kind)
                {
                    diffs += 1;
                }
            }
        }
        assert!(diffs > 50, "only {diffs} cell differences");
    }

    #[test]
    fn dims_match_params() {
        for kind in ALL_LEVELS {
            let p = kind.params();
            let w = generate(42, kind);
            assert_eq!(w.width(), p.width);
            assert_eq!(w.height(), p.height);
            assert_eq!(w.surface_row, p.surface_row);
        }
    }

    #[test]
    fn top_row_is_all_air() {
        for kind in ALL_LEVELS {
            let w = generate(42, kind);
            for col in 0..w.width() {
                assert!(w.tile(col, 0).is_none());
            }
        }
    }

    #[test]
    fn bottom_rows_are_bedrock() {
        for kind in ALL_LEVELS {
            let p = kind.params();
            let w = generate(7, kind);
            for row in (p.height - p.bedrock_rows)..p.height {
                for col in 0..p.width {
                    assert_eq!(
                        w.tile(col, row).map(|t| t.kind),
                        Some(TileKind::Bedrock),
                        "{kind:?} ({col}, {row})",
                    );
                }
            }
        }
    }

    #[test]
    fn plain_surface_is_at_surface_row() {
        for kind in ALL_LEVELS {
            let p = kind.params();
            let w = generate(42, kind);
            for col in 0..p.plain_end_col {
                assert!(w.tile(col, p.surface_row - 1).is_none());
                assert!(w.tile(col, p.surface_row).is_some());
            }
        }
    }

    #[test]
    fn mountain_rises_above_plain() {
        for kind in ALL_LEVELS {
            let p = kind.params();
            let w = generate(42, kind);
            let mut rises = false;
            for col in p.plain_end_col..w.width() {
                for row in 0..p.surface_row {
                    if w.tile(col, row).is_some() {
                        rises = true;
                        break;
                    }
                }
                if rises {
                    break;
                }
            }
            assert!(rises, "{kind:?} mountain never rises above the plain");
        }
    }

    #[test]
    fn mountain_profile_has_variation() {
        for kind in ALL_LEVELS {
            let p = kind.params();
            let w = generate(42, kind);
            let mut heights: Vec<usize> = Vec::new();
            for col in p.plain_end_col..w.width() {
                let mut top = p.height;
                for row in 0..p.height {
                    if w.tile(col, row).is_some() {
                        top = row;
                        break;
                    }
                }
                heights.push(top);
            }
            let min_h = *heights.iter().min().unwrap();
            let max_h = *heights.iter().max().unwrap();
            assert!(
                max_h >= min_h + 3,
                "{kind:?} profile too flat: min {min_h}, max {max_h}",
            );
        }
    }

    #[test]
    fn level_one_has_coal_and_iron() {
        let w = generate(42, LevelKind::One);
        assert!(count_kind(&w, TileKind::Coal) >= 30);
        assert!(count_kind(&w, TileKind::Iron) >= 20);
    }

    #[test]
    fn level_one_has_no_crystal() {
        let w = generate(42, LevelKind::One);
        assert_eq!(count_kind(&w, TileKind::Crystal), 0);
    }

    #[test]
    fn level_one_has_no_hardrock() {
        let w = generate(42, LevelKind::One);
        assert_eq!(count_kind(&w, TileKind::HardRock), 0);
    }

    #[test]
    fn level_two_has_crystal() {
        let w = generate(42, LevelKind::Two);
        assert!(count_kind(&w, TileKind::Crystal) >= 5);
    }

    #[test]
    fn level_two_has_hardrock() {
        let w = generate(42, LevelKind::Two);
        assert!(count_kind(&w, TileKind::HardRock) >= 5);
    }

    #[test]
    fn level_three_generates() {
        let p = LevelKind::Three.params();
        let w = generate(42, LevelKind::Three);
        assert_eq!(w.width(), p.width);
        assert!(count_kind(&w, TileKind::Crystal) >= 8);
        assert!(count_kind(&w, TileKind::HardRock) >= 10);
    }

    #[test]
    fn level_three_is_harder_than_two() {
        let two = generate(42, LevelKind::Two);
        let three = generate(42, LevelKind::Three);
        let hr_two = count_kind(&two, TileKind::HardRock);
        let hr_three = count_kind(&three, TileKind::HardRock);
        assert!(
            hr_three > hr_two,
            "level three hardrock ({hr_three}) should exceed level two ({hr_two})",
        );
    }

    #[test]
    fn level_three_is_not_softlocked() {
        for seed in [1u32, 42, 999] {
            let w = generate(seed, LevelKind::Three);
            assert!(
                bfs_reaches_right_edge(&w),
                "seed {seed} softlocked",
            );
        }
    }

    #[test]
    fn level_two_is_never_softlocked() {
        for seed in [1u32, 2, 3, 7, 42, 99, 12345, 54321, 999_999] {
            let w = generate(seed, LevelKind::Two);
            assert!(bfs_reaches_right_edge(&w), "seed {seed} softlocked");
        }
    }

    #[test]
    fn veins_never_appear_above_surface() {
        for kind in ALL_LEVELS {
            let p = kind.params();
            let w = generate(42, kind);
            for row in 0..p.surface_row {
                for col in 0..p.width {
                    let k = w.tile(col, row).map(|t| t.kind);
                    assert!(
                        !matches!(
                            k,
                            Some(TileKind::Coal)
                                | Some(TileKind::Iron)
                                | Some(TileKind::Crystal)
                        ),
                        "{kind:?} vein above surface at ({col}, {row})",
                    );
                }
            }
        }
    }

    #[test]
    fn veins_are_not_bedrock() {
        for kind in ALL_LEVELS {
            let p = kind.params();
            let w = generate(99, kind);
            for row in (p.height - p.bedrock_rows)..p.height {
                for col in 0..p.width {
                    assert_eq!(
                        w.tile(col, row).map(|t| t.kind),
                        Some(TileKind::Bedrock),
                        "{kind:?} bedrock overwritten at ({col}, {row})",
                    );
                }
            }
        }
    }

    #[test]
    fn hardrock_never_appears_above_surface_plus_six() {
        for kind in [LevelKind::Two, LevelKind::Three] {
            let p = kind.params();
            for seed in [1u32, 42, 99, 12345] {
                let w = generate(seed, kind);
                for row in 0..(p.surface_row + 6) {
                    for col in 0..p.width {
                        assert_ne!(
                            w.tile(col, row).map(|t| t.kind),
                            Some(TileKind::HardRock),
                            "{kind:?} seed {seed} hardrock too high at ({col}, {row})",
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn hardrock_never_touches_bedrock() {
        for kind in [LevelKind::Two, LevelKind::Three] {
            let p = kind.params();
            let top_bedrock = p.height - p.bedrock_rows;
            for seed in [1u32, 42, 99, 12345] {
                let w = generate(seed, kind);
                for col in 0..p.width {
                    assert_ne!(
                        w.tile(col, top_bedrock - 1).map(|t| t.kind),
                        Some(TileKind::HardRock),
                        "{kind:?} seed {seed} hardrock at bedrock boundary ({col})",
                    );
                }
            }
        }
    }

    #[test]
    fn level_ids_are_distinct() {
        let ids = [
            LevelKind::One.id(),
            LevelKind::Two.id(),
            LevelKind::Three.id(),
        ];
        for (i, a) in ids.iter().enumerate() {
            for b in ids.iter().skip(i + 1) {
                assert_ne!(a, b);
            }
        }
    }

    #[test]
    fn next_advances_through_levels() {
        assert_eq!(LevelKind::One.next(), LevelKind::Two);
        assert_eq!(LevelKind::Two.next(), LevelKind::Three);
        assert_eq!(LevelKind::Three.next(), LevelKind::Three);
    }

    #[test]
    fn cycle_wraps_around() {
        assert_eq!(LevelKind::One.cycle(), LevelKind::Two);
        assert_eq!(LevelKind::Two.cycle(), LevelKind::Three);
        assert_eq!(LevelKind::Three.cycle(), LevelKind::One);
    }

    fn is_mineable_or_air(world: &World, col: usize, row: usize) -> bool {
        match world.tile(col, row).map(|t| t.kind) {
            None => true,
            Some(TileKind::HardRock) | Some(TileKind::Bedrock) => false,
            _ => true,
        }
    }

    fn bfs_reaches_right_edge(world: &World) -> bool {
        use std::collections::VecDeque;
        let w = world.width();
        let h = world.height();
        let mut visited = vec![false; w * h];
        let mut queue = VecDeque::new();

        visited[0] = true;
        queue.push_back((0usize, 0usize));

        while let Some((col, row)) = queue.pop_front() {
            if col == w - 1 {
                return true;
            }
            for (dc, dr) in [(0i32, -1i32), (0, 1), (-1, 0), (1, 0)] {
                let nc = col as i32 + dc;
                let nr = row as i32 + dr;
                if nc < 0 || nr < 0 || nc >= w as i32 || nr >= h as i32 {
                    continue;
                }
                let (nc, nr) = (nc as usize, nr as usize);
                if visited[nr * w + nc] {
                    continue;
                }
                if !is_mineable_or_air(world, nc, nr) {
                    continue;
                }
                visited[nr * w + nc] = true;
                queue.push_back((nc, nr));
            }
        }
        false
    }
}