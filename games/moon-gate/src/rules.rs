//! The rules of Moon Gate: one move, the state it produces, and undo.
//!
//! Pure functions over [`Level`] and [`State`], no engine. The rules:
//!
//! 1. **Slide.** A move picks a direction; the spirit slides cell by cell
//!    until the next cell blocks it. Walls, lanterns, holes, the board edge
//!    and a closed gate block.
//! 2. **Lanterns.** Any lantern orthogonally next to the spirit lights: at
//!    the start, and on every cell it enters during a slide. Lit lanterns
//!    stay lit.
//! 3. **Gate.** Once every lantern is lit the gate is open; sliding into it
//!    wins. A level with no lanterns starts with the gate open.
//! 4. **Stone.** Sliding into a stone pushes it one cell if that cell is
//!    plain floor with no stone, and the spirit stops where it is. If the
//!    stone cannot move, the spirit just stops. Stones never cross pools,
//!    bridges, holes or the gate.
//! 5. **Ink pool.** Entering a pool sends the spirit out of its twin; the
//!    slide goes on in the same direction from the twin.
//! 6. **Paper bridge.** Floor that tears into a hole once the spirit leaves
//!    it (passing over counts).
//! 7. A move that changes nothing is not a move (it is not counted). A slide
//!    that would never end (pools in a cycle) is refused.

use crate::level::{Cell, Level, Tile};
use std::collections::HashSet;
use std::fmt;

/// A move direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

impl Dir {
    pub const ALL: [Dir; 4] = [Dir::Up, Dir::Down, Dir::Left, Dir::Right];

    /// `(dx, dy)`, with `y` growing downwards.
    pub fn delta(self) -> (isize, isize) {
        match self {
            Dir::Up => (0, -1),
            Dir::Down => (0, 1),
            Dir::Left => (-1, 0),
            Dir::Right => (1, 0),
        }
    }

    /// `U`, `D`, `L` or `R` (either case) as a direction.
    pub fn from_char(ch: char) -> Option<Dir> {
        match ch.to_ascii_uppercase() {
            'U' => Some(Dir::Up),
            'D' => Some(Dir::Down),
            'L' => Some(Dir::Left),
            'R' => Some(Dir::Right),
            _ => None,
        }
    }

    /// Read a move string such as `"RDL"`; `None` on any other char.
    pub fn parse_moves(text: &str) -> Option<Vec<Dir>> {
        text.chars().map(Dir::from_char).collect()
    }

    /// Write moves as a string such as `"RDL"`.
    pub fn format_moves(moves: &[Dir]) -> String {
        moves.iter().map(ToString::to_string).collect()
    }
}

impl fmt::Display for Dir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ch = match self {
            Dir::Up => 'U',
            Dir::Down => 'D',
            Dir::Left => 'L',
            Dir::Right => 'R',
        };
        write!(f, "{ch}")
    }
}

/// Everything that changes during play. Small, cloneable and hashable, so
/// undo keeps copies and the solver uses it as a search key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct State {
    /// The spirit's cell.
    pub spirit: Cell,
    /// Bit `i` set: lantern `i` of [`Level::lanterns`] is lit.
    pub lit: u32,
    /// Stone cells, sorted.
    pub stones: Vec<Cell>,
    /// Bit `i` set: bridge `i` of [`Level::bridges`] is torn.
    pub torn: u64,
    /// The spirit has entered the open gate.
    pub won: bool,
}

/// What happened during a move, in order. The view (G2+) animates these.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// The spirit moved one cell.
    Step { from: Cell, to: Cell },
    /// The spirit entered pool `from` and came out of its twin `to`.
    Teleport { from: Cell, to: Cell },
    /// A stone was pushed one cell.
    Push { from: Cell, to: Cell },
    /// Lantern `lantern` (index in [`Level::lanterns`]) lit.
    Lit { lantern: usize },
    /// Bridge `bridge` (index in [`Level::bridges`]) tore into a hole.
    Tore { bridge: usize },
    /// The last lantern lit; the gate is open.
    GateOpened,
    /// The spirit entered the open gate.
    Won,
}

