//! Integration scenarios for FreeCell.
//!
//! These exercise the full `Game` through the public API (try_drop,
//! undo, redo), simulating multi-step moves and end-of-game states.
//! They complement the unit tests inside `systems.rs`.

use std::collections::HashMap;
use std::path::PathBuf;

use ember_stdlib::persistence::Persistence;

use freecell::GameState;
use freecell::components::{Card, Rank, Suit, Zone};
use freecell::config::load_config;
use freecell::systems::Game;

/// Build a `Game` with a temp-file best-times handle.
fn test_game(name: &str) -> (Game, PathBuf) {
    let path = std::env::temp_dir().join(format!("freecell_test_{name}.ron"));
    let _ = std::fs::remove_file(&path);
    let handle: Persistence<HashMap<String, f32>> = Persistence::new(&path);
    let g = Game::with_best_handle(handle);
    (g, path)
}

// ============================================================================
// Full solve: build a near-complete board and finish it
// ============================================================================

#[test]
fn test_full_easy_solve_known_sequence() {
    let ctx = load_config();
    let (mut g, path) = test_game("full_solve");

    g.start_game(1, &ctx);

    // Suits in foundation order: Spade(0), Heart(1), Diamond(2), Club(3).
    // For each suit, build the column as [K, Q, ..., A] (top to bottom).
    let all_suits = [Suit::Spade, Suit::Heart, Suit::Diamond, Suit::Club];
    for (col_idx, suit) in all_suits.iter().enumerate() {
        let mut column = Vec::new();
        for rank in (1..=13).rev() {
            column.push(Card::new(*suit, Rank(rank)));
        }
        g.columns[col_idx] = column;
    }
    for col in 4..8 {
        g.columns[col].clear();
    }
    for f in 0..4 {
        g.foundations[f].clear();
    }
    for c in 0..4 {
        g.free_cells[c] = None;
    }

    // Play: for each suit, move the A (bottom), then 2, ... up to K.
    let total_moves = 13 * 4;
    for (col, suit) in all_suits.iter().enumerate() {
        let foundation_idx = suit.index();
        for _ in 0..13 {
            let len = g.columns[col].len();
            let card = g.columns[col][len - 1];
            let start_idx = len - 1;
            let ok = g.try_drop(
                Zone::Column(col),
                Zone::Foundation(foundation_idx),
                &[card],
                start_idx,
            );
            assert!(
                ok,
                "drop of {:?} on foundation {} failed",
                card, foundation_idx
            );
        }
    }

    assert_eq!(g.moves, total_moves);
    assert_eq!(g.state, GameState::Win);
    assert!(g.foundations.iter().all(|f| f.len() == 13));
    let _ = std::fs::remove_file(path);
}

// ============================================================================
// Super-move: move a multi-card sequence
// ============================================================================

#[test]
fn test_super_move_sequence_to_empty_column() {
    let ctx = load_config();
    let (mut g, path) = test_game("super_move");
    g.start_game(1, &ctx);

    // Deterministic board: column 0 = [K♠, Q♥, J♠], other columns empty.
    for col in 0..8 {
        g.columns[col].clear();
    }
    g.columns[0].push(Card::new(Suit::Spade, Rank(13))); // K♠
    g.columns[0].push(Card::new(Suit::Heart, Rank(12))); // Q♥
    g.columns[0].push(Card::new(Suit::Spade, Rank(11))); // J♠
    for f in 0..4 {
        g.foundations[f].clear();
    }
    for c in 0..4 {
        g.free_cells[c] = None;
    }

    let cards = g.columns[0].clone();
    assert_eq!(cards.len(), 3);
    let ok = g.try_drop(Zone::Column(0), Zone::Column(1), &cards, 0);
    assert!(ok);
    assert!(g.columns[0].is_empty());
    assert_eq!(g.columns[1].len(), 3);
    assert_eq!(g.columns[1][0], Card::new(Suit::Spade, Rank(13)));
    assert_eq!(g.columns[1][1], Card::new(Suit::Heart, Rank(12)));
    assert_eq!(g.columns[1][2], Card::new(Suit::Spade, Rank(11)));
    let _ = std::fs::remove_file(path);
}

// ============================================================================
// Best time persistence
// ============================================================================

#[test]
fn test_best_time_persisted_on_win() {
    let ctx = load_config();
    let (mut g, path) = test_game("best_time");

    g.start_game(42, &ctx);
    g.elapsed = 12.5;

    // Pre-fill foundations 1..4 at 13 cards; foundation 0 stays at 12.
    // The last card (K♠) will complete foundation 0.
    g.columns = Default::default();
    g.free_cells = Default::default();
    for f in 0..4 {
        g.foundations[f].clear();
        let suit = Suit::ALL[f];
        let max_rank = if f == 0 { 12 } else { 13 };
        for r in 1..=max_rank {
            g.foundations[f].push(Card::new(suit, Rank(r)));
        }
    }
    g.columns[0].push(Card::new(Suit::Spade, Rank(13)));

    let ok = g.try_drop(
        Zone::Column(0),
        Zone::Foundation(Suit::Spade.index()),
        &[Card::new(Suit::Spade, Rank(13))],
        0,
    );
    assert!(ok);
    assert_eq!(g.state, GameState::Win);
    assert_eq!(g.best_times.get("42"), Some(&g.elapsed));

    // Persisted to disk.
    let loaded: HashMap<String, f32> =
        ember_core::io::load_from_file(&path).expect("best_times.ron should exist");
    assert_eq!(loaded.get("42"), Some(&g.elapsed));

    let _ = std::fs::remove_file(path);
}

