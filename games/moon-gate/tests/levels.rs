//! Every level is solvable in par, and each chapter's new piece matters.
//!
//! Adding a level that cannot be solved, or whose par is wrong, fails here
//! (and so fails `make pre-push`).

use moon_gate::levels::{LEVELS, LevelDef};
use moon_gate::{Game, Level, solve};
use std::collections::HashSet;

fn parse(def: &LevelDef) -> Level {
    def.parse()
        .unwrap_or_else(|e| panic!("level {} does not parse: {e}", def.id))
}

#[test]
fn every_level_is_solvable_in_par() {
    for def in LEVELS {
        let level = parse(def);
        let moves = solve(&level).unwrap_or_else(|| panic!("level {} has no solution", def.id));
        assert_eq!(
            moves.len(),
            def.par,
            "level {}: the shortest solution has {} moves, par says {}",
            def.id,
            moves.len(),
            def.par
        );

        // The solution replays through `Game` and earns the star.
        let mut game = Game::new(level);
        for dir in moves {
            game.play(dir).expect("solver moves are legal");
        }
        assert!(game.within_par(def.par), "level {}", def.id);
    }
}

#[test]
fn each_chapter_needs_its_new_piece() {
    // The piece a chapter adds; without it the level has no solution in par.
    let piece = |chapter| match chapter {
        2 => Some('o'),
        3 => Some('1'),
        4 => Some('='),
        _ => None,
    };
    for def in LEVELS {
        let Some(ch) = piece(def.chapter()) else {
            continue;
        };
        // Pools go in pairs, so replacing every `1` keeps the level valid.
        let text = def.text.replace(ch, ".");
        let Ok(level) = Level::parse(&text) else {
            continue;
        };
        if let Some(moves) = solve(&level) {
            assert!(
                moves.len() > def.par,
                "level {} is solvable in par without {ch:?}",
                def.id
            );
        }
    }
}

#[test]
fn ids_are_unique_and_in_order() {
    let mut seen = HashSet::new();
    for def in LEVELS {
        assert!(seen.insert(def.id), "duplicate level id {}", def.id);
        assert!(def.chapter() >= 1, "bad id {}", def.id);
    }
    let ids: Vec<_> = LEVELS.iter().map(|d| d.id).collect();
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    assert_eq!(ids, sorted);
}