/// Why a move was refused. A refused move leaves the state unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveError {
    /// The level is already won.
    Won,
    /// Nothing moved: the spirit is blocked and no stone can be pushed.
    Blocked,
    /// The slide would loop through ink pools forever.
    Endless,
}

impl fmt::Display for MoveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            MoveError::Won => "the level is already won",
            MoveError::Blocked => "nothing can move that way",
            MoveError::Endless => "the slide would never end",
        })
    }
}

impl std::error::Error for MoveError {}

impl State {
    /// The state a level starts in (lanterns next to the start are lit).
    pub fn initial(level: &Level) -> Self {
        let mut stones = level.stones().to_vec();
        stones.sort_unstable();
        let mut state = Self {
            spirit: level.start(),
            lit: 0,
            stones,
            torn: 0,
            won: false,
        };
        state.light_around(level, &mut Vec::new());
        state
    }

    pub fn gate_open(&self, level: &Level) -> bool {
        self.lit == level.all_lit()
    }

    pub fn has_stone(&self, cell: Cell) -> bool {
        self.stones.binary_search(&cell).is_ok()
    }

    pub fn is_lit(&self, lantern: usize) -> bool {
        self.lit & (1 << lantern) != 0
    }

    pub fn is_torn(&self, bridge: usize) -> bool {
        self.torn & (1 << bridge) != 0
    }

    /// The board as text, in the level legend plus: `*` lit lantern,
    /// `O` open gate, `x` torn bridge. Meant for tests and debugging.
    pub fn render(&self, level: &Level) -> String {
        let mut out = String::new();
        for row in 0..level.height() {
            for col in 0..level.width() {
                let cell = (col, row);
                let ch = if self.spirit == cell {
                    '@'
                } else if self.has_stone(cell) {
                    'o'
                } else {
                    match level.tile(cell) {
                        Some(Tile::Floor) | None => '.',
                        Some(Tile::Wall) => '#',
                        Some(Tile::Hole) => 'x',
                        Some(Tile::Pool(d)) => (b'0' + d) as char,
                        Some(Tile::Lantern) => match level.lantern_index(cell) {
                            Some(i) if self.is_lit(i) => '*',
                            _ => 'L',
                        },
                        Some(Tile::Gate) if self.gate_open(level) => 'O',
                        Some(Tile::Gate) => 'G',
                        Some(Tile::Bridge) => match level.bridge_index(cell) {
                            Some(i) if self.is_torn(i) => 'x',
                            _ => '=',
                        },
                    }
                };
                out.push(ch);
            }
            out.push('\n');
        }
        out
    }

    /// Light every unlit lantern next to the spirit.
    fn light_around(&mut self, level: &Level, events: &mut Vec<Event>) {
        let was_open = self.gate_open(level);
        for dir in Dir::ALL {
            let (dx, dy) = dir.delta();
            let Some(cell) = level.offset(self.spirit, dx, dy) else {
                continue;
            };
            if let Some(i) = level.lantern_index(cell)
                && !self.is_lit(i)
            {
                self.lit |= 1 << i;
                events.push(Event::Lit { lantern: i });
            }
        }
        if !was_open && self.gate_open(level) {
            events.push(Event::GateOpened);
        }
    }

    /// Tear the bridge under the spirit, if any.
    fn leave(&mut self, level: &Level, events: &mut Vec<Event>) {
        if let Some(i) = level.bridge_index(self.spirit)
            && !self.is_torn(i)
        {
            self.torn |= 1 << i;
            events.push(Event::Tore { bridge: i });
        }
    }

    fn step(&mut self, to: Cell, events: &mut Vec<Event>) {
        events.push(Event::Step {
            from: self.spirit,
            to,
        });
        self.spirit = to;
    }

    fn stone_can_enter(&self, level: &Level, cell: Cell) -> bool {
        level.tile(cell) == Some(Tile::Floor) && !self.has_stone(cell)
    }

