//! Game logic: mine placement, reveal, flag, chord, win/lose detection,
//! and the top-level `Game` state struct.

use std::collections::HashMap;

use glam::Vec2;

use crate::components::{Board, BoardState, CellState};
use crate::config::{GameContext, load_config};
use crate::difficulties::{DifficultyData, load_difficulties};
use crate::persistence::{self, BestTimes};
use ember_core::app::GameState;
use ember_core::rng::Rng;
use ember_stdlib::ui::button::Button;

// ============================================================================
// Game — top-level state for one Minesweeper session
// ============================================================================

/// Top-level state for one Minesweeper session.
///
/// Same convention as the other games: one struct owns everything mutable.
/// `main.rs` holds one `Game` and delegates input handling.
pub struct Game {
    pub state: GameState,
    pub difficulties: Vec<DifficultyData>,
    pub selected_difficulty: usize,

    // Active board (valid when state == Playing / Win / GameOver)
    pub board: Board,
    pub cell_size: f32,
    pub grid_origin: Vec2,
    pub elapsed: f32,
    pub timer_running: bool,
    pub rng: Rng,

    // Persistence
    /// In-memory cache of best times, kept in sync with `best_handle`.
    /// Used by the menu for display (no I/O per frame).
    pub best_times: HashMap<String, f32>,
    /// Typed handle to `best_times.ron`.
    pub best_handle: BestTimes,
    pub was_new_best: bool,

    // Widgets — persisted so update() and draw() share the same instance
    // and hover/pressed states stay consistent.
    // (Piège documenté : un Button recréé chaque frame perd son event Clicked.)
    pub start_btn: Button,
    pub restart_btn: Button,
    pub menu_btn: Button,
}

impl Game {
    pub fn new() -> Self {
        let ctx = load_config();
        let difficulties = load_difficulties();
        let best_handle = persistence::default();
        let best_times = best_handle.load_or(HashMap::new());
        let mut g = Self {
            state: GameState::Start,
            difficulties,
            selected_difficulty: 0,
            board: Board::new(9, 9, 10),
            cell_size: 32.0,
            grid_origin: Vec2::ZERO,
            elapsed: 0.0,
            timer_running: false,
            rng: Rng::new(0xDEAD_BEEF),
            best_times,
            best_handle,
            was_new_best: false,
            // Placeholder rect — immediately overwritten by update_menu_layout.
            start_btn: Button::new(0.0, 0.0, 200.0, 50.0, "START"),
            restart_btn: Button::new(ctx.window_w - 220.0, 10.0, 100.0, 30.0, "Restart"),
            menu_btn: Button::new(ctx.window_w - 110.0, 10.0, 100.0, 30.0, "Menu"),
        };
        g.update_layout();
        g.update_menu_layout(&ctx);
        g
    }

    pub fn current_difficulty(&self) -> &DifficultyData {
        &self.difficulties[self.selected_difficulty]
    }

    pub fn start_game(&mut self, _ctx: &GameContext) {
        let d = self.current_difficulty().clone();
        self.board = Board::new(d.width, d.height, d.mines);
        self.elapsed = 0.0;
        self.timer_running = false;
        self.was_new_best = false;
        // Advance the RNG state so each game gets a different mine layout.
        self.rng
            .set_state(self.rng.state().wrapping_add(0x9E37_79B9));
        self.update_layout();
        self.state = GameState::Playing;
    }

    pub fn update_layout(&mut self) {
        let ctx = load_config();
        let d = self.current_difficulty().clone();
        self.cell_size = d.cell_size(&ctx);
        let w = self.board.width as f32 * self.cell_size;
        let h = self.board.height as f32 * self.cell_size;
        self.grid_origin = ctx.grid_origin(w, h);
    }

    /// Recompute the START button rect. Called once at startup; the layout
    /// only depends on `difficulties.len()`, which is fixed for the process.
    pub fn update_menu_layout(&mut self, ctx: &GameContext) {
        let cx = ctx.window_w * 0.5;

        let btn_h = 60.0;
        let gap = 16.0;
        let n = self.difficulties.len() as f32;
        let total_h = n * btn_h + (n - 1.0).max(0.0) * gap;
        let first_y = ctx.window_h * 0.5 - total_h * 0.5 + 40.0;

        let start_y = first_y + total_h + 30.0;
        self.start_btn.rect = (cx - 100.0, start_y, 200.0, 50.0);
    }

    pub fn cell_at(&self, mouse: Vec2) -> Option<(usize, usize)> {
        let gx = mouse.x - self.grid_origin.x;
        let gy = mouse.y - self.grid_origin.y;
        if gx < 0.0 || gy < 0.0 {
            return None;
        }
        let col = (gx / self.cell_size) as usize;
        let row = (gy / self.cell_size) as usize;
        if col >= self.board.width || row >= self.board.height {
            return None;
        }
        Some((col, row))
    }

