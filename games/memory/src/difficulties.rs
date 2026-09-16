//! Difficulty definitions. Loaded from `difficulties.ron` (same format
//! as Minesweeper).

use std::path::PathBuf;

use serde::Deserialize;

use crate::config::GameContext;

#[derive(Debug, Clone, Deserialize)]
pub struct DifficultyData {
    pub name: String,
    pub cols: usize,
    pub rows: usize,
    /// Number of distinct symbols used. Must be `(cols * rows) / 2`.
    pub pairs: usize,
}

impl DifficultyData {
    /// Card size so that the board fits the playfield with a small margin.
    /// Cards are square. The smaller of the horizontal/vertical fits wins.
    pub fn card_size(&self, ctx: &GameContext) -> f32 {
        let avail_w = ctx.window_w - ctx.board_margin * 2.0;
        let avail_h = ctx.playfield_h() - ctx.board_margin * 2.0;
        // Each card takes `card_size`, plus `card_gap` between cards.
        // Total width  = cols * card + (cols - 1) * gap
        // Total height = rows * card + (rows - 1) * gap
        // Solve for card_size on both axes, take the min.
        let by_w =
            (avail_w - (self.cols.saturating_sub(1) as f32) * ctx.card_gap) / self.cols as f32;
        let by_h =
            (avail_h - (self.rows.saturating_sub(1) as f32) * ctx.card_gap) / self.rows as f32;
        by_w.min(by_h).clamp(20.0, 120.0)
    }

    /// Total number of cards on the board.
    pub fn card_count(&self) -> usize {
        self.cols * self.rows
    }

    /// Sanity check: `pairs * 2` must equal `card_count`.
    pub fn is_valid(&self) -> bool {
        self.pairs * 2 == self.card_count() && self.pairs > 0
    }
}

/// Load difficulties from `difficulties.ron`. Falls back to a single
/// Easy config if the file is missing or invalid.
pub fn load_difficulties() -> Vec<DifficultyData> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("difficulties.ron");
    match ember_core::io::load_from_file::<Vec<DifficultyData>>(&path) {
        Ok(list) if !list.is_empty() => list,
        _ => fallback(),
    }
}

fn fallback() -> Vec<DifficultyData> {
    vec![DifficultyData {
        name: "Easy".into(),
        cols: 4,
        rows: 4,
        pairs: 8,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_is_valid() {
        let list = fallback();
        assert_eq!(list.len(), 1);
        assert!(list[0].is_valid());
        assert_eq!(list[0].card_count(), 16);
    }

    #[test]
    fn test_load_returns_something() {
        let list = load_difficulties();
        assert!(!list.is_empty());
        for d in &list {
            assert!(d.is_valid(), "difficulty {} is invalid", d.name);
        }
    }

    #[test]
    fn test_card_size_fits_easy() {
        let ctx = crate::config::load_config();
        let d = DifficultyData {
            name: "Test".into(),
            cols: 8,
            rows: 8,
            pairs: 32,
        };
        let cs = d.card_size(&ctx);
        assert!(cs >= 20.0);
        assert!(cs <= 120.0);
    }
}
