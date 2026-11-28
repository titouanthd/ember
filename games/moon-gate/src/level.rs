//! The static board of a level and its ASCII text form.
//!
//! A level is written as rows of characters (blank lines and leading or
//! trailing spaces are ignored):
//!
//! | Char | Piece |
//! |---|---|
//! | `.` | floor |
//! | `#` | wall |
//! | `@` | spirit start (on floor) |
//! | `L` | lantern (blocks, lights when the spirit is next to it) |
//! | `G` | moon gate (blocks until every lantern is lit, then wins) |
//! | `o` | stone (on floor, pushed one cell) |
//! | `1`–`9` | ink pool; each digit appears exactly twice (the twins) |
//! | `=` | paper bridge (crossed once, then a hole) |
//! | `x` | hole (blocks) |
//!
//! The edge of the board blocks like a wall. Stones and the spirit move;
//! they live in [`crate::State`], everything else lives here.

use ember_stdlib::Grid;
use std::fmt;

/// A board cell as `(col, row)`, row 0 at the top.
pub type Cell = (usize, usize);

/// Most lanterns a level may hold (the lit set is a `u32` mask).
pub const MAX_LANTERNS: usize = 32;
/// Most paper bridges a level may hold (the torn set is a `u64` mask).
pub const MAX_BRIDGES: usize = 64;

/// What a board cell is made of. Stones and the spirit are not tiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tile {
    Floor,
    Wall,
    Lantern,
    Gate,
    /// An ink pool; the digit pairs it with its twin.
    Pool(u8),
    Bridge,
    Hole,
}

/// A parsed level: the static board plus where the moving pieces start.
#[derive(Debug, Clone)]
pub struct Level {
    board: Grid<Tile>,
    start: Cell,
    stones: Vec<Cell>,
    lanterns: Vec<Cell>,
    bridges: Vec<Cell>,
    /// Pool twins, indexed by digit (`pools[d]` for digit `d`).
    pools: [Option<(Cell, Cell)>; 10],
}

/// Why a level's text could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    Empty,
    /// Rows must all have the same width; `row` is the first that differs.
    Ragged {
        row: usize,
    },
    UnknownChar {
        ch: char,
        cell: Cell,
    },
    NoStart,
    ManyStarts,
    NoGate,
    /// A pool digit must appear exactly twice.
    UnpairedPool {
        digit: u8,
        count: usize,
    },
    TooManyLanterns(usize),
    TooManyBridges(usize),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "the level has no rows"),
            Self::Ragged { row } => write!(f, "row {row} has a different width"),
            Self::UnknownChar { ch, cell } => write!(f, "unknown char {ch:?} at {cell:?}"),
            Self::NoStart => write!(f, "no spirit start '@'"),
            Self::ManyStarts => write!(f, "more than one spirit start '@'"),
            Self::NoGate => write!(f, "no moon gate 'G'"),
            Self::UnpairedPool { digit, count } => {
                write!(f, "pool {digit} appears {count} times, not 2")
            }
            Self::TooManyLanterns(n) => write!(f, "{n} lanterns (max {MAX_LANTERNS})"),
            Self::TooManyBridges(n) => write!(f, "{n} bridges (max {MAX_BRIDGES})"),
        }
    }
}

impl std::error::Error for ParseError {}

