//! Integration scenarios for Memory.
//!
//! These exercise the full `Game` through `reveal` and `tick`, simulating
//! real play sessions. They complement the unit tests inside `systems.rs`.

use std::collections::HashMap;
use std::path::PathBuf;

use ember_stdlib::persistence::Persistence;

use memory::components::CardState;
use memory::config::load_config;
use memory::difficulties::DifficultyData;
use memory::systems::{Game, SelectionPhase};
use memory::GameState;

/// Build a `Game` with a temp-file best-times handle, so integration tests
/// don't touch the project's `best_times.ron`.
///
/// Each test passes a unique `name` so parallel test execution doesn't
/// clobber the shared file.
fn test_game(name: &str) -> (Game, PathBuf) {
    let path = std::env::temp_dir().join(format!("memory_test_best_{name}.ron"));
    let _ = std::fs::remove_file(&path);
    let handle: Persistence<HashMap<String, f32>> = Persistence::new(&path);
    let g = Game::with_best_handle(handle);
    (g, path)
}

/// Set a synthetic difficulty on the game, with a known RNG seed.
fn configure(g: &mut Game, name: &str, cols: usize, rows: usize, pairs: usize, seed: u32) {
    g.difficulties = vec![DifficultyData {
        name: name.to_string(),
        cols,
        rows,
        pairs,
    }];
    g.selected_difficulty = 0;
    g.rng = ember_core::rng::Rng::new(seed);
}

/// Find two hidden cards with the same symbol.
fn find_matching_pair(g: &Game) -> Option<((usize, usize), (usize, usize))> {
    for r1 in 0..g.board.height() {
        for c1 in 0..g.board.width() {
            if !g.board.get(c1, r1).unwrap().is_hidden() {
                continue;
            }
            let s1 = g.board.get(c1, r1).unwrap().symbol;
            for r2 in 0..g.board.height() {
                for c2 in 0..g.board.width() {
                    if (c1, r1) == (c2, r2) {
                        continue;
                    }
                    if !g.board.get(c2, r2).unwrap().is_hidden() {
                        continue;
                    }
                    if g.board.get(c2, r2).unwrap().symbol == s1 {
                        return Some(((c1, r1), (c2, r2)));
                    }
                }
            }
        }
    }
    None
}

/// Find two hidden cards with different symbols.
fn find_mismatched_pair(g: &Game) -> Option<((usize, usize), (usize, usize))> {
    for r1 in 0..g.board.height() {
        for c1 in 0..g.board.width() {
            if !g.board.get(c1, r1).unwrap().is_hidden() {
                continue;
            }
            let s1 = g.board.get(c1, r1).unwrap().symbol;
            for r2 in 0..g.board.height() {
                for c2 in 0..g.board.width() {
                    if (c1, r1) == (c2, r2) {
                        continue;
                    }
                    if !g.board.get(c2, r2).unwrap().is_hidden() {
                        continue;
                    }
                    if g.board.get(c2, r2).unwrap().symbol != s1 {
                        return Some(((c1, r1), (c2, r2)));
                    }
                }
            }
        }
    }
    None
}

// ============================================================================
// Full playthrough
// ============================================================================

#[test]
fn test_full_easy_playthrough() {
    let ctx = load_config();
    let (mut g, path) = test_game("test_full_easy_playthrough");
    configure(&mut g, "Easy", 4, 4, 8, 42);
    g.start_game(&ctx);

    // Match all 8 pairs.
    for _ in 0..8 {
        let ((c1, r1), (c2, r2)) = find_matching_pair(&g).expect("deck must have pairs");
        g.reveal(c1, r1, &ctx);
        g.reveal(c2, r2, &ctx);
    }

    assert_eq!(g.pairs_found, 8);
    assert_eq!(g.state, GameState::Win);
    assert!(!g.timer_running);
    // All cards matched.
    for r in 0..4 {
        for c in 0..4 {
            assert!(g.board.get(c, r).unwrap().is_matched());
        }
    }

    let _ = std::fs::remove_file(path);
}

// ============================================================================
// No-match cycles back
// ============================================================================

#[test]
fn test_no_match_cycles_back_to_hidden() {
    let ctx = load_config();
    let (mut g, path) = test_game("test_no_match_cycles_back_to_hidden");
    configure(&mut g, "Easy", 4, 4, 8, 7);
    g.start_game(&ctx);

    let ((c1, r1), (c2, r2)) = find_mismatched_pair(&g).unwrap();
    g.reveal(c1, r1, &ctx);
    g.reveal(c2, r2, &ctx);

    assert!(matches!(g.selection, SelectionPhase::Resolving { .. }));
    assert!(g.board.get(c1, r1).unwrap().is_flipped());
    assert!(g.board.get(c2, r2).unwrap().is_flipped());

    // Tick past the delay.
    g.tick(ctx.no_match_delay + 0.01);

    assert_eq!(g.selection, SelectionPhase::Idle);
    assert_eq!(g.board.get(c1, r1).unwrap().state, CardState::Hidden);
    assert_eq!(g.board.get(c2, r2).unwrap().state, CardState::Hidden);
    assert_eq!(g.pairs_found, 0);

    let _ = std::fs::remove_file(path);
}

// ============================================================================
// Best time persistence
// ============================================================================

#[test]
fn test_best_time_persisted_on_win() {
    let ctx = load_config();
    let (mut g, path) = test_game("test_best_time_persisted_on_win");
    configure(&mut g, "Easy", 4, 4, 8, 99);
    g.start_game(&ctx);

    // Simulate some elapsed time, then win.
    g.tick(12.5);
    for _ in 0..8 {
        let ((c1, r1), (c2, r2)) = find_matching_pair(&g).unwrap();
        g.reveal(c1, r1, &ctx);
        g.reveal(c2, r2, &ctx);
    }

    assert_eq!(g.state, GameState::Win);
    // Best time should have been recorded.
    assert_eq!(g.best_times.get("Easy"), Some(&g.elapsed));
    // And persisted to disk.
    let loaded: HashMap<String, f32> =
        ember_core::io::load_from_file(&path).expect("best_times.ron should exist");
    assert_eq!(loaded.get("Easy"), Some(&g.elapsed));

    let _ = std::fs::remove_file(path);
}

// ============================================================================
// Deck sizes for the other difficulties
// ============================================================================

#[test]
fn test_medium_and_hard_deck_sizes() {
    let ctx = load_config();

    for (name, cols, rows, pairs) in [("Medium", 6, 6, 18), ("Hard", 8, 8, 32)] {
        let (mut g, path) = test_game(&format!("test_medium_and_hard_deck_sizes_{name}"));
        configure(&mut g, name, cols, rows, pairs, 1234);
        g.start_game(&ctx);

        assert_eq!(g.board.width(), cols, "{name}: wrong cols");
        assert_eq!(g.board.height(), rows, "{name}: wrong rows");
        assert_eq!(g.total_pairs, pairs, "{name}: wrong total_pairs");
        assert_eq!(g.board.len(), cols * rows, "{name}: wrong card count");

        // All cards start hidden.
        for r in 0..rows {
            for c in 0..cols {
                assert!(
                    g.board.get(c, r).unwrap().is_hidden(),
                    "{name}: card ({c},{r}) not hidden at start"
                );
            }
        }

        let _ = std::fs::remove_file(path);
    }
}