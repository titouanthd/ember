//! Prestige: collapse the run into a Gravitational Constant bonus.
//!
//! DESIGN §10. The run is reset, G is awarded based on the mass
//! accumulated, and `g_multiplier` is recomputed. The multiplier is
//! consumed by `economy::compute_multipliers` on every tick.

use crate::components::Paradigm;
use crate::economy::id_to_def;
use crate::systems::Game;

/// Minimum paradigm to prestige.
///
/// DESIGN §10 says Planets, but Planets runs are 2.4 min with optimal
/// play — too short to be worth a reset. Session 13 tuning pushes to
/// Gas Giant (the first "real" paradigm).
pub const MIN_PARADIGM: Paradigm = Paradigm::GasGiant;

/// Minimum G gain per prestige (avoids zero-gain edge cases).
pub const MIN_G_GAIN: u64 = 1;

/// `g_multiplier = 1.0 + G_MULTIPLIER_PER_POINT × g_total`.
pub const G_MULTIPLIER_PER_POINT: f64 = 0.05;

/// Whether the current run may prestige.
pub fn can_prestige(game: &Game) -> bool {
    game.paradigm >= MIN_PARADIGM
}

/// Paradigms reached in the current run (1 = Dust only).
pub fn paradigms_reached(game: &Game) -> u8 {
    game.paradigm as u8 + 1
}

/// Value of the current run (DESIGN §10 "total_mass_run").
/// Held resources + sum(buildings × base_cost).
pub fn total_mass_run(game: &Game) -> f64 {
    let resources: f64 = game.resources.values().sum();
    let buildings: f64 = game
        .buildings
        .iter()
        .map(|b| {
            let base: f64 = id_to_def(b.id).base_cost.iter().map(|(_, v)| *v).sum();
            b.count as f64 * base
        })
        .sum();
    resources + buildings
}

/// Compute the G gained by prestiging now.
pub fn compute_g_gain(game: &Game) -> u64 {
    let mass = total_mass_run(game);
    let paradigms = paradigms_reached(game) as f64;
    let raw = (mass + 1.0).log10() * (1.0 + 0.1 * paradigms);
    (raw.floor() as u64).max(MIN_G_GAIN)
}

