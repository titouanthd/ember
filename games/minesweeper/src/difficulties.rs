//! Difficulty definitions, loaded from `difficulties.ron`.

use std::path::PathBuf;

use serde::Deserialize;

use crate::config::GameContext;

#[derive(Debug, Clone, Deserialize)]
pub struct DifficultyData {
    pub name: String,
    pub width: usize,
    pub height: usize,
    pub mines: usize,
}

impl DifficultyData {
    /// Cell size so that the board fits the playfield with a small margin.
    pub fn cell_size(&self, ctx: &GameContext) -> f32 {
        let avail_w = ctx.window_w - 40.0; // 20px margin each side
        let avail_h = ctx.playfield_h() - 20.0;
        let by_w = avail_w / self.width as f32;
        let by_h = avail_h / self.height as f32;
        // Cap to keep tiny difficulties from looking ridiculous, floor so
        // Expert cells stay clickable.
        by_w.min(by_h).clamp(18.0, 44.0)
    }
}

/// Load difficulties from `difficulties.ron`. Falls back to a single
/// Beginner config if the file is missing or invalid.
pub fn load_difficulties() -> Vec<DifficultyData> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("difficulties.ron");
    match ember_core::io::load_from_file::<Vec<DifficultyData>>(&path) {
        Ok(list) if !list.is_empty() => list,
        _ => fallback(),
    }
}

fn fallback() -> Vec<DifficultyData> {
    vec![DifficultyData {
        name: "Beginner".into(),
        width: 9,
        height: 9,
        mines: 10,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_has_one() {
        let list = fallback();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].width, 9);
    }

    #[test]
    fn test_load_returns_something() {
        // Should load the .ron file if present, fallback otherwise.
        // Either way, non-empty.
        let list = load_difficulties();
        assert!(!list.is_empty());
    }

    #[test]
    fn test_cell_size_fits_playfield() {
        let ctx = crate::config::load_config();
        let d = DifficultyData {
            name: "Test".into(),
            width: 30,
            height: 16,
            mines: 99,
        };
        let cs = d.cell_size(&ctx);
        assert!(cs >= 18.0);
        assert!(cs <= 44.0);
        // Board must fit horizontally.
        assert!(d.width as f32 * cs <= ctx.window_w);
    }
}