impl Level {
    /// Read a level from its ASCII form (see the module docs).
    pub fn parse(text: &str) -> Result<Self, ParseError> {
        let rows: Vec<&str> = text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect();
        let Some(first) = rows.first() else {
            return Err(ParseError::Empty);
        };
        let width = first.chars().count();
        let height = rows.len();

        let mut tiles = Vec::with_capacity(width * height);
        let mut start = None;
        let mut stones = Vec::new();
        let mut lanterns = Vec::new();
        let mut bridges = Vec::new();
        let mut has_gate = false;
        let mut pool_cells: [Vec<Cell>; 10] = Default::default();

        for (row, line) in rows.iter().enumerate() {
            if line.chars().count() != width {
                return Err(ParseError::Ragged { row });
            }
            for (col, ch) in line.chars().enumerate() {
                let cell = (col, row);
                let tile = match ch {
                    '.' => Tile::Floor,
                    '#' => Tile::Wall,
                    '@' => {
                        if start.replace(cell).is_some() {
                            return Err(ParseError::ManyStarts);
                        }
                        Tile::Floor
                    }
                    'L' => {
                        lanterns.push(cell);
                        Tile::Lantern
                    }
                    'G' => {
                        has_gate = true;
                        Tile::Gate
                    }
                    'o' => {
                        stones.push(cell);
                        Tile::Floor
                    }
                    '1'..='9' => {
                        let digit = ch as u8 - b'0';
                        pool_cells[digit as usize].push(cell);
                        Tile::Pool(digit)
                    }
                    '=' => {
                        bridges.push(cell);
                        Tile::Bridge
                    }
                    'x' => Tile::Hole,
                    _ => return Err(ParseError::UnknownChar { ch, cell }),
                };
                tiles.push(tile);
            }
        }

        let start = start.ok_or(ParseError::NoStart)?;
        if !has_gate {
            return Err(ParseError::NoGate);
        }
        if lanterns.len() > MAX_LANTERNS {
            return Err(ParseError::TooManyLanterns(lanterns.len()));
        }
        if bridges.len() > MAX_BRIDGES {
            return Err(ParseError::TooManyBridges(bridges.len()));
        }
        let mut pools = [None; 10];
        for (digit, cells) in pool_cells.iter().enumerate() {
            match cells.as_slice() {
                [] => {}
                [a, b] => pools[digit] = Some((*a, *b)),
                _ => {
                    return Err(ParseError::UnpairedPool {
                        digit: digit as u8,
                        count: cells.len(),
                    });
                }
            }
        }

        Ok(Self {
            board: Grid::from_vec(width, height, tiles),
            start,
            stones,
            lanterns,
            bridges,
            pools,
        })
    }

    pub fn width(&self) -> usize {
        self.board.width()
    }

    pub fn height(&self) -> usize {
        self.board.height()
    }

    /// The tile at `cell`, or `None` outside the board.
    pub fn tile(&self, cell: Cell) -> Option<Tile> {
        self.board.get(cell.0, cell.1).copied()
    }

    /// The spirit's start cell.
    pub fn start(&self) -> Cell {
        self.start
    }

    /// Where the stones start, in reading order.
    pub fn stones(&self) -> &[Cell] {
        &self.stones
    }

    /// Lantern cells, in reading order; a lantern's index is its bit in
    /// [`crate::State::lit`].
    pub fn lanterns(&self) -> &[Cell] {
        &self.lanterns
    }

    /// Bridge cells, in reading order; a bridge's index is its bit in
    /// [`crate::State::torn`].
    pub fn bridges(&self) -> &[Cell] {
        &self.bridges
    }

    /// The lit mask with every lantern lit.
    pub fn all_lit(&self) -> u32 {
        match self.lanterns.len() {
            MAX_LANTERNS => u32::MAX,
            n => (1u32 << n) - 1,
        }
    }

    /// The other pool of the pair `cell` belongs to.
    pub fn twin(&self, cell: Cell) -> Option<Cell> {
        let Some(Tile::Pool(digit)) = self.tile(cell) else {
            return None;
        };
        let (a, b) = self.pools[digit as usize]?;
        Some(if a == cell { b } else { a })
    }

    /// `cell` moved by `(dx, dy)`, or `None` if that leaves the board.
    pub fn offset(&self, cell: Cell, dx: isize, dy: isize) -> Option<Cell> {
        let col = cell.0.checked_add_signed(dx)?;
        let row = cell.1.checked_add_signed(dy)?;
        (col < self.width() && row < self.height()).then_some((col, row))
    }

    pub(crate) fn lantern_index(&self, cell: Cell) -> Option<usize> {
        self.lanterns.iter().position(|&c| c == cell)
    }

