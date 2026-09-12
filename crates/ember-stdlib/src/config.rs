// ember_stdlib/src/config.rs
//! Configuration par variables d'environnement.
//!
//! Fournit des helpers pour lire un `.env` et parser des valeurs typées.
//! Chaque jeu appelle `load_dotenv_from` une fois, puis lit ses valeurs
//! avec `env_f32`, `env_i32`, `env_u32`, `env_color`.

use macroquad::prelude::Color;
use std::path::Path;

/// Charge un fichier `.env` situé dans `manifest_dir`.
///
/// À appeler depuis le `config.rs` de chaque jeu, avec :
/// ```ignore
/// let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
/// ember_stdlib::config::load_dotenv_from(&manifest_dir, "Snake");
/// ```
pub fn load_dotenv_from(manifest_dir: &Path, game_name: &str) {
    let env_path = manifest_dir.join(".env");
    if env_path.exists() {
        dotenvy::from_path(&env_path).ok();
        println!("✅ Fichier .env chargé pour {}", game_name);
    } else {
        println!("⚠️  Aucun .env trouvé pour {}, valeurs par défaut", game_name);
    }
}

/// Lit une variable d'environnement `f32`, ou retourne la valeur par défaut.
pub fn env_f32(key: &str, default: f32) -> f32 {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

/// Lit une variable d'environnement `i32`, ou retourne la valeur par défaut.
pub fn env_i32(key: &str, default: i32) -> i32 {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

/// Lit une variable d'environnement `u32`, ou retourne la valeur par défaut.
pub fn env_u32(key: &str, default: u32) -> u32 {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

/// Lit une couleur RGBA depuis 4 variables d'environnement.
pub fn env_color(prefix: &str, default: Color) -> Color {
    let r = env_f32(&format!("{}_R", prefix), default.r);
    let g = env_f32(&format!("{}_G", prefix), default.g);
    let b = env_f32(&format!("{}_B", prefix), default.b);
    let a = env_f32(&format!("{}_A", prefix), default.a);
    Color::new(r, g, b, a)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_f32_returns_default_when_missing() {
        let v = env_f32("EMBER_TEST_DOES_NOT_EXIST_12345", 42.5);
        assert_eq!(v, 42.5);
    }

    #[test]
    fn test_env_i32_returns_default_when_missing() {
        let v = env_i32("EMBER_TEST_DOES_NOT_EXIST_12345", -7);
        assert_eq!(v, -7);
    }

    #[test]
    fn test_env_u32_returns_default_when_missing() {
        let v = env_u32("EMBER_TEST_DOES_NOT_EXIST_12345", 99);
        assert_eq!(v, 99);
    }

    #[test]
    fn test_env_color_returns_default_when_missing() {
        let default = Color::new(0.1, 0.2, 0.3, 0.4);
        let c = env_color("EMBER_TEST_COLOR_DOES_NOT_EXIST_12345", default);
        assert_eq!(c.r, 0.1);
        assert_eq!(c.g, 0.2);
        assert_eq!(c.b, 0.3);
        assert_eq!(c.a, 0.4);
    }
}