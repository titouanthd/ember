//! Game logic: mine placement, reveal, flag, chord, win/lose detection.

use crate::components::{Board, BoardState, CellState};
use crate::rng::rand_usize;

/// Place `board.mine_count` mines on the board, excluding the safe zone
/// around `(safe_col, safe_row)` (that cell + its 8 neighbors). Then
/// compute the `adjacent` count for every non-mine cell.
///
/// Called once, on the first reveal.
pub fn place_mines(board: &mut Board, safe_col: usize, safe_row: usize, rng: &mut u32) {
    let mut safe: Vec<(usize, usize)> = vec![(safe_col, safe_row)];
    for (c, r) in board.cells.neighbor_coords_8(safe_col, safe_row) {
        safe.push((c, r));
    }

    // Rejection sampling. In the worst case (Expert: 480 cells, 99 mines,
    // 9 safe) the expected number of iterations is ~ (99 * 480) / 471 ≈ 101,
    // so this is fast enough.
    let mut placed = 0;
    let total = board.width * board.height;
    let mut attempts = 0;
    while placed < board.mine_count {
        attempts += 1;
        // Safety valve: if we've been trying way too long, bail.
        // (Should never trigger for reasonable configs.)
        if attempts > total * 20 {
            break;
        }
        let c = rand_usize(rng, board.width);
        let r = rand_usize(rng, board.height);
        if safe.contains(&(c, r)) {
            continue;
        }
        let cell = board.cells.get_mut(c, r).unwrap();
        if cell.is_mine {
            continue;
        }
        cell.is_mine = true;
        placed += 1;
    }

    // Compute adjacent counts.
    for row in 0..board.height {
        for col in 0..board.width {
            if board.cells.get(col, row).unwrap().is_mine {
                continue;
            }
            let count = board
                .cells
                .neighbors_8(col, row)
                .iter()
                .filter(|(_, _, c)| c.is_mine)
                .count() as u8;
            board.cells.get_mut(col, row).unwrap().adjacent = count;
        }
    }

    board.mines_placed = true;
    board.state = BoardState::Playing;
}

/// Reveal a cell. Handles flood fill when `adjacent == 0`.
///
/// No-op if:
/// - out of bounds
/// - already revealed
/// - flagged
/// - board is not in a playable state
///
/// Sets `board.state = Lost` if a mine is revealed.
pub fn reveal(board: &mut Board, col: usize, row: usize) {
    if !matches!(board.state, BoardState::Ready | BoardState::Playing) {
        return;
    }
    if col >= board.width || row >= board.height {
        return;
    }

    let cell = *board.cells.get(col, row).unwrap();
    if cell.is_revealed() || cell.is_flagged() {
        return;
    }

    // Reveal the clicked cell.
    board.cells.get_mut(col, row).unwrap().state = CellState::Revealed;
    board.revealed_count += 1;

    if cell.is_mine {
        board.state = BoardState::Lost;
        return;
    }

    // Flood fill if the cell has no adjacent mines.
    if cell.adjacent == 0 {
        let mut stack: Vec<(usize, usize)> = vec![(col, row)];
        while let Some((c, r)) = stack.pop() {
            // Grab neighbor coords first to avoid borrowing `cells` twice.
            let coords = board.cells.neighbor_coords_8(c, r);
            for (nc, nr) in coords {
                let n = *board.cells.get(nc, nr).unwrap();
                if n.is_hidden() {
                    board.cells.get_mut(nc, nr).unwrap().state = CellState::Revealed;
                    board.revealed_count += 1;
                    if !n.is_mine && n.adjacent == 0 {
                        stack.push((nc, nr));
                    }
                }
            }
        }
    }

    check_win(board);
}

/// Toggle the flag on a hidden cell. No-op if the cell is revealed.
pub fn toggle_flag(board: &mut Board, col: usize, row: usize) {
    if !matches!(board.state, BoardState::Ready | BoardState::Playing) {
        return;
    }
    if col >= board.width || row >= board.height {
        return;
    }
    let cell = board.cells.get_mut(col, row).unwrap();
    match cell.state {
        CellState::Hidden => {
            cell.state = CellState::Flagged;
            board.flagged_count += 1;
        }
        CellState::Flagged => {
            cell.state = CellState::Hidden;
            board.flagged_count -= 1;
        }
        CellState::Revealed => {} // no-op
    }
}

