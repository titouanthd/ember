//! Tests d'intégration cross-module (design §10.3).
//!
//! Ne pilotent pas `Game::tick` (qui dépend de `screen_width()`
//! macroquad, indisponible en headless). À la place, ils valident les
//! invariants au niveau des composants purs : chargement RON, scoring,
//! progression, cohérence poème, déterminisme.

use std::collections::HashSet;

use jade_garden::components::{Jade, GRID_H, GRID_W};
use jade_garden::grid::Grid;
use jade_garden::level::{self, Objective};
use jade_garden::poem;
use jade_garden::progress::Progress;
use jade_garden::scoring::{
    base_score_for_tile_count, cascade_multiplier, compute_stars,
};

// ─────────────────────────────────────────────────────────────
// 1. Chargement des assets
// ─────────────────────────────────────────────────────────────

#[test]
fn levels_ron_loads_12_entries() {
    let levels = level::load_levels().expect("levels.ron should parse");
    assert_eq!(levels.len(), 12);
}

#[test]
fn level_configs_have_valid_objectives() {
    for lvl in level::load_levels().unwrap() {
        assert!(lvl.moves > 0, "{} has no moves", lvl.id);
        assert!(lvl.star_target > 0, "{} has no star target", lvl.id);
        match lvl.objective {
            Objective::Score(n) => assert!(n > 0, "{} has zero score target", lvl.id),
            Objective::ClearJade(_, n) => {
                assert!(n > 0, "{} has zero clear count", lvl.id)
            }
            Objective::FillPoem(n) => {
                assert!(n > 0, "{} has zero poem target", lvl.id);
                assert!(
                    n <= poem::TOTAL_CHARS,
                    "{} wants more poem chars than exist",
                    lvl.id
                );
            }
        }
    }
}

#[test]
fn all_level_ids_are_unique() {
    let levels = level::load_levels().unwrap();
    let unique: HashSet<&str> = levels.iter().map(|l| l.id.as_str()).collect();
    assert_eq!(unique.len(), levels.len(), "duplicate level IDs");
}

#[test]
fn every_chapter_has_exactly_three_levels() {
    let levels = level::load_levels().unwrap();
    let mut counts = [0u32; 4];
    for l in &levels {
        counts[l.chapter as usize] += 1;
    }
    for (i, c) in counts.iter().enumerate() {
        assert_eq!(*c, 3, "chapter {i} has {c} levels");
    }
}

// ─────────────────────────────────────────────────────────────
// 2. Cohérence objectif ↔ poème
// ─────────────────────────────────────────────────────────────

#[test]
fn poem_reveal_sums_to_exactly_twenty() {
    let levels = level::load_levels().unwrap();
    let total: u32 = levels.iter().map(|l| l.poem_reveal as u32).sum();
    assert_eq!(total, poem::TOTAL_CHARS as u32);
}

