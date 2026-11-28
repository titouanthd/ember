//! Moon Gate — a sliding grid puzzle (Game 18).
//!
//! A paper-lantern spirit slides across ink-wash courtyards until something
//! stops it. Light every lantern to open the moon gate, then slide into it.
//!
//! This crate is pure for now (round G1): no engine runtime, no rendering.
//! - [`level`]: the static board and its ASCII text form.
//! - [`rules`]: one move, the state it produces, and an undo history.
//! - [`solver`]: breadth-first search for the shortest solution (the par).
//! - [`levels`]: draft levels, each proven solvable in par by the tests.

pub mod level;
pub mod levels;
pub mod rules;
pub mod solver;

pub use level::{Cell, Level, ParseError, Tile};
pub use rules::{Dir, Event, Game, MoveError, State, apply};
pub use solver::{DEFAULT_STATE_LIMIT, solve, solve_within};
