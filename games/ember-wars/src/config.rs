//! Contexte global + chargement lazy des données `.ron`.

use std::path::Path;
use std::sync::OnceLock;

use ember_core::io::load_from_file;
use ember_stdlib::config::{env_color, env_f32, load_dotenv_once};
use macroquad::prelude::Color;
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct GameContext {
    pub colors: Colors,
    pub viewport_w: f32,
    pub viewport_h: f32,
}

#[derive(Debug, Clone)]
pub struct Colors {
    pub bg: Color,
    pub ground: Color,
    pub player: Color,
    pub enemy: Color,
    pub mana: Color,
    pub gold: Color,
    pub hp_player: Color,
    pub hp_enemy: Color,
    pub turret: Color,
    pub text: Color,
    pub accent: Color,
}

impl GameContext {
    pub fn from_env() -> Self {
        load_dotenv_once(Path::new(env!("CARGO_MANIFEST_DIR")), "ember-wars");

        Self {
            colors: Colors {
                bg: env_color("COLOR_BG", Color::new(0.06, 0.07, 0.09, 1.0)),
                ground: env_color("COLOR_GROUND", Color::new(0.14, 0.16, 0.20, 1.0)),
                player: env_color("COLOR_PLAYER", Color::new(0.35, 0.65, 1.00, 1.0)),
                enemy: env_color("COLOR_ENEMY", Color::new(1.00, 0.35, 0.35, 1.0)),
                mana: env_color("COLOR_MANA", Color::new(0.40, 0.65, 1.00, 1.0)),
                gold: env_color("COLOR_GOLD", Color::new(1.00, 0.82, 0.30, 1.0)),
                hp_player: env_color("COLOR_HP_PLAYER", Color::new(0.35, 0.85, 0.45, 1.0)),
                hp_enemy: env_color("COLOR_HP_ENEMY", Color::new(0.95, 0.30, 0.30, 1.0)),
                turret: env_color("COLOR_TURRET", Color::new(0.90, 0.90, 0.95, 1.0)),
                text: env_color("COLOR_TEXT", Color::new(0.92, 0.94, 0.98, 1.0)),
                accent: env_color("COLOR_ACCENT", Color::new(1.00, 0.82, 0.30, 1.0)),
            },
            viewport_w: env_f32("VIEWPORT_W", 1280.0),
            viewport_h: env_f32("VIEWPORT_H", 720.0),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub enum Objective {
    Destroy,
    Survive(f32),
}

#[derive(Debug, Clone, Deserialize)]
pub struct AiWave {
    pub start_at: f32,
    pub priority: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AiConfig {
    pub aggression: f32,
    pub mana_regen_mult: f32,
    pub waves: Vec<AiWave>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LevelConfig {
    pub id: String,
    pub name: String,
    pub palette: String,
    pub width: f32,
    pub tower_hp: f32,
    pub objective: Objective,
    pub reward_gold: f32,
    pub ai: AiConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChapterConfig {
    pub id: String,
    pub name: String,
    pub subtitle: String,
    pub levels: Vec<LevelConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Balance {
    pub mana_start: f32,
    pub mana_max: f32,
    pub mana_regen: f32,
    pub mana_per_kill: f32,
    pub gold_per_kill: f32,
    pub gold_start: f32,
}

static CHAPTERS: OnceLock<Vec<ChapterConfig>> = OnceLock::new();
static BALANCE: OnceLock<Balance> = OnceLock::new();

pub fn all_chapters() -> &'static [ChapterConfig] {
    CHAPTERS
        .get_or_init(|| {
            let path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("assets")
                .join("chapters.ron");
            load_from_file(&path).expect("assets/chapters.ron should load")
        })
        .as_slice()
}

pub fn chapter_by_id(id: &str) -> Option<&'static ChapterConfig> {
    all_chapters().iter().find(|c| c.id == id)
}

pub fn level_by_id(id: &str) -> Option<&'static LevelConfig> {
    all_chapters()
        .iter()
        .flat_map(|c| c.levels.iter())
        .find(|l| l.id == id)
}

pub fn chapter_of_level(level_id: &str) -> Option<&'static ChapterConfig> {
    all_chapters()
        .iter()
        .find(|c| c.levels.iter().any(|l| l.id == level_id))
}

pub fn balance() -> &'static Balance {
    BALANCE.get_or_init(|| {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join("balance.ron");
        load_from_file(&path).expect("assets/balance.ron should load")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chapters_ron_loads_two_chapters() {
        assert_eq!(all_chapters().len(), 2);
    }

    #[test]
    fn each_chapter_has_three_levels() {
        for c in all_chapters() {
            assert_eq!(c.levels.len(), 3);
        }
    }

    #[test]
    fn all_level_ids_are_unique() {
        let mut ids: Vec<&str> = all_chapters()
            .iter()
            .flat_map(|c| c.levels.iter().map(|l| l.id.as_str()))
            .collect();
        ids.sort_unstable();
        let before = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), before);
    }

    #[test]
    fn level_by_id_finds_level() {
        assert!(level_by_id("sh_01").is_some());
        assert!(level_by_id("gy_03").is_some());
        assert!(level_by_id("nope").is_none());
    }

    #[test]
    fn chapter_of_level_works() {
        assert_eq!(chapter_of_level("sh_01").unwrap().id, "shanghai");
        assert_eq!(chapter_of_level("gy_02").unwrap().id, "guiyang");
        assert!(chapter_of_level("nope").is_none());
    }

    #[test]
    fn all_levels_have_a_palette() {
        for c in all_chapters() {
            for l in &c.levels {
                assert!(!l.palette.is_empty(), "level {} has no palette", l.id);
            }
        }
    }

    #[test]
    fn balance_loads() {
        let b = balance();
        assert!(b.mana_start > 0.0);
        assert!(b.mana_regen > 0.0);
    }
}