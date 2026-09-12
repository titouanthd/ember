//! Board data model. Session A : types only, no rules yet.

use ember_stdlib::Grid;

/// Visual/logical state of a single cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellState {
    Hidden,
    Revealed,
    Flagged,
}

/// One cell of the board.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub is_mine: bool,
    /// 0..=8, number of adjacent mines. Computed at placement time.
    pub adjacent: u8,
    pub state: CellState,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            is_mine: false,
            adjacent: 0,
            state: CellState::Hidden,
        }
    }
}

/// High-level state of the board itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardState {
    /// Waiting for the first click. Mines not placed yet.
    Ready,
    Playing,
    Won,
    Lost,
}

/// The entire board. Session A : constructed but never modified after init.
pub struct Board {
    pub cells: Grid<Cell>,
    pub width: usize,
    pub height: usize,
    pub mine_count: usize,
    pub mines_placed: bool,
    pub revealed_count: usize,
    pub flagged_count: usize,
    pub state: BoardState,
}

impl Board {
    /// Create an empty board (no mines yet — first-click safety).
    pub fn new(width: usize, height: usize, mine_count: usize) -> Self {
        Self {
            cells: Grid::new(width, height, Cell::default()),
            width,
            height,
            mine_count,
            mines_placed: false,
            revealed_count: 0,
            flagged_count: 0,
            state: BoardState::Ready,
        }
    }

    /// Number of safe cells (total minus mines).
    pub fn safe_cell_count(&self) -> usize {
        self.width * self.height - self.mine_count
    }

    /// Mines remaining to be flagged. Can go negative if over-flagged.
    pub fn mines_remaining(&self) -> i32 {
        self.mine_count as i32 - self.flagged_count as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_board_starts_ready_and_empty() {
        let b = Board::new(9, 9, 10);
        assert_eq!(b.state, BoardState::Ready);
        assert!(!b.mines_placed);
        assert_eq!(b.revealed_count, 0);
        assert_eq!(b.flagged_count, 0);
        assert_eq!(b.mines_remaining(), 10);
    }

    #[test]
    fn test_safe_cell_count() {
        let b = Board::new(9, 9, 10);
        assert_eq!(b.safe_cell_count(), 71);
    }

    #[test]
    fn test_all_cells_default_hidden() {
        let b = Board::new(3, 3, 1);
        b.cells.for_each(|_, _, c| {
            assert_eq!(c.state, CellState::Hidden);
            assert!(!c.is_mine);
        });
    }
}