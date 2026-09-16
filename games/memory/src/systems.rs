//! Game logic: deck creation, reveal, match, no-match, win detection.

use std::collections::HashMap;

use macroquad::prelude::Rect;

use ember_core::app::GameState;
use ember_core::rng::Rng;
use ember_stdlib::grid::Grid;
use ember_stdlib::ui::button::Button;

use crate::components::{Card, CardState, Symbol};
use crate::config::GameContext;
use crate::difficulties::DifficultyData;
use crate::persistence::{self, BestTimes};

/// The set of symbols used by the deck. Terminal / dev theme.
///
/// 25 symbols — enough for Hard (32 pairs) with a bit of variety; Medium
/// uses 18, Easy uses 8. Always the first N in order (deterministic).
pub const SYMBOLS: &[char] = &[
    '$', '§', '>', '{', '}', '[', ']', '(', ')', '*', '&', '#', '@', '%', '!', '?', '/', '\\', '|',
    '~', '^', '=', '+', '-', '<',
];

// ============================================================================
// Deck
// ============================================================================

/// Build a shuffled deck of `pairs * 2` cards for the given difficulty.
///
/// Uses the first `pairs` symbols from [`SYMBOLS`] (in order), duplicated
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

// ============================================================================
// Selection phase
// ============================================================================

/// What the player's selection is currently doing.
#[derive(Debug, Clone, PartialEq)]
pub enum SelectionPhase {
    /// No card selected. Clicks on hidden cards are accepted.
    Idle,
    /// One card flipped, waiting for the second.
    FirstPick { col: usize, row: usize },
    /// Two cards flipped but not matching. Waiting for the delay to elapse
    /// before flipping them back. Clicks are ignored during this phase.
    Resolving {
        col_a: usize,
        row_a: usize,
        col_b: usize,
        row_b: usize,
        /// Seconds remaining before the cards flip back.
        remaining: f32,
    },
}

// ============================================================================
// Game — top-level state
// ============================================================================

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

    // Layout cache: computed on start_game.
    pub board_origin: (f32, f32),
    pub card_size: f32,

    // Widgets — persisted so update() and draw() share the same instance
    // and hover/pressed states stay consistent.
    // (Piège documenté : un Button recréé chaque frame perd son event Clicked.)
    pub start_btn: Button,
    pub restart_btn: Button,
    pub menu_btn: Button,
}

impl Game {
    pub fn new() -> Self {
        let ctx = crate::config::load_config();
        let difficulties = crate::difficulties::load_difficulties();
        let best_handle = persistence::default();
        let best_times = best_handle.load_or(HashMap::new());

        // Placeholder board, replaced by `start_game`.
        let board = Grid::new(1, 1, Card::new(Symbol('?')));

        let mut g = Self {
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
            board_origin: (0.0, 0.0),
            card_size: 0.0,
            start_btn: Button::new(0.0, 0.0, 200.0, 50.0, "START"),
            restart_btn: Button::new(ctx.window_w - 240.0, 10.0, 100.0, 30.0, "Restart"),
            menu_btn: Button::new(ctx.window_w - 120.0, 10.0, 100.0, 30.0, "Menu"),
        };
        g.update_menu_layout(&ctx);
        g
    }

    /// Test-only constructor: build a game with an explicit best-times
    /// handle (typically a temp file) so tests don't touch the project's
    /// `best_times.ron`.
    ///
    /// Always available (not `#[cfg(test)]`) because integration tests
    /// live in `tests/` and don't see `cfg(test)` items.
    pub fn with_best_handle(handle: BestTimes) -> Self {
        let mut g = Self::new();
        g.best_times = handle.load_or(HashMap::new());
        g.best_handle = handle;
        g
    }

    pub fn current_difficulty(&self) -> &DifficultyData {
        &self.difficulties[self.selected_difficulty]
    }

    /// Start a new game with the currently selected difficulty.
    pub fn start_game(&mut self, ctx: &GameContext) {
        let d = self.current_difficulty().clone();
        let cards = build_deck(d.pairs, &mut self.rng);
        self.board = Grid::from_vec(d.cols, d.rows, cards);
        self.selection = SelectionPhase::Idle;
        self.pairs_found = 0;
        self.total_pairs = d.pairs;
        self.elapsed = 0.0;
        self.timer_running = false;
        self.was_new_best = false;
        self.update_layout(&d, ctx);
        self.state = GameState::Playing;
    }