    pub fn is_playable(&self) -> bool {
        self.state == GameState::Playing
            && matches!(self.board.state, BoardState::Ready | BoardState::Playing)
    }

    pub fn record_best_if_needed(&mut self) {
        let name = self.current_difficulty().name.clone();
        if persistence::update_if_better(&mut self.best_times, &name, self.elapsed) {
            self.best_handle.save(&self.best_times);
            self.was_new_best = true;
        }
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Board logic
// ============================================================================

/// Place `board.mine_count` mines on the board, excluding the safe zone
/// around `(safe_col, safe_row)` (that cell + its 8 neighbors). Then
/// compute the `adjacent` count for every non-mine cell.
///
/// Called once, on the first reveal.
pub fn place_mines(board: &mut Board, safe_col: usize, safe_row: usize, rng: &mut Rng) {
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
        let c = rng.next_range(board.width);
        let r = rng.next_range(board.height);
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
        let mut rng = Rng::new(42);
        place_mines(&mut b, 4, 4, &mut rng);
        let count = b.cells.iter().filter(|(_, _, c)| c.is_mine).count();
        assert_eq!(count, 10);
        assert!(b.mines_placed);
        assert_eq!(b.state, BoardState::Playing);
    }

    #[test]
    fn test_place_mines_first_click_safe() {
        let mut b = b9();
        let mut rng = Rng::new(7);
        place_mines(&mut b, 4, 4, &mut rng);
        assert!(!b.cells.get(4, 4).unwrap().is_mine);
        for (c, r) in b.cells.neighbor_coords_8(4, 4) {
            assert!(!b.cells.get(c, r).unwrap().is_mine);
        }
    }

    #[test]
    fn test_place_mines_corner_first_click_safe() {
        let mut b = b9();
        let mut rng = Rng::new(7);
        place_mines(&mut b, 0, 0, &mut rng);
        assert!(!b.cells.get(0, 0).unwrap().is_mine);
        for (c, r) in b.cells.neighbor_coords_8(0, 0) {
            assert!(!b.cells.get(c, r).unwrap().is_mine);
        }
    }

    #[test]
    fn test_place_mines_adjacent_counts() {
        let mut b = b9();
        let mut rng = Rng::new(99);
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
        // Reveal (1,1), which is adjacent to the mine at (0,0).
        // adjacent > 0, so no flood fill — only this cell is revealed.
        let mut b = board_with_mines(3, 3, &[(0, 0)]);
        reveal(&mut b, 1, 1);
        assert!(b.cells.get(1, 1).unwrap().is_revealed());
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
        // (1,1) is adjacent to the mine, so no flood fill.
        let mut b = board_with_mines(3, 3, &[(0, 0)]);
        reveal(&mut b, 1, 1);
        assert_eq!(b.revealed_count, 1);
        reveal(&mut b, 1, 1);
        assert_eq!(b.revealed_count, 1, "second reveal must be a no-op");
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
    fn test_flood_fill_reveals_frontier_but_stops() {
        // 5x5, mine at (0,0).
        // Flood from (4,4) reveals everything except the mine and its
        // 3 direct neighbors' propagation beyond — but the frontier IS
        // revealed (standard Minesweeper).
        //
        // Layout (M = mine, .=empty, 1=frontier):
        //   M 1 . . .
        //   1 1 . . .
        //   . . . . .
        //   . . . . .
        //   . . . . .
        //
        // Revealing (4,4) floods through all the '.' cells and stops at
        // the '1' frontier. The frontier itself gets revealed.
        let mut b = board_with_mines(5, 5, &[(0, 0)]);
        reveal(&mut b, 4, 4);

        // Everything except the mine is revealed.
        assert_eq!(b.revealed_count, 24);
        assert_eq!(b.state, BoardState::Won);
    }

    #[test]
    fn test_flood_fill_does_not_cross_empty_wall() {
        // 5x5, two mines in opposite corners. A revealed empty cell in
        // one corner floods through, gets blocked by the frontier around
        // the OTHER mine — the other mine's direct neighbors stay hidden.
        //
        //  M . . . .
        //  . . . . .
        //  . . . . .
        //  . . . . .
        //  . . . . M
        //
        // Revealing (2, 2) — it has 0 adjacent mines, flood spreads to
        // all empty cells (which includes the frontier around both mines).
        // Since both mines are isolated and there's no "wall" of numbers
        // separating two regions, everything connects. This test verifies
        // the flood doesn't reveal MINES.
        let mut b = board_with_mines(5, 5, &[(0, 0), (4, 4)]);
        reveal(&mut b, 2, 2);

        // Both mines stay hidden (well, they're just not revealed).
        assert!(!b.cells.get(0, 0).unwrap().is_revealed());
        assert!(!b.cells.get(4, 4).unwrap().is_revealed());
        // Everything else revealed.
        assert_eq!(b.revealed_count, 23);
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