/// Apply the prestige. Returns the G gain (for logging / UI).
///
/// The run is reset via `Game::new()`, then `g_total`, `g_multiplier`
/// and the log are set to their post-collapse values.
pub fn prestige(game: &mut Game) -> u64 {
    assert!(can_prestige(game), "prestige requires paradigm >= {MIN_PARADIGM:?}");

    let gain = compute_g_gain(game);
    let new_total = game.g_total + gain;
    let new_multiplier = 1.0 + G_MULTIPLIER_PER_POINT * new_total as f64;

    game.reset_run();
    game.g_total = new_total;
    game.g_multiplier = new_multiplier;
    game.push_log(format!("Collapse. G +{gain}."));
    game.push_log("I fall again.".to_string());

    gain
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::ResourceId;

    fn at_paradigm(p: Paradigm) -> Game {
        let mut g = Game::new();
        g.paradigm = p;
        g
    }

    #[test]
    fn cannot_prestige_at_dust() {
        assert!(!can_prestige(&at_paradigm(Paradigm::Dust)));
    }

    #[test]
    fn cannot_prestige_at_planets() {
        assert!(!can_prestige(&at_paradigm(Paradigm::Planets)));
    }

    #[test]
    fn can_prestige_at_gasgiant() {
        assert!(can_prestige(&at_paradigm(Paradigm::GasGiant)));
    }

    #[test]
    fn can_prestige_at_star() {
        assert!(can_prestige(&at_paradigm(Paradigm::Star)));
    }

    #[test]
    fn paradigms_reached_counts_inclusive() {
        assert_eq!(paradigms_reached(&at_paradigm(Paradigm::Dust)), 1);
        assert_eq!(paradigms_reached(&at_paradigm(Paradigm::GasGiant)), 5);
        assert_eq!(paradigms_reached(&at_paradigm(Paradigm::Star)), 6);
    }

    #[test]
    fn total_mass_is_zero_for_new_game() {
        assert_eq!(total_mass_run(&Game::new()), 0.0);
    }

    #[test]
    fn total_mass_counts_held_resources() {
        let mut g = Game::new();
        *g.resources.get_mut(&ResourceId::Dust).unwrap() = 1000.0;
        assert_eq!(total_mass_run(&g), 1000.0);
    }

    #[test]
    fn total_mass_counts_buildings() {
        let mut g = at_paradigm(Paradigm::Pebbles);
        *g.resources.get_mut(&ResourceId::Dust).unwrap() = 1000.0;
        // DustCollector base_cost = 10. Buying 3 costs 10 + 11.5 + 13.225,
        // but mass counts count × base_cost = 3 × 10 = 30.
        for _ in 0..3 {
            g.buy_building(crate::components::BuildingId::DustCollector);
        }
        // Resources: 1000 - 34.725 = 965.275, buildings: 30. Total ≈ 995.275
        let expected = 1000.0 - (10.0 + 11.5 + 13.225) + 30.0;
        assert!((total_mass_run(&g) - expected).abs() < 1e-6);
    }

    #[test]
    fn g_gain_at_least_one() {
        let g = at_paradigm(Paradigm::GasGiant);
        assert!(compute_g_gain(&g) >= MIN_G_GAIN);
    }

    #[test]
    fn g_gain_grows_with_mass() {
        let mut small = at_paradigm(Paradigm::GasGiant);
        *small.resources.get_mut(&ResourceId::Dust).unwrap() = 1_000.0;

        let mut big = at_paradigm(Paradigm::GasGiant);
        *big.resources.get_mut(&ResourceId::Dust).unwrap() = 1_000_000.0;

        assert!(compute_g_gain(&big) > compute_g_gain(&small));
    }

    #[test]
    fn prestige_resets_run_and_preserves_g() {
        let mut g = at_paradigm(Paradigm::GasGiant);
        *g.resources.get_mut(&ResourceId::Dust).unwrap() = 500_000.0;
        let gain = prestige(&mut g);

        assert_eq!(g.paradigm, Paradigm::Dust);
        assert_eq!(g.g_total, gain);
        assert_eq!(g.resources[&ResourceId::Dust], 0.0);
        for id in crate::components::BuildingId::ALL {
            assert_eq!(g.building_count(id), 0, "{id:?} should be reset");
        }
    }

    #[test]
    fn prestige_updates_g_multiplier() {
        let mut g = at_paradigm(Paradigm::GasGiant);
        *g.resources.get_mut(&ResourceId::Dust).unwrap() = 500_000.0;
        let gain = prestige(&mut g);

        let expected = 1.0 + G_MULTIPLIER_PER_POINT * gain as f64;
        assert!((g.g_multiplier - expected).abs() < 1e-9);
    }

    #[test]
    fn prestige_twice_stacks_g() {
        let mut g = at_paradigm(Paradigm::GasGiant);
        *g.resources.get_mut(&ResourceId::Dust).unwrap() = 500_000.0;
        let g1 = prestige(&mut g);

        g.paradigm = Paradigm::GasGiant;
        *g.resources.get_mut(&ResourceId::Dust).unwrap() = 500_000.0;
        let g2 = prestige(&mut g);

        assert_eq!(g.g_total, g1 + g2);
    }

    #[test]
    fn prestige_pushes_collapse_messages() {
        let mut g = at_paradigm(Paradigm::GasGiant);
        *g.resources.get_mut(&ResourceId::Dust).unwrap() = 500_000.0;
        prestige(&mut g);

        assert!(g.log.iter().any(|l| l.contains("Collapse")));
        assert!(g.log.iter().any(|l| l.contains("I fall again")));
    }

    #[test]
    #[should_panic(expected = "prestige requires paradigm")]
    fn prestige_at_planets_panics() {
        let mut g = at_paradigm(Paradigm::Planets);
        prestige(&mut g);
    }

    #[test]
    fn prestige_preserves_endings_seen() {
        use crate::components::Ending;
        let mut g = at_paradigm(Paradigm::GasGiant);
        g.endings_seen.push(Ending::NeutronStar);
        *g.resources.get_mut(&ResourceId::Dust).unwrap() = 500_000.0;
        prestige(&mut g);
        assert_eq!(g.endings_seen.len(), 1);
    }
}