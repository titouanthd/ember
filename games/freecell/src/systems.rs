//! Game logic: rules, moves, super-moves, undo/redo. To be filled in
//! Session 13.

use std::collections::HashMap;

use ember_core::app::GameState;
use ember_core::rng::Rng;

use crate::components::Card;
use crate::config::GameContext;
use crate::drag::DragState;
use crate::persistence::BestTimes;

/// Top-level state for one FreeCell session.
///
/// Same convention as the other games: one struct owns everything mutable.
pub struct Game {
    pub state: GameState,

    pub columns: [Vec<Card>; 8],
    pub free_cells: [Option<Card>; 4],
    pub foundations: [Vec<Card>; 4],

    pub drag: DragState,
    pub moves: u32,
    pub elapsed: f32,
    pub timer_running: bool,
    pub seed: u32,

    pub rng: Rng,
    pub best_times: HashMap<String, f32>,
    pub best_handle: BestTimes,
    pub was_new_best: bool,
}

impl Game {
    pub fn new() -> Self {
        let best_handle = crate::persistence::default();
        let best_times = best_handle.load_or(HashMap::new());

        Self {
            state: GameState::Start,
            columns: Default::default(),
            free_cells: Default::default(),
            foundations: Default::default(),
            drag: DragState::Idle,
            moves: 0,
            elapsed: 0.0,
            timer_running: false,
            seed: 1,
            rng: Rng::new(1),
            best_times,
            best_handle,
            was_new_best: false,
        }
    }

    /// Start a new game with the given seed.
    pub fn start_game(&mut self, seed: u32, _ctx: &GameContext) {
        // TODO Session 13 : distribuer les cartes, reset moves/elapsed.
        self.seed = seed;
        self.rng = Rng::new(seed);
        self.moves = 0;
        self.elapsed = 0.0;
        self.timer_running = false;
        self.was_new_best = false;
        self.state = GameState::Playing;
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}