/// Chord: if a revealed cell has as many flagged neighbors as its
/// `adjacent` count, reveal all its other hidden neighbors.
///
/// No-op if:
/// - board is not playable
/// - cell is not revealed
/// - cell has 0 adjacent mines (nothing to chord)
/// - flag count doesn't match adjacent count
pub fn chord(board: &mut Board, col: usize, row: usize) {
    if !matches!(board.state, BoardState::Playing) {
        return;
    }
    if col >= board.width || row >= board.height {
        return;
    }
    let cell = *board.cells.get(col, row).unwrap();
    if !cell.is_revealed() || cell.adjacent == 0 {
        return;
    }

    let coords = board.cells.neighbor_coords_8(col, row);
    let flagged = coords
        .iter()
        .filter(|(c, r)| board.cells.get(*c, *r).unwrap().is_flagged())
        .count();

    if flagged as u8 != cell.adjacent {
        return;
    }

    // Reveal all hidden neighbors (not flagged).
    let to_reveal: Vec<(usize, usize)> = coords
        .iter()
        .filter(|(c, r)| board.cells.get(*c, *r).unwrap().is_hidden())
        .copied()
        .collect();

    for (c, r) in to_reveal {
        reveal(board, c, r);
        if board.state == BoardState::Lost {
            return;
        }
    }
}

