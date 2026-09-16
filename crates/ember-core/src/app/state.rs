// crates/ember-core/src/app/state.rs
//! États de jeu.

/// État global d'un jeu.
///
/// Tous les jeux Ember partagent ce squelette :
/// - `Start` : écran titre, en attente d'un input pour commencer.
/// - `Playing` : le jeu tourne.
/// - `LevelCleared` : transition entre niveaux (optionnel).
/// - `GameOver` : le joueur a perdu.
/// - `Win` : le joueur a gagné.
///
/// **Convention** : les jeux qui n'utilisent pas `LevelCleared` ne
/// l'assignent jamais. Pas de coût, juste une variante disponible.
#[derive(PartialEq, Eq, Debug, Clone, Copy, Hash)]
pub enum GameState {
    Start,
    Playing,
    LevelCleared,
    GameOver,
    Win,
}

impl GameState {
    /// Vrai si le jeu est dans un état "actif" (Playing ou LevelCleared).
    pub fn is_active(self) -> bool {
        matches!(self, GameState::Playing | GameState::LevelCleared)
    }

    /// Vrai si c'est un état terminal (GameOver ou Win).
    pub fn is_terminal(self) -> bool {
        matches!(self, GameState::GameOver | GameState::Win)
    }

    /// Vrai si c'est l'écran titre.
    pub fn is_start(self) -> bool {
        self == GameState::Start
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_active() {
        assert!(GameState::Playing.is_active());
        assert!(GameState::LevelCleared.is_active());
        assert!(!GameState::Start.is_active());
        assert!(!GameState::GameOver.is_active());
        assert!(!GameState::Win.is_active());
    }

    #[test]
    fn test_is_terminal() {
        assert!(GameState::GameOver.is_terminal());
        assert!(GameState::Win.is_terminal());
        assert!(!GameState::Playing.is_terminal());
        assert!(!GameState::Start.is_terminal());
    }

    #[test]
    fn test_is_start() {
        assert!(GameState::Start.is_start());
        assert!(!GameState::Playing.is_start());
    }
}