    fn move_stone(&mut self, from: Cell, to: Cell) {
        if let Ok(i) = self.stones.binary_search(&from) {
            self.stones.remove(i);
        }
        let at = self.stones.binary_search(&to).unwrap_or_else(|i| i);
        self.stones.insert(at, to);
    }
}

/// Play one move. Returns the new state and what happened, or why nothing
/// did (the input state is never changed).
pub fn apply(level: &Level, state: &State, dir: Dir) -> Result<(State, Vec<Event>), MoveError> {
    if state.won {
        return Err(MoveError::Won);
    }
    let (dx, dy) = dir.delta();
    let mut s = state.clone();
    let mut events = Vec::new();
    // Lanterns and bridges only change one way, so a slide loops forever
    // exactly when it comes back to the same (cell, lit, torn).
    let mut seen = HashSet::from([(s.spirit, s.lit, s.torn)]);

    loop {
        let Some(next) = level.offset(s.spirit, dx, dy) else {
            break;
        };
        if s.has_stone(next) {
            if let Some(beyond) = level.offset(next, dx, dy)
                && s.stone_can_enter(level, beyond)
            {
                s.move_stone(next, beyond);
                events.push(Event::Push {
                    from: next,
                    to: beyond,
                });
            }
            break;
        }
        match level.tile(next) {
            None | Some(Tile::Wall | Tile::Lantern | Tile::Hole) => break,
            Some(Tile::Bridge) if level.bridge_index(next).is_some_and(|i| s.is_torn(i)) => break,
            Some(Tile::Gate) => {
                if s.gate_open(level) {
                    s.leave(level, &mut events);
                    s.step(next, &mut events);
                    s.won = true;
                    events.push(Event::Won);
                }
                break;
            }
            Some(Tile::Floor | Tile::Bridge) => {
                s.leave(level, &mut events);
                s.step(next, &mut events);
                s.light_around(level, &mut events);
            }
            Some(Tile::Pool(_)) => {
                s.leave(level, &mut events);
                s.step(next, &mut events);
                s.light_around(level, &mut events);
                let twin = level.twin(next).expect("a parsed pool always has a twin");
                events.push(Event::Teleport {
                    from: next,
                    to: twin,
                });
                s.spirit = twin;
                s.light_around(level, &mut events);
            }
        }
        if !seen.insert((s.spirit, s.lit, s.torn)) {
            return Err(MoveError::Endless);
        }
    }

    if events.is_empty() {
        Err(MoveError::Blocked)
    } else {
        Ok((s, events))
    }
}

/// A level being played: the current state plus an undo history.
#[derive(Debug, Clone)]
pub struct Game {
    level: Level,
    state: State,
    history: Vec<State>,
}

impl Game {
    pub fn new(level: Level) -> Self {
        let state = State::initial(&level);
        Self {
            level,
            state,
            history: Vec::new(),
        }
    }

    pub fn level(&self) -> &Level {
        &self.level
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    /// Moves played since the start (undone moves are not counted).
    pub fn moves(&self) -> usize {
        self.history.len()
    }

    pub fn is_won(&self) -> bool {
        self.state.won
    }

    /// Won in `par` moves or fewer (earns the star).
    pub fn within_par(&self, par: usize) -> bool {
        self.is_won() && self.moves() <= par
    }

    /// Play a move. A refused move changes nothing and is not counted.
    pub fn play(&mut self, dir: Dir) -> Result<Vec<Event>, MoveError> {
        let (next, events) = apply(&self.level, &self.state, dir)?;
        self.history.push(std::mem::replace(&mut self.state, next));
        Ok(events)
    }

    /// Take back the last move; `false` if there is none.
    pub fn undo(&mut self) -> bool {
        match self.history.pop() {
            Some(previous) => {
                self.state = previous;
                true
            }
            None => false,
        }
    }

    /// Back to the start, with an empty history.
    pub fn restart(&mut self) {
        self.state = State::initial(&self.level);
        self.history.clear();
    }
}