    pub(crate) fn bridge_index(&self, cell: Cell) -> Option<usize> {
        self.bridges.iter().position(|&c| c == cell)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_piece() {
        let level = Level::parse(
            "
            #######
            #@.L.G#
            #o=x11#
            #######
            ",
        )
        .unwrap();
        assert_eq!((level.width(), level.height()), (7, 4));
        assert_eq!(level.start(), (1, 1));
        assert_eq!(level.tile((1, 1)), Some(Tile::Floor));
        assert_eq!(level.tile((3, 1)), Some(Tile::Lantern));
        assert_eq!(level.tile((5, 1)), Some(Tile::Gate));
        assert_eq!(level.tile((1, 2)), Some(Tile::Floor));
        assert_eq!(level.tile((2, 2)), Some(Tile::Bridge));
        assert_eq!(level.tile((3, 2)), Some(Tile::Hole));
        assert_eq!(level.tile((4, 2)), Some(Tile::Pool(1)));
        assert_eq!(level.tile((0, 0)), Some(Tile::Wall));
        assert_eq!(level.tile((7, 0)), None);
        assert_eq!(level.stones(), &[(1, 2)]);
        assert_eq!(level.lanterns(), &[(3, 1)]);
        assert_eq!(level.bridges(), &[(2, 2)]);
        assert_eq!(level.twin((4, 2)), Some((5, 2)));
        assert_eq!(level.twin((5, 2)), Some((4, 2)));
        assert_eq!(level.twin((1, 1)), None);
        assert_eq!(level.all_lit(), 0b1);
    }

    #[test]
    fn offset_stays_on_the_board() {
        let level = Level::parse("@G").unwrap();
        assert_eq!(level.offset((0, 0), 1, 0), Some((1, 0)));
        assert_eq!(level.offset((0, 0), -1, 0), None);
        assert_eq!(level.offset((1, 0), 1, 0), None);
        assert_eq!(level.offset((0, 0), 0, 1), None);
    }

    #[test]
    fn no_lanterns_means_all_lit_is_zero() {
        assert_eq!(Level::parse("@G").unwrap().all_lit(), 0);
    }

    #[test]
    fn rejects_bad_text() {
        assert_eq!(Level::parse("  \n ").unwrap_err(), ParseError::Empty);
        assert!(matches!(
            Level::parse("@G\n.").unwrap_err(),
            ParseError::Ragged { row: 1 }
        ));
        assert!(matches!(
            Level::parse("@G?").unwrap_err(),
            ParseError::UnknownChar {
                ch: '?',
                cell: (2, 0)
            }
        ));
        assert_eq!(Level::parse("..G").unwrap_err(), ParseError::NoStart);
        assert_eq!(Level::parse("@@G").unwrap_err(), ParseError::ManyStarts);
        assert_eq!(Level::parse("@..").unwrap_err(), ParseError::NoGate);
        assert_eq!(
            Level::parse("@1G").unwrap_err(),
            ParseError::UnpairedPool { digit: 1, count: 1 }
        );
        assert_eq!(
            Level::parse("@222G").unwrap_err(),
            ParseError::UnpairedPool { digit: 2, count: 3 }
        );
        let many = format!("@G{}", "L".repeat(MAX_LANTERNS + 1));
        assert_eq!(
            Level::parse(&many).unwrap_err(),
            ParseError::TooManyLanterns(MAX_LANTERNS + 1)
        );
        let bridges = format!("@G{}", "=".repeat(MAX_BRIDGES + 1));
        assert_eq!(
            Level::parse(&bridges).unwrap_err(),
            ParseError::TooManyBridges(MAX_BRIDGES + 1)
        );
    }

    #[test]
    fn full_lantern_mask() {
        let text = format!("@G{}", "L".repeat(MAX_LANTERNS));
        assert_eq!(Level::parse(&text).unwrap().all_lit(), u32::MAX);
    }
}
