//! Endgame: choose Neutron Star (peace) or Black Hole (transcendence).
//!
//! DESIGN §11. Reachable at `Paradigm::Star` when Light ≥ threshold.
//! Choosing an ending records it in `endings_seen` and auto-prestiges
//! (the run is over — bank the G, reset for next run).

use crate::components::{Ending, Paradigm, ResourceId};
use crate::prestige;
use crate::systems::Game;

/// Light required at Star paradigm to trigger the endgame choice.
/// DESIGN §9.7 says 50,000, lowered to 10,000 for V1.3 balance.
pub const ENDGAME_LIGHT_THRESHOLD: f64 = 10_000.0;

/// Whether the player can trigger the endgame choice right now.
pub fn can_endgame(game: &Game) -> bool {
    game.paradigm == Paradigm::Star
        && game.resources.get(&ResourceId::Light).copied().unwrap_or(0.0)
            >= ENDGAME_LIGHT_THRESHOLD
}

/// Apply the chosen ending: record it, auto-prestige, reset the run.
/// Returns the G gain from the auto-prestige (0 if not prestigeable,
/// though in practice Star always is).
pub fn choose_ending(game: &mut Game, ending: Ending) -> u64 {
    assert!(can_endgame(game), "endgame requires Star + threshold Light");
    if !game.endings_seen.contains(&ending) {
        game.endings_seen.push(ending);
    }
    // Auto-prestige: `prestige::prestige` calls `reset_run`, which
    // preserves `endings_seen`, `g_total`, `g_multiplier`.
    if prestige::can_prestige(game) {
        prestige::prestige(game)
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at_star(light: f64) -> Game {
        let mut g = Game::new();
        g.paradigm = Paradigm::Star;
        *g.resources.get_mut(&ResourceId::Light).unwrap() = light;
        // Give some mass so prestige actually yields G.
        *g.resources.get_mut(&ResourceId::Dust).unwrap() = 50_000.0;
        g
    }

    #[test]
    fn can_endgame_false_before_star() {
        let mut g = Game::new();
        g.paradigm = Paradigm::GasGiant;
        *g.resources.get_mut(&ResourceId::Light).unwrap() = 100_000.0;
        assert!(!can_endgame(&g));
    }

    #[test]
    fn can_endgame_false_below_threshold() {
        let g = at_star(ENDGAME_LIGHT_THRESHOLD - 1.0);
        assert!(!can_endgame(&g));
    }

    #[test]
    fn can_endgame_true_at_threshold() {
        let g = at_star(ENDGAME_LIGHT_THRESHOLD);
        assert!(can_endgame(&g));
    }

    #[test]
    fn can_endgame_true_above_threshold() {
        let g = at_star(ENDGAME_LIGHT_THRESHOLD * 2.0);
        assert!(can_endgame(&g));
    }

    #[test]
    fn choose_ending_adds_to_endings_seen() {
        let mut g = at_star(ENDGAME_LIGHT_THRESHOLD);
        choose_ending(&mut g, Ending::NeutronStar);
        assert!(g.endings_seen.contains(&Ending::NeutronStar));
    }

    #[test]
    fn choose_ending_is_idempotent() {
        let mut g = at_star(ENDGAME_LIGHT_THRESHOLD);
        choose_ending(&mut g, Ending::NeutronStar);
        let after_first = g.endings_seen.len();
        // Second ending requires new run: simulate
        g.paradigm = Paradigm::Star;
        *g.resources.get_mut(&ResourceId::Light).unwrap() = ENDGAME_LIGHT_THRESHOLD;
        choose_ending(&mut g, Ending::NeutronStar);
        assert_eq!(g.endings_seen.len(), after_first);
    }

    #[test]
    fn choose_ending_prestiges() {
        let mut g = at_star(ENDGAME_LIGHT_THRESHOLD);
        let gain = choose_ending(&mut g, Ending::NeutronStar);
        assert!(gain >= 1);
        assert_eq!(g.paradigm, Paradigm::Dust);
        assert_eq!(g.g_total, gain);
    }

    #[test]
    fn both_endings_are_reachable() {
        let mut g1 = at_star(ENDGAME_LIGHT_THRESHOLD);
        choose_ending(&mut g1, Ending::NeutronStar);
        assert!(g1.endings_seen.contains(&Ending::NeutronStar));

        let mut g2 = at_star(ENDGAME_LIGHT_THRESHOLD);
        choose_ending(&mut g2, Ending::BlackHole);
        assert!(g2.endings_seen.contains(&Ending::BlackHole));
    }

    #[test]
    fn endings_persist_through_two_runs() {
        let mut g = at_star(ENDGAME_LIGHT_THRESHOLD);
        choose_ending(&mut g, Ending::NeutronStar);
        // Post-prestige, back to Dust. Simulate reaching Star again.
        g.paradigm = Paradigm::Star;
        *g.resources.get_mut(&ResourceId::Light).unwrap() = ENDGAME_LIGHT_THRESHOLD;
        *g.resources.get_mut(&ResourceId::Dust).unwrap() = 50_000.0;
        choose_ending(&mut g, Ending::BlackHole);
        assert_eq!(g.endings_seen.len(), 2);
        assert!(g.endings_seen.contains(&Ending::NeutronStar));
        assert!(g.endings_seen.contains(&Ending::BlackHole));
    }
}