// ============================================================================
// Undo / redo integration
// ============================================================================

#[test]
fn test_undo_redo_roundtrip() {
    let ctx = load_config();
    let (mut g, path) = test_game("undo_redo");
    g.start_game(1, &ctx);

    for col in 0..8 {
        g.columns[col].clear();
    }
    g.columns[0].push(Card::new(Suit::Spade, Rank(8)));
    g.columns[1].push(Card::new(Suit::Heart, Rank(7)));

    let before_move = g.columns.clone();

    let ok = g.try_drop(
        Zone::Column(1),
        Zone::Column(0),
        &[Card::new(Suit::Heart, Rank(7))],
        0,
    );
    assert!(ok);
    assert_eq!(g.moves, 1);

    assert!(g.undo());
    assert_eq!(g.columns, before_move);
    assert_eq!(g.moves, 0);

    assert!(g.redo());
    assert_eq!(g.moves, 1);

    let _ = std::fs::remove_file(path);
}

// ============================================================================
// Deal is a permutation of the 52 cards
// ============================================================================

#[test]
fn test_dealt_board_contains_all_52_unique_cards() {
    let ctx = load_config();
    let (mut g, path) = test_game("deal_unique");
    g.start_game(7, &ctx);

    let mut seen: std::collections::HashSet<Card> = std::collections::HashSet::new();
    for col in &g.columns {
        for c in col {
            assert!(seen.insert(*c), "duplicate card in dealt board: {:?}", c);
        }
    }
    assert_eq!(seen.len(), 52);

    let _ = std::fs::remove_file(path);
}

// ============================================================================
// Win detection after a full solve
// ============================================================================

#[test]
fn test_win_after_all_foundations_filled() {
    let ctx = load_config();
    let (mut g, path) = test_game("win_all");

    g.start_game(1, &ctx);
    g.columns = Default::default();
    g.free_cells = Default::default();

    for f in 0..4 {
        g.foundations[f].clear();
        let suit = Suit::ALL[f];
        let max_rank = if f == 0 { 12 } else { 13 };
        for r in 1..=max_rank {
            g.foundations[f].push(Card::new(suit, Rank(r)));
        }
    }
    g.columns[0].push(Card::new(Suit::Spade, Rank(13)));

    assert!(g.try_drop(
        Zone::Column(0),
        Zone::Foundation(Suit::Spade.index()),
        &[Card::new(Suit::Spade, Rank(13))],
        0,
    ));
    assert_eq!(g.state, GameState::Win);

    let _ = std::fs::remove_file(path);
}

// ============================================================================
// Hint
// ============================================================================

#[test]
fn test_hint_is_only_an_indicator() {
    let ctx = load_config();
    let (mut g, path) = test_game("hint_indicator");
    g.start_game(1, &ctx);

    for col in 0..8 {
        g.columns[col].clear();
    }
    let ace = Card::new(Suit::Heart, Rank::ACE);
    g.columns[0].push(ace);

    let columns_before = g.columns.clone();
    let foundations_before = g.foundations.clone();
    let cells_before = g.free_cells;
    let moves_before = g.moves;
    let timer_before = g.timer_running;

    let hint = g.find_hint().expect("expected a hint");

    assert_eq!(hint.from, Zone::Column(0));
    assert_eq!(hint.to, Zone::Foundation(Suit::Heart.index()));
    assert_eq!(hint.card, ace);
    assert_eq!(g.columns, columns_before);
    assert_eq!(g.foundations, foundations_before);
    assert_eq!(g.free_cells, cells_before);
    assert_eq!(g.moves, moves_before);
    assert_eq!(g.timer_running, timer_before);

    let _ = std::fs::remove_file(path);
}

#[test]
fn test_hint_returns_none_when_no_move_exists() {
    let ctx = load_config();
    let (mut g, path) = test_game("hint_none");
    g.start_game(1, &ctx);

    g.columns = Default::default();
    g.free_cells = Default::default();
    for f in 0..4 {
        g.foundations[f].clear();
        let suit = Suit::ALL[f];
        for rank in 1..=13 {
            g.foundations[f].push(Card::new(suit, Rank(rank)));
        }
    }

    assert_eq!(g.find_hint(), None);

    let _ = std::fs::remove_file(path);
}