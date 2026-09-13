//! Game logic: deck creation, reveal, match, no-match, win detection.

use std::collections::HashMap;

use ember_core::app::GameState;
use ember_core::rng::Rng;
use ember_stdlib::grid::Grid;

use crate::components::{Card, Symbol};
use crate::config::GameContext;
use crate::difficulties::DifficultyData;
use crate::persistence::BestTimes;

/// The set of symbols used by the deck. The terminal/dev theme.
pub const SYMBOLS: &[char] = &[
    '$', '§', '>', '{', '}', '[', ']', '(', ')', '*', '&', '#', '@', '%', '!',
    '?', '/', '\\', '|', '~', '^', '=', '+', '-', '<',
];

/// Build a shuffled deck of `pairs * 2` cards for the given difficulty.
///
/// Uses the first `pairs` symbols from `SYMBOLS` (in order), duplicated
/// twice, then Fisher-Yates shuffled with the given RNG. Deterministic
/// given the same seed.
pub fn build_deck(pairs: usize, rng: &mut Rng) -> Vec<Card> {
    let mut cards: Vec<Card> = Vec::with_capacity(pairs * 2);
    for i in 0..pairs {
        let sym = Symbol(SYMBOLS[i % SYMBOLS.len()]);
        cards.push(Card::new(sym));
        cards.push(Card::new(sym));
    }
    // Fisher-Yates shuffle.
    let n = cards.len();
    for i in (1..n).rev() {
        let j = rng.next_range(i + 1);
        cards.swap(i, j);
    }
    cards
}

/// What phase the player's selection is in.
#[derive(Debug, Clone, PartialEq)]
pub enum SelectionPhase {
    /// No card selected. Clicks on hidden cards are accepted.
    Idle,
    /// One card flipped, waiting for the second.
    FirstPick { col: usize, row: usize },
    /// Two cards flipped. Waiting for the no-match delay to elapse.
    Resolving { col_a: usize, row_a: usize, col_b: usize, row_b: usize, until: f32 },
}

/// Top-level state for one Memory session.
///
/// Same convention as the other games: one struct owns everything mutable.
pub struct Game {
    pub state: GameState,
    pub difficulties: Vec<DifficultyData>,
    pub selected_difficulty: usize,

    pub board: Grid<Card>,
    pub selection: SelectionPhase,
    pub pairs_found: usize,
    pub total_pairs: usize,
    pub elapsed: f32,
    pub timer_running: bool,

    pub rng: Rng,
    pub best_times: HashMap<String, f32>,
    pub best_handle: BestTimes,
    pub was_new_best: bool,
}

impl Game {
    pub fn new() -> Self {
        let difficulties = crate::difficulties::load_difficulties();
        let best_handle = crate::persistence::default();
        let best_times = best_handle.load_or(HashMap::new());

        // Placeholder board. Replaced by `start_game`.
        let board = Grid::new(1, 1, Card::new(Symbol('?')));

        Self {
            state: GameState::Start,
            difficulties,
            selected_difficulty: 0,
            board,
            selection: SelectionPhase::Idle,
            pairs_found: 0,
            total_pairs: 0,
            elapsed: 0.0,
            timer_running: false,
            rng: Rng::new(0xDEAD_BEEF),
            best_times,
            best_handle,
            was_new_best: false,
        }
    }

    pub fn current_difficulty(&self) -> &DifficultyData {
        &self.difficulties[self.selected_difficulty]
    }

    /// Start a new game with the currently selected difficulty.
    pub fn start_game(&mut self, _ctx: &GameContext) {
        let d = self.current_difficulty().clone();
        let cards = build_deck(d.pairs, &mut self.rng);
        self.board = Grid::from_vec(d.cols, d.rows, cards);
        self.selection = SelectionPhase::Idle;
        self.pairs_found = 0;
        self.total_pairs = d.pairs;
        self.elapsed = 0.0;
        self.timer_running = false;
        self.was_new_best = false;
        self.state = GameState::Playing;
    }

    /// Is the game currently accepting clicks?
    pub fn is_playable(&self) -> bool {
        self.state == GameState::Playing
            && !matches!(self.selection, SelectionPhase::Resolving { .. })
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}