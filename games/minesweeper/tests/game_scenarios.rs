//! Integration scenarios for Minesweeper.

use ember_core::rng::Rng;
use minesweeper::components::{Board, BoardState};
use minesweeper::systems;

/// Build a board with mines placed at explicit positions, adjacency
/// computed, state = Playing. Test helper.
fn board_with(w: usize, h: usize, mines: &[(usize, usize)]) -> Board {
    let mut b = Board::new(w, h, mines.len());
    for &(c, r) in mines {
        b.cells.get_mut(c, r).unwrap().is_mine = true;
    }
    for row in 0..h {
        for col in 0..w {
            if b.cells.get(col, row).unwrap().is_mine {
                continue;
            }
            let count = b
                .cells
                .neighbors_8(col, row)
                .iter()
                .filter(|(_, _, c)| c.is_mine)
                .count() as u8;
            b.cells.get_mut(col, row).unwrap().adjacent = count;
        }
    }
    b.mines_placed = true;
    b.state = BoardState::Playing;
    b
}

#[test]
fn test_full_win_scenario() {
    let mut b = board_with(3, 3, &[(0, 0)]);
    for r in 0..3 {
        for c in 0..3 {
            if b.cells.get(c, r).unwrap().is_mine {
                continue;
            }
            systems::reveal(&mut b, c, r);
        }
    }
    assert_eq!(b.state, BoardState::Won);
    assert_eq!(b.revealed_count, 8);
}

#[test]
fn test_full_lose_scenario() {
    let mut b = board_with(3, 3, &[(0, 0)]);
    systems::reveal(&mut b, 0, 0);
    assert_eq!(b.state, BoardState::Lost);
}

#[test]
fn test_flood_fill_on_large_empty_board() {
    let mut b = board_with(5, 5, &[(0, 0)]);
    systems::reveal(&mut b, 4, 4);
    assert_eq!(b.state, BoardState::Won);
    assert_eq!(b.revealed_count, 24);
}

#[test]
fn test_flag_then_chord_sequence() {
    let mut b = board_with(3, 3, &[(0, 0)]);
    systems::reveal(&mut b, 1, 1);
    assert!(b.cells.get(1, 1).unwrap().is_revealed());

    systems::toggle_flag(&mut b, 0, 0);
    assert!(b.cells.get(0, 0).unwrap().is_flagged());

    systems::chord(&mut b, 1, 1);
    assert_eq!(b.state, BoardState::Won);
}

#[test]
fn test_first_click_never_hits_mine() {
    for seed in 0..200u32 {
        let mut b = Board::new(9, 9, 10);
        let mut rng = Rng::new(seed.wrapping_mul(2654435761).wrapping_add(1));
        let (c, r) = (4, 4);
        systems::place_mines(&mut b, c, r, &mut rng);
        assert!(
            !b.cells.get(c, r).unwrap().is_mine,
            "seed {seed} placed a mine on the first-click cell"
        );
        for (nc, nr) in b.cells.neighbor_coords_8(c, r) {
            assert!(
                !b.cells.get(nc, nr).unwrap().is_mine,
                "seed {seed} placed a mine in the safe zone at ({nc},{nr})"
            );
        }
    }
}

// ============================================================================
// Expert (30x16) — flood fill, densité réelle, performance
// ============================================================================

#[test]
fn test_flood_fill_on_expert_30x16() {
    // Expert : 30x16, 99 mines. On place UNE seule mine dans un coin
    // pour garantir qu'un flood fill depuis le coin opposé traverse
    // toute la grille sans être bloqué.
    let mut b = Board::new(30, 16, 1);
    b.cells.get_mut(0, 0).unwrap().is_mine = true;
    // Calcule les adjacent counts pour toutes les cellules non-mine.
    for row in 0..16 {
        for col in 0..30 {
            if b.cells.get(col, row).unwrap().is_mine {
                continue;
            }
            let count = b
                .cells
                .neighbors_8(col, row)
                .iter()
                .filter(|(_, _, c)| c.is_mine)
                .count() as u8;
            b.cells.get_mut(col, row).unwrap().adjacent = count;
        }
    }
    b.mines_placed = true;
    b.state = BoardState::Playing;

    // Flood depuis le coin opposé.
    systems::reveal(&mut b, 29, 15);

    // 30*16 - 1 = 479 cellules sûres.
    assert_eq!(b.revealed_count, 479);
    assert_eq!(b.state, BoardState::Won);
    // La mine reste cachée.
    assert!(!b.cells.get(0, 0).unwrap().is_revealed());
}

#[test]
fn test_flood_fill_on_expert_with_real_density() {
    // Expert réaliste : 99 mines. On vérifie que le flood fill
    // démarre bien (premier clic safe) et révèle une zone cohérente.
    let mut b = Board::new(30, 16, 99);
    let mut rng = Rng::new(42);
    // Premier clic au centre (14, 7) — safe zone garantie.
    systems::place_mines(&mut b, 14, 7, &mut rng);
    systems::reveal(&mut b, 14, 7);

    // Le flood fill a pu révéler plus ou moins selon la densité,
    // mais au moins la cellule cliquée doit être révélée.
    assert!(b.cells.get(14, 7).unwrap().is_revealed());
    // La partie n'est ni gagnée ni perdue.
    assert!(matches!(b.state, BoardState::Playing | BoardState::Won));
}

#[test]
fn test_flood_fill_performance_on_expert() {
    // Vérifie que le flood fill complet sur 30x16 ne prend pas un temps
    // déraisonnable (< 100 ms, large marge).
    let mut b = Board::new(30, 16, 1);
    b.cells.get_mut(0, 0).unwrap().is_mine = true;
    for row in 0..16 {
        for col in 0..30 {
            if b.cells.get(col, row).unwrap().is_mine {
                continue;
            }
            let count = b
                .cells
                .neighbors_8(col, row)
                .iter()
                .filter(|(_, _, c)| c.is_mine)
                .count() as u8;
            b.cells.get_mut(col, row).unwrap().adjacent = count;
        }
    }
    b.mines_placed = true;
    b.state = BoardState::Playing;

    let start = std::time::Instant::now();
    systems::reveal(&mut b, 29, 15);
    let elapsed = start.elapsed();

    assert!(elapsed.as_millis() < 100, "flood fill took {elapsed:?}");
    assert_eq!(b.revealed_count, 479);
}