#[test]
fn levels_with_fill_poem_have_target_within_level_reveal_budget() {
    for lvl in level::load_levels().unwrap() {
        if let Objective::FillPoem(target) = lvl.objective {
            assert!(
                target as u32 <= lvl.moves,
                "{} wants {} poem matches but has only {} moves",
                lvl.id,
                target,
                lvl.moves
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────
// 3. Scoring ↔ victoire
// ─────────────────────────────────────────────────────────────

#[test]
fn scoring_increases_with_tile_count_and_cascade() {
    for n in 3..8 {
        assert!(
            base_score_for_tile_count(n + 1) > base_score_for_tile_count(n),
            "base score not increasing at n={n}"
        );
    }
    for c in 0..8 {
        assert!(
            cascade_multiplier(c + 1) > cascade_multiplier(c)
                || cascade_multiplier(c) >= 5.0,
            "cascade multiplier not increasing at c={c}"
        );
    }
}

#[test]
fn twenty_moves_can_reach_1500_without_cascades() {
    // À 60 pts par match-3, 20 coups = 1200 minimum. Quelques
    // cascades (naturellement présentes) suffisent pour 1500.
    let base = base_score_for_tile_count(3);
    let min_score = 20 * base;
    assert!(
        min_score >= 1200,
        "20 moves should reach at least 1200, got {min_score}"
    );
}

#[test]
fn stars_computed_correctly_for_realistic_scores() {
    assert_eq!(compute_stars(1000, 1000), 1);
    assert_eq!(compute_stars(1500, 1000), 2);
    assert_eq!(compute_stars(2000, 1000), 3);
}

// ─────────────────────────────────────────────────────────────
// 4. Progression ↔ déverrouillage
// ─────────────────────────────────────────────────────────────

#[test]
fn sequential_completion_unlocks_all_levels() {
    let mut p = Progress::default();
    let levels = level::load_levels().unwrap();
    for (i, lvl) in levels.iter().enumerate() {
        let unlocked = i == 0 || p.level_completed(&levels[i - 1].id);
        assert!(unlocked, "level {i} ({}) should be unlocked", lvl.id);
        p.record_win(&lvl.id, lvl.star_target, 1, lvl.poem_reveal);
    }
    assert_eq!(p.total_stars(), 12, "should have all 12 levels completed");
}

#[test]
fn playing_all_levels_reveals_full_poem() {
    let mut p = Progress::default();
    for lvl in level::load_levels().unwrap() {
        p.record_win(&lvl.id, lvl.star_target, 1, lvl.poem_reveal);
    }
    assert_eq!(p.poem_fragments, poem::TOTAL_CHARS);
}

#[test]
fn replay_does_not_reveal_extra_poem_fragments() {
    let mut p = Progress::default();
    let lvl = &level::load_levels().unwrap()[0];
    p.record_win(&lvl.id, 1000, 1, lvl.poem_reveal);
    let after_first = p.poem_fragments;
    for _ in 0..5 {
        p.record_win(&lvl.id, 5000, 3, lvl.poem_reveal);
    }
    assert_eq!(p.poem_fragments, after_first);
}

#[test]
fn best_score_is_kept_not_overwritten() {
    let mut p = Progress::default();
    p.record_win("l01", 5000, 3, 2);
    p.record_win("l01", 1000, 1, 2);
    assert_eq!(p.level_best_score("l01"), 5000);
    assert_eq!(p.level_stars("l01"), 3);
}

// ─────────────────────────────────────────────────────────────
// 5. Déterminisme
// ─────────────────────────────────────────────────────────────

#[test]
fn grid_is_deterministic_for_same_seed() {
    for seed in [1, 42, 12345, u32::MAX] {
        let a = Grid::new(seed);
        let b = Grid::new(seed);
        assert_eq!(a, b, "seed {seed} should produce identical grids");
    }
}

#[test]
fn grid_is_valid_for_every_level_seed() {
    for lvl in level::load_levels().unwrap() {
        let g = Grid::new(lvl.seed);
        assert!(
            g.find_matches().is_empty(),
            "{} (seed {}) has an initial match",
            lvl.id,
            lvl.seed
        );
        assert!(
            g.has_any_valid_move(),
            "{} (seed {}) has no valid move",
            lvl.id,
            lvl.seed
        );
    }
}

// ─────────────────────────────────────────────────────────────
// 6. Persistance
// ─────────────────────────────────────────────────────────────

#[test]
fn progress_survives_ron_roundtrip() {
    let mut p = Progress::default();
    for lvl in &level::load_levels().unwrap()[..3] {
        p.record_win(&lvl.id, 1000 + lvl.star_target, 2, lvl.poem_reveal);
    }
    let text = ron::ser::to_string(&p).unwrap();
    let back: Progress = ron::from_str(&text).unwrap();
    assert_eq!(back.poem_fragments, p.poem_fragments);
    assert_eq!(back.total_stars(), p.total_stars());
    for lvl in &level::load_levels().unwrap()[..3] {
        assert_eq!(back.level_best_score(&lvl.id), p.level_best_score(&lvl.id));
    }
}

#[test]
fn progress_total_stars_matches_sum_of_level_stars() {
    let mut p = Progress::default();
    let levels = level::load_levels().unwrap();
    let mut expected = 0u32;
    for (i, lvl) in levels.iter().enumerate() {
        let stars = ((i % 3) + 1) as u8;
        p.record_win(&lvl.id, 1000, stars, 0);
        expected += stars as u32;
    }
    assert_eq!(p.total_stars(), expected);
}

// ─────────────────────────────────────────────────────────────
// 7. Poème — séquence ordonnée
// ─────────────────────────────────────────────────────────────

#[test]
fn poem_fragments_revealed_in_order() {
    assert_eq!(poem::char_at(0), Some('春'));
    assert_eq!(poem::char_at(19), Some('少'));
    for i in 0..poem::TOTAL_CHARS {
        assert!(
            poem::char_at(i).is_some(),
            "index {i} should resolve to a character"
        );
        assert!(
            poem::pinyin_at(i).is_some(),
            "index {i} should resolve to a pinyin"
        );
    }
}

#[test]
fn poem_has_exactly_four_lines_of_five() {
    assert_eq!(poem::POEM_LINES.len(), 4);
    for line in poem::POEM_LINES {
        assert_eq!(line.chars().count(), 5);
    }
}

// ─────────────────────────────────────────────────────────────
// 8. Jade types — cohérence
// ─────────────────────────────────────────────────────────────

#[test]
fn every_level_objective_references_a_valid_jade() {
    for lvl in level::load_levels().unwrap() {
        if let Objective::ClearJade(jade, _) = lvl.objective {
            assert!(
                Jade::ALL.contains(&jade),
                "{} references unknown jade {jade:?}",
                lvl.id
            );
        }
    }
}

#[test]
fn all_clear_jade_objectives_use_distinct_types() {
    let levels = level::load_levels().unwrap();
    let mut seen: HashSet<Jade> = HashSet::new();
    for lvl in &levels {
        if let Objective::ClearJade(jade, _) = lvl.objective {
            seen.insert(jade);
        }
    }
    assert!(
        seen.len() >= 3,
        "expected at least 3 distinct jades across ClearJade levels, got {}",
        seen.len()
    );
}

// ─────────────────────────────────────────────────────────────
// 9. Cohérence grille ↔ composants
// ─────────────────────────────────────────────────────────────

#[test]
fn grid_dimensions_match_constants() {
    assert_eq!(GRID_W, 8);
    assert_eq!(GRID_H, 8);
    let g = Grid::new(1);
    for row in 0..GRID_H {
        for col in 0..GRID_W {
            assert!(g.get(row, col).is_some());
        }
    }
    assert!(g.get(GRID_H, 0).is_none());
    assert!(g.get(0, GRID_W).is_none());
}