    /// Compute `board_origin` and `card_size` for the given difficulty.
    fn update_layout(&mut self, d: &DifficultyData, ctx: &GameContext) {
        let card = d.card_size(ctx);
        let board_w = d.cols as f32 * card + (d.cols.saturating_sub(1) as f32) * ctx.card_gap;
        let board_h = d.rows as f32 * card + (d.rows.saturating_sub(1) as f32) * ctx.card_gap;
        let origin = ctx.grid_origin(board_w, board_h);
        self.board_origin = (origin.x, origin.y);
        self.card_size = card;
    }

    /// Recompute the START button rect. Called once at startup; the menu
    /// layout only depends on `difficulties.len()`.
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

    /// Is the game currently accepting clicks on cards?
    pub fn is_playable(&self) -> bool {
        self.state == GameState::Playing
            && !matches!(self.selection, SelectionPhase::Resolving { .. })
    }

    /// True if the given card is currently in the "wrong pair" state
    /// (waiting for the no-match delay to elapse before flipping back).
    ///
    /// Used by the renderer to show a red border on mismatched cards.
    pub fn is_resolving_at(&self, col: usize, row: usize) -> bool {
        match self.selection {
            SelectionPhase::Resolving {
                col_a,
                row_a,
                col_b,
                row_b,
                ..
            } => (col == col_a && row == row_a) || (col == col_b && row == row_b),
            _ => false,
        }
    }

    /// Screen-space rect (x, y, w, h) for a card at (col, row).
    pub fn card_rect(&self, col: usize, row: usize, ctx: &GameContext) -> Rect {
        let size = self.card_size;
        let x = self.board_origin.0 + col as f32 * (size + ctx.card_gap);
        let y = self.board_origin.1 + row as f32 * (size + ctx.card_gap);
        Rect::new(x, y, size, size)
    }

    /// Convert a mouse position to grid coordinates, if it lands on a card.
    pub fn cell_at(&self, mouse: glam::Vec2, ctx: &GameContext) -> Option<(usize, usize)> {
        let cols = self.board.width();
        let rows = self.board.height();
        for row in 0..rows {
            for col in 0..cols {
                let r = self.card_rect(col, row, ctx);
                if mouse.x >= r.x && mouse.x <= r.x + r.w && mouse.y >= r.y && mouse.y <= r.y + r.h
                {
                    return Some((col, row));
                }
            }
        }
        None
    }

    /// Flip a card. Applies the match / no-match logic and updates
    /// `selection`. Ignores clicks that aren't allowed (not playing, card
    /// already flipped or matched, same card twice, resolving phase).
    pub fn reveal(&mut self, col: usize, row: usize, ctx: &GameContext) {
        if self.state != GameState::Playing {
            return;
        }
        if matches!(self.selection, SelectionPhase::Resolving { .. }) {
            return;
        }

        let card = match self.board.get(col, row) {
            Some(c) => *c,
            None => return,
        };
        if !card.is_clickable() {
            return;
        }

        // Flip it.
        self.board.get_mut(col, row).unwrap().state = CardState::Flipped;

        // Start the timer on the first successful reveal.
        if !self.timer_running {
            self.timer_running = true;
        }

        match self.selection {
            SelectionPhase::Idle => {
                self.selection = SelectionPhase::FirstPick { col, row };
            }
            SelectionPhase::FirstPick { col: ca, row: ra } => {
                // Safety: cannot pick the same card twice.
                if ca == col && ra == row {
                    return;
                }
                let first = *self.board.get(ca, ra).unwrap();
                let second = card;

                if first.symbol == second.symbol {
                    // Match.
                    self.board.get_mut(ca, ra).unwrap().state = CardState::Matched;
                    self.board.get_mut(col, row).unwrap().state = CardState::Matched;
                    self.pairs_found += 1;
                    self.selection = SelectionPhase::Idle;
                    self.check_win();
                } else {
                    // No match: start resolving phase.
                    self.selection = SelectionPhase::Resolving {
                        col_a: ca,
                        row_a: ra,
                        col_b: col,
                        row_b: row,
                        remaining: ctx.no_match_delay,
                    };
                }
            }
            SelectionPhase::Resolving { .. } => unreachable!("guarded above"),
        }
    }

