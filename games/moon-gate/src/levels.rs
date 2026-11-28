//! Draft levels, two per piece, in the ASCII form of [`Level`].
//!
//! These prove the rules and the solver on real puzzles. From round G2 the
//! levels live in `scenes/levels/<chapter>-<nn>.escene`, painted in the
//! editor; the par check in `tests/levels.rs` moves with them.

use crate::level::{Level, ParseError};

/// A level as shipped: an id, a name, its par and its board.
#[derive(Debug, Clone, Copy)]
pub struct LevelDef {
    /// `<chapter>-<nn>`, the future `.escene` file name.
    pub id: &'static str,
    pub name: &'static str,
    /// The fewest moves that win; the tests check the solver agrees.
    pub par: usize,
    pub text: &'static str,
}

impl LevelDef {
    pub fn parse(&self) -> Result<Level, ParseError> {
        Level::parse(self.text)
    }

    /// The chapter number, from the id (`"2-01"` → 2).
    pub fn chapter(&self) -> u32 {
        self.id
            .split('-')
            .next()
            .and_then(|c| c.parse().ok())
            .unwrap_or(0)
    }
}

/// Every draft level, in play order.
pub const LEVELS: &[LevelDef] = &[
    // Chapter 1 — Courtyard: wall, lantern, gate.
    LevelDef {
        id: "1-01",
        name: "First Light",
        par: 3,
        text: "
            #######
            #@...L#
            #.....#
            #....G#
            #######
        ",
    },
    LevelDef {
        id: "1-02",
        name: "Two Lanterns",
        par: 5,
        text: "
            ########
            #@....L#
            #.#....#
            #......#
            #L...#.#
            #....G.#
            ########
        ",
    },
    // Chapter 2 — Garden: stone.
    LevelDef {
        id: "2-01",
        name: "Heavy Stone",
        par: 8,
        text: "
            ###G####
            #....o.#
            #...####
            #......#
            #o..#L.#
            #@..L.##
            ########
        ",
    },
    LevelDef {
        id: "2-02",
        name: "Rock Garden",
        par: 8,
        text: "
            ########
            #...#.##
            ##..@o.#
            #.#.L.L#
            #....o.#
            #..#...#
            #####G##
        ",
    },
    // Chapter 3 — Pond: ink pool.
    LevelDef {
        id: "3-01",
        name: "Ink Pond",
        par: 4,
        text: "
            #########
            #@..1#..#
            #....#L.#
            #.G..#.1#
            #########
        ",
    },
    LevelDef {
        id: "3-02",
        name: "Twin Pools",
        par: 8,
        text: "
            ########
            ##...1.#
            #..L...#
            #.#.##L#
            #1...#.#
            #..@...#
            ####G###
        ",
    },
    // Chapter 4 (stretch) — Bridge: paper bridge.
    LevelDef {
        id: "4-01",
        name: "Paper Bridge",
        par: 5,
        text: "
            #######
            #@=..L#
            #.#.#.#
            #..=..#
            #L#G#.#
            #######
        ",
    },
    LevelDef {
        id: "4-02",
        name: "Torn Paths",
        par: 8,
        text: "
            ##G#####
            #.@....#
            #=..=#.#
            ##L....#
            ##...L.#
            ###..=.#
            ########
        ",
    },
];