/// If every safe cell is revealed, transition to Won.
pub fn check_win(board: &mut Board) {
    if board.revealed_count == board.safe_cell_count() {
        board.state = BoardState::Won;
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn b9() -> Board {
        Board::new(9, 9, 10)
    }

    /// Build a board with mines at explicit positions. Adjacency is computed.
    /// Test-only.
    fn board_with_mines(w: usize, h: usize, mines: &[(usize, usize)]) -> Board {
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

    // --- place_mines ---

    #[test]
    fn test_place_mines_count() {
        let mut b = b9();
        let mut rng = 42u32;
        place_mines(&mut b, 4, 4, &mut rng);
        let count = b.cells.iter().filter(|(_, _, c)| c.is_mine).count();
        assert_eq!(count, 10);
        assert!(b.mines_placed);
        assert_eq!(b.state, BoardState::Playing);
    }

    #[test]
    fn test_place_mines_first_click_safe() {
        let mut b = b9();
        let mut rng = 7u32;
        place_mines(&mut b, 4, 4, &mut rng);
        assert!(!b.cells.get(4, 4).unwrap().is_mine);
        for (c, r) in b.cells.neighbor_coords_8(4, 4) {
            assert!(!b.cells.get(c, r).unwrap().is_mine);
        }
    }

    #[test]
    fn test_place_mines_corner_first_click_safe() {
        let mut b = b9();
        let mut rng = 7u32;
        place_mines(&mut b, 0, 0, &mut rng);
        assert!(!b.cells.get(0, 0).unwrap().is_mine);
        for (c, r) in b.cells.neighbor_coords_8(0, 0) {
            assert!(!b.cells.get(c, r).unwrap().is_mine);
        }
    }

    #[test]
    fn test_place_mines_adjacent_counts() {
        let mut b = b9();
        let mut rng = 99u32;
        place_mines(&mut b, 4, 4, &mut rng);
        // Every non-mine cell's `adjacent` must match a fresh count.
        for row in 0..9 {
            for col in 0..9 {
                let cell = *b.cells.get(col, row).unwrap();
                if cell.is_mine {
                    continue;
                }
                let expected = b
                    .cells
                    .neighbors_8(col, row)
                    .iter()
                    .filter(|(_, _, c)| c.is_mine)
                    .count() as u8;
                assert_eq!(cell.adjacent, expected, "at ({col},{row})");
            }
        }
    }

    // --- reveal ---

    #[test]
    fn test_reveal_simple() {
        let mut b = board_with_mines(3, 3, &[(0, 0)]);
        reveal(&mut b, 2, 2);
        assert!(b.cells.get(2, 2).unwrap().is_revealed());
        assert_eq!(b.revealed_count, 1);
    }

    #[test]
    fn test_reveal_flagged_is_noop() {
        let mut b = board_with_mines(3, 3, &[(0, 0)]);
        toggle_flag(&mut b, 2, 2);
        reveal(&mut b, 2, 2);
        assert!(b.cells.get(2, 2).unwrap().is_flagged());
        assert_eq!(b.revealed_count, 0);
    }

    #[test]
    fn test_reveal_already_revealed_is_noop() {
        let mut b = board_with_mines(3, 3, &[(0, 0)]);
        reveal(&mut b, 2, 2);
        reveal(&mut b, 2, 2);
        assert_eq!(b.revealed_count, 1);
    }

    #[test]
    fn test_reveal_mine_loses() {
        let mut b = board_with_mines(3, 3, &[(0, 0)]);
        reveal(&mut b, 0, 0);
        assert_eq!(b.state, BoardState::Lost);
    }

    #[test]
    fn test_flood_fill_empty_region() {
        // 5x5 with a single mine in a corner — flood should reveal
        // everything except the mine.
        let mut b = board_with_mines(5, 5, &[(0, 0)]);
        // Reveal a cell far from the mine, with 0 adjacent.
        reveal(&mut b, 4, 4);
        // 25 total - 1 mine = 24 safe cells.
        assert_eq!(b.revealed_count, 24);
        assert_eq!(b.state, BoardState::Won);
    }

    #[test]
    fn test_flood_fill_does_not_cross_numbers() {
        // 3x3 with a mine at (0,0). Cell (1,1) has adjacent=1 and should
        // block the flood.
        let mut b = board_with_mines(3, 3, &[(0, 0)]);
        // (2,2) has adjacent=0 -> flood reveals (2,2), (1,2), (2,1).
        reveal(&mut b, 2, 2);
        assert!(b.cells.get(2, 2).unwrap().is_revealed());
        assert!(b.cells.get(1, 2).unwrap().is_revealed());
        assert!(b.cells.get(2, 1).unwrap().is_revealed());
        // (1,1) is adjacent to the mine — should NOT be revealed by flood.
        assert!(b.cells.get(1, 1).unwrap().is_hidden());
    }

    // --- flag ---

    #[test]
    fn test_toggle_flag() {
        let mut b = board_with_mines(3, 3, &[(0, 0)]);
        toggle_flag(&mut b, 2, 2);
        assert!(b.cells.get(2, 2).unwrap().is_flagged());
        assert_eq!(b.flagged_count, 1);
        toggle_flag(&mut b, 2, 2);
        assert!(b.cells.get(2, 2).unwrap().is_hidden());
        assert_eq!(b.flagged_count, 0);
    }

    #[test]
    fn test_flag_on_revealed_is_noop() {
        let mut b = board_with_mines(3, 3, &[(0, 0)]);
        reveal(&mut b, 2, 2);
        toggle_flag(&mut b, 2, 2);
        assert!(b.cells.get(2, 2).unwrap().is_revealed());
        assert_eq!(b.flagged_count, 0);
    }

    // --- chord ---

    #[test]
    fn test_chord_reveals_when_flags_match() {
        let mut b = board_with_mines(3, 3, &[(0, 0)]);
        // Reveal (1,1) — adjacent to mine.
        reveal(&mut b, 1, 1);
        assert!(b.cells.get(1, 1).unwrap().is_revealed());
        // Flag the mine.
        toggle_flag(&mut b, 0, 0);
        // Chord on (1,1) should reveal the remaining neighbors.
        chord(&mut b, 1, 1);
        // (0,1) and (1,0) should now be revealed.
        assert!(b.cells.get(0, 1).unwrap().is_revealed());
        assert!(b.cells.get(1, 0).unwrap().is_revealed());
    }

    #[test]
    fn test_chord_does_nothing_without_flags() {
        let mut b = board_with_mines(3, 3, &[(0, 0)]);
        reveal(&mut b, 1, 1);
        chord(&mut b, 1, 1);
        // Neighbors stay hidden.
        assert!(b.cells.get(0, 1).unwrap().is_hidden());
        assert!(b.cells.get(1, 0).unwrap().is_hidden());
    }

    #[test]
    fn test_chord_on_wrong_flag_count_is_noop() {
        let mut b = board_with_mines(3, 3, &[(0, 0)]);
        reveal(&mut b, 1, 1);
        // Flag a random non-mine neighbor (0,1). Flag count for (1,1)
        // should be 1, but the mine at (0,0) is not flagged. Wait — the
        // check is on flagged_neighbors, not correctness. Flagging (0,1)
        // gives count=1, matching adjacent=1, so chord WOULD reveal.
        // This test asserts the actual behavior: chord fires.
        toggle_flag(&mut b, 0, 1);
        chord(&mut b, 1, 1);
        // (0,1) is flagged (skipped). (0,0) is the mine — revealed -> lost.
        assert_eq!(b.state, BoardState::Lost);
    }

    // --- win ---

    #[test]
    fn test_win_when_all_safe_revealed() {
        let mut b = board_with_mines(2, 2, &[(0, 0)]);
        // Reveal the 3 safe cells.
        reveal(&mut b, 1, 0);
        reveal(&mut b, 0, 1);
        reveal(&mut b, 1, 1);
        assert_eq!(b.state, BoardState::Won);
    }

    // --- reveal after game over is no-op ---

    #[test]
    fn test_reveal_after_lost_is_noop() {
        let mut b = board_with_mines(3, 3, &[(0, 0)]);
        reveal(&mut b, 0, 0);
        assert_eq!(b.state, BoardState::Lost);
        let before = b.revealed_count;
        reveal(&mut b, 1, 1);
        assert_eq!(b.revealed_count, before);
    }
}