    /// Advance time. Handles the game timer and the no-match delay.
    pub fn tick(&mut self, dt: f32) {
        if self.timer_running {
            self.elapsed += dt;
        }

        // Resolve a pending no-match.
        if let SelectionPhase::Resolving {
            col_a,
            row_a,
            col_b,
            row_b,
            remaining,
        } = self.selection
        {
            let next = remaining - dt;
            if next <= 0.0 {
                self.board.get_mut(col_a, row_a).unwrap().state = CardState::Hidden;
                self.board.get_mut(col_b, row_b).unwrap().state = CardState::Hidden;
                self.selection = SelectionPhase::Idle;
            } else {
                self.selection = SelectionPhase::Resolving {
                    col_a,
                    row_a,
                    col_b,
                    row_b,
                    remaining: next,
                };
            }
        }
    }

    /// If all pairs are matched, transition to `Win`.
    fn check_win(&mut self) {
        if self.pairs_found == self.total_pairs {
            self.state = GameState::Win;
            self.timer_running = false;
            self.record_best_if_needed();
        }
    }

    /// Persist the current time if it beats the record for the current
    /// difficulty.
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
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> GameContext {
        crate::config::load_config()
    }

    /// Build a game with a known difficulty and a deterministic deck.
    fn game_with(pairs: usize, seed: u32, ctx: &GameContext) -> Game {
        let mut g = Game::new();
        // Force Easy-like difficulty: use 4x4 for 8 pairs if pairs==8,
        // else find a difficulty matching.
        let d = g
            .difficulties
            .iter()
            .find(|d| d.pairs == pairs)
            .cloned()
            .unwrap_or_else(|| DifficultyData {
                name: "Test".into(),
                cols: 4,
                rows: 4,
                pairs: 8,
            });
        g.selected_difficulty = 0; // will be replaced below
        // Override selected difficulty with a synthetic one.
        g.difficulties = vec![d.clone()];
        g.selected_difficulty = 0;
        g.rng = Rng::new(seed);
        g.start_game(ctx);
        g
    }

    // --- deck ---

    #[test]
    fn test_deck_has_two_of_each_symbol() {
        let mut rng = Rng::new(42);
        let cards = build_deck(8, &mut rng);
        assert_eq!(cards.len(), 16);

        // Count occurrences of each symbol.
        let mut counts: std::collections::HashMap<char, usize> = std::collections::HashMap::new();
        for c in &cards {
            *counts.entry(c.symbol.as_char()).or_insert(0) += 1;
        }
        for (sym, n) in &counts {
            assert_eq!(*n, 2, "symbol '{sym}' appears {n} times");
        }
        assert_eq!(counts.len(), 8);
    }

    #[test]
    fn test_deck_is_shuffled_deterministic() {
        let mut rng_a = Rng::new(123);
        let mut rng_b = Rng::new(123);
        let a = build_deck(8, &mut rng_a);
        let b = build_deck(8, &mut rng_b);
        assert_eq!(a, b);

        // Two different seeds should (almost surely) give different orders.
        let mut rng_c = Rng::new(999);
        let c = build_deck(8, &mut rng_c);
        assert_ne!(a, c, "different seeds should shuffle differently");
    }

    // --- reveal / match ---

    #[test]
    fn test_start_game_sets_playing_state() {
        let cx = ctx();
        let mut g = Game::new();
        assert_eq!(g.state, GameState::Start);
        g.start_game(&cx);
        assert_eq!(g.state, GameState::Playing);
        assert_eq!(g.pairs_found, 0);
        assert_eq!(g.elapsed, 0.0);
        assert!(!g.timer_running);
        assert_eq!(g.selection, SelectionPhase::Idle);
    }

    #[test]
    fn test_reveal_flips_hidden_card_and_starts_timer() {
        let cx = ctx();
        let mut g = game_with(8, 42, &cx);
        // Pick a known card.
        let (c, r) = (0, 0);
        assert!(g.board.get(c, r).unwrap().is_hidden());
        assert!(!g.timer_running);

        g.reveal(c, r, &cx);

        assert!(g.board.get(c, r).unwrap().is_flipped());
        assert!(g.timer_running);
        assert_eq!(g.selection, SelectionPhase::FirstPick { col: c, row: r });
    }

    #[test]
    fn test_reveal_same_card_twice_is_ignored() {
        let cx = ctx();
        let mut g = game_with(8, 42, &cx);
        g.reveal(0, 0, &cx);
        let selection_before = g.selection.clone();
        // Second click on the same card: should be a no-op because it's
        // already Flipped (is_clickable() == false).
        g.reveal(0, 0, &cx);
        assert_eq!(g.selection, selection_before);
    }

    #[test]
    fn test_match_marks_both_matched() {
        let cx = ctx();
        let mut g = game_with(8, 42, &cx);
        // Find two cards with the same symbol.
        let mut pair: Option<((usize, usize), (usize, usize))> = None;
        'outer: for r1 in 0..g.board.height() {
            for c1 in 0..g.board.width() {
                let s1 = g.board.get(c1, r1).unwrap().symbol;
                for r2 in 0..g.board.height() {
                    for c2 in 0..g.board.width() {
                        if (c1, r1) == (c2, r2) {
                            continue;
                        }
                        let s2 = g.board.get(c2, r2).unwrap().symbol;
                        if s1 == s2 {
                            pair = Some(((c1, r1), (c2, r2)));
                            break 'outer;
                        }
                    }
                }
            }
        }
        let ((c1, r1), (c2, r2)) = pair.expect("deck must have matching pairs");

        g.reveal(c1, r1, &cx);
        g.reveal(c2, r2, &cx);

        assert!(g.board.get(c1, r1).unwrap().is_matched());
        assert!(g.board.get(c2, r2).unwrap().is_matched());
        assert_eq!(g.pairs_found, 1);
        assert_eq!(g.selection, SelectionPhase::Idle);
    }

    #[test]
    fn test_no_match_starts_resolving_phase() {
        let cx = ctx();
        let mut g = game_with(8, 42, &cx);
        // Find two cards with different symbols.
        let mut pair: Option<((usize, usize), (usize, usize))> = None;
        'outer: for r1 in 0..g.board.height() {
            for c1 in 0..g.board.width() {
                let s1 = g.board.get(c1, r1).unwrap().symbol;
                for r2 in 0..g.board.height() {
                    for c2 in 0..g.board.width() {
                        if (c1, r1) == (c2, r2) {
                            continue;
                        }
                        let s2 = g.board.get(c2, r2).unwrap().symbol;
                        if s1 != s2 {
                            pair = Some(((c1, r1), (c2, r2)));
                            break 'outer;
                        }
                    }
                }
            }
        }
        let ((c1, r1), (c2, r2)) = pair.expect("deck must have different pairs");

        g.reveal(c1, r1, &cx);
        g.reveal(c2, r2, &cx);

        assert!(matches!(g.selection, SelectionPhase::Resolving { .. }));
        // Both cards are Flipped (not Matched).
        assert!(g.board.get(c1, r1).unwrap().is_flipped());
        assert!(g.board.get(c2, r2).unwrap().is_flipped());
    }

    #[test]
    fn test_resolving_blocks_clicks() {
        let cx = ctx();
        let mut g = game_with(8, 42, &cx);
        // Pick two different cards to trigger Resolving.
        let mut pair: Option<((usize, usize), (usize, usize))> = None;
        'outer: for r1 in 0..g.board.height() {
            for c1 in 0..g.board.width() {
                let s1 = g.board.get(c1, r1).unwrap().symbol;
                for r2 in 0..g.board.height() {
                    for c2 in 0..g.board.width() {
                        if (c1, r1) == (c2, r2) {
                            continue;
                        }
                        let s2 = g.board.get(c2, r2).unwrap().symbol;
                        if s1 != s2 {
                            pair = Some(((c1, r1), (c2, r2)));
                            break 'outer;
                        }
                    }
                }
            }
        }
        let ((c1, r1), (c2, r2)) = pair.unwrap();
        g.reveal(c1, r1, &cx);
        g.reveal(c2, r2, &cx);

        // Find a third card that's still hidden.
        let mut third: Option<(usize, usize)> = None;
        for r in 0..g.board.height() {
            for c in 0..g.board.width() {
                if (c, r) != (c1, r1) && (c, r) != (c2, r2) {
                    third = Some((c, r));
                    break;
                }
            }
            if third.is_some() {
                break;
            }
        }
        let (c3, r3) = third.unwrap();

        let selection_before = g.selection.clone();
        g.reveal(c3, r3, &cx);
        assert_eq!(
            g.selection, selection_before,
            "click during Resolving is ignored"
        );
        assert!(
            g.board.get(c3, r3).unwrap().is_hidden(),
            "third card not flipped"
        );
    }

    #[test]
    fn test_resolving_ends_after_delay() {
        let cx = ctx();
        let mut g = game_with(8, 42, &cx);
        // Two different cards.
        let mut pair: Option<((usize, usize), (usize, usize))> = None;
        'outer: for r1 in 0..g.board.height() {
            for c1 in 0..g.board.width() {
                let s1 = g.board.get(c1, r1).unwrap().symbol;
                for r2 in 0..g.board.height() {
                    for c2 in 0..g.board.width() {
                        if (c1, r1) == (c2, r2) {
                            continue;
                        }
                        let s2 = g.board.get(c2, r2).unwrap().symbol;
                        if s1 != s2 {
                            pair = Some(((c1, r1), (c2, r2)));
                            break 'outer;
                        }
                    }
                }
            }
        }
        let ((c1, r1), (c2, r2)) = pair.unwrap();
        g.reveal(c1, r1, &cx);
        g.reveal(c2, r2, &cx);
        assert!(matches!(g.selection, SelectionPhase::Resolving { .. }));

        // Tick past the delay.
        g.tick(cx.no_match_delay + 0.01);

        assert_eq!(g.selection, SelectionPhase::Idle);
        assert!(g.board.get(c1, r1).unwrap().is_hidden());
        assert!(g.board.get(c2, r2).unwrap().is_hidden());
    }

    #[test]
    fn test_timer_advances_when_running() {
        let cx = ctx();
        let mut g = game_with(8, 42, &cx);
        assert!(!g.timer_running);
        g.tick(1.0);
        assert_eq!(g.elapsed, 0.0, "timer not running yet");

        g.reveal(0, 0, &cx);
        assert!(g.timer_running);
        g.tick(1.0);
        assert!((g.elapsed - 1.0).abs() < 1e-5);
    }

    // --- win ---

    #[test]
    fn test_win_when_all_pairs_matched() {
        let cx = ctx();
        // Use a tiny 2x2 board (2 pairs) for a fast full playthrough.
        let mut g = Game::new();
        g.difficulties = vec![DifficultyData {
            name: "Tiny".into(),
            cols: 2,
            rows: 2,
            pairs: 2,
        }];
        g.selected_difficulty = 0;
        g.rng = Rng::new(42);
        g.start_game(&cx);

        // Find the two pairs and match them.
        for _ in 0..2 {
            // Find two hidden cards with the same symbol.
            let mut pair: Option<((usize, usize), (usize, usize))> = None;
            'outer: for r1 in 0..g.board.height() {
                for c1 in 0..g.board.width() {
                    if !g.board.get(c1, r1).unwrap().is_hidden() {
                        continue;
                    }
                    let s1 = g.board.get(c1, r1).unwrap().symbol;
                    for r2 in 0..g.board.height() {
                        for c2 in 0..g.board.width() {
                            if (c1, r1) == (c2, r2) {
                                continue;
                            }
                            if !g.board.get(c2, r2).unwrap().is_hidden() {
                                continue;
                            }
                            let s2 = g.board.get(c2, r2).unwrap().symbol;
                            if s1 == s2 {
                                pair = Some(((c1, r1), (c2, r2)));
                                break 'outer;
                            }
                        }
                    }
                }
            }
            let ((c1, r1), (c2, r2)) = pair.expect("must find a pair");
            g.reveal(c1, r1, &cx);
            g.reveal(c2, r2, &cx);
        }

        assert_eq!(g.pairs_found, 2);
        assert_eq!(g.state, GameState::Win);
        assert!(!g.timer_running);
    }

    // --- resolving render helper ---

    #[test]
    fn test_is_resolving_at() {
        let cx = ctx();
        let mut g = game_with(8, 42, &cx);
        // Initially nothing is resolving.
        assert!(!g.is_resolving_at(0, 0));

        // Find two different cards.
        let mut pair: Option<((usize, usize), (usize, usize))> = None;
        'outer: for r1 in 0..g.board.height() {
            for c1 in 0..g.board.width() {
                let s1 = g.board.get(c1, r1).unwrap().symbol;
                for r2 in 0..g.board.height() {
                    for c2 in 0..g.board.width() {
                        if (c1, r1) == (c2, r2) {
                            continue;
                        }
                        let s2 = g.board.get(c2, r2).unwrap().symbol;
                        if s1 != s2 {
                            pair = Some(((c1, r1), (c2, r2)));
                            break 'outer;
                        }
                    }
                }
            }
        }
        let ((c1, r1), (c2, r2)) = pair.unwrap();
        g.reveal(c1, r1, &cx);
        g.reveal(c2, r2, &cx);

        // Both cards are resolving.
        assert!(g.is_resolving_at(c1, r1));
        assert!(g.is_resolving_at(c2, r2));
        // A third card is not.
        for r in 0..g.board.height() {
            for c in 0..g.board.width() {
                if (c, r) != (c1, r1) && (c, r) != (c2, r2) {
                    assert!(!g.is_resolving_at(c, r));
                }
            }
        }
    }
}
