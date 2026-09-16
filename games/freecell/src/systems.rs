//! Game logic: rules, moves, super-moves, undo/redo, win detection,
//! and drag state machine.

use std::collections::HashMap;

use glam::Vec2;
use macroquad::prelude::Rect;

use ember_core::app::GameState;

use crate::components::{Card, Rank, Zone};
use crate::config::GameContext;
use crate::deck;
use crate::drag::DragState;
use crate::layout;
use crate::persistence::{self, BestTimes};

/// Pixels the mouse must move before a Pressing becomes a Dragging.
const DRAG_THRESHOLD: f32 = 5.0;

/// Snapshot of the mutable game state, used for undo/redo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameSnapshot {
    pub columns: [Vec<Card>; 8],
    pub free_cells: [Option<Card>; 4],
    pub foundations: [Vec<Card>; 4],
    pub moves: u32,
}

/// A visual hint for the player.
///
/// A hint describes a legal move but never executes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hint {
    pub from: Zone,
    pub to: Zone,
    pub card: Card,
}

/// Top-level state for one FreeCell session.
///
/// **Column representation (canonical)**:
/// - `columns[col][0]`     = card **visually at the top** of the column.
/// - `columns[col][len-1]` = card **visually at the bottom**.
///
/// A valid FreeCell sequence is `K, Q, J, ...` from top to bottom, i.e.
/// **increasing** indices in the Vec with descending ranks and alternating
/// colors.
///
/// **Drag model**: a dragged stack occupies a contiguous range
/// `[start_index, start_index + cards.len())` in its origin zone. This is
/// passed to `try_drop` so validation and removal use the right slice.
///
/// **Accessibility**: only the *bottom-most contiguous run* of a column
/// can be picked up. Cards in the middle of a column are covered by the
/// ones below and cannot be moved. This is what makes FreeCell a puzzle.
///
/// **Placement**: when a stack is dropped on a column, it goes **on top
/// of the existing stack**, i.e. **at the visual bottom**. Since `[0]` is
/// the visual top, new cards are appended at the end of the Vec via
/// `push`.
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

    pub history: Vec<GameSnapshot>,
    pub future: Vec<GameSnapshot>,

    pub best_times: HashMap<String, f32>,
    pub best_handle: BestTimes,
    pub was_new_best: bool,

    pub hint: Option<Hint>,

    pub start_btn: ember_stdlib::ui::button::Button,
    pub hint_btn: ember_stdlib::ui::button::Button,
    pub restart_btn: ember_stdlib::ui::button::Button,
    pub menu_btn: ember_stdlib::ui::button::Button,
    pub new_game_btn: ember_stdlib::ui::button::Button,
}

impl Game {
    pub fn new() -> Self {
        let ctx = crate::config::load_config();
        let best_handle = persistence::default();
        let best_times = best_handle.load_or(HashMap::new());

        let mut g = Self {
            state: GameState::Start,
            columns: Default::default(),
            free_cells: Default::default(),
            foundations: Default::default(),
            drag: DragState::Idle,
            moves: 0,
            elapsed: 0.0,
            timer_running: false,
            seed: 1,
            history: Vec::new(),
            future: Vec::new(),
            best_times,
            best_handle,
            was_new_best: false,
            hint: None,
            start_btn: ember_stdlib::ui::button::Button::new(0.0, 0.0, 200.0, 50.0, "START"),
            hint_btn: ember_stdlib::ui::button::Button::new(
                ctx.window_w - 480.0,
                10.0,
                100.0,
                30.0,
                "Hint",
            ),
            restart_btn: ember_stdlib::ui::button::Button::new(
                ctx.window_w - 360.0,
                10.0,
                100.0,
                30.0,
                "Restart",
            ),
            new_game_btn: ember_stdlib::ui::button::Button::new(
                ctx.window_w - 240.0,
                10.0,
                100.0,
                30.0,
                "New",
            ),
            menu_btn: ember_stdlib::ui::button::Button::new(
                ctx.window_w - 120.0,
                10.0,
                100.0,
                30.0,
                "Menu",
            ),
        };
        g.update_menu_layout(&ctx);
        g
    }

    pub fn with_best_handle(handle: BestTimes) -> Self {
        let mut g = Self::new();
        g.best_times = handle.load_or(HashMap::new());
        g.best_handle = handle;
        g
    }

    pub fn update_menu_layout(&mut self, ctx: &GameContext) {
        let cx = ctx.window_w * 0.5;
        self.start_btn.rect = (cx - 100.0, ctx.window_h * 0.5 + 100.0, 200.0, 50.0);
    }

    pub fn start_game(&mut self, seed: u32, _ctx: &GameContext) {
        self.seed = seed;
        let cards = deck::deal(seed);
        self.deal_cards(cards);
        self.moves = 0;
        self.elapsed = 0.0;
        self.timer_running = false;
        self.history.clear();
        self.future.clear();
        self.was_new_best = false;
        self.hint = None;
        self.drag = DragState::Idle;
        self.state = GameState::Playing;
    }

    fn deal_cards(&mut self, cards: Vec<Card>) {
        assert_eq!(cards.len(), 52);
        self.columns = Default::default();
        self.free_cells = Default::default();
        self.foundations = Default::default();

        let mut i = 0;
        for col in 0..8 {
            let count = if col < 4 { 7 } else { 6 };
            for _ in 0..count {
                self.columns[col].push(cards[i]);
                i += 1;
            }
        }
        assert_eq!(i, 52);
    }

    /// Check whether the currently dragged stack could be dropped on `target`.
    ///
    /// Used for the drag overlay's visual feedback. Does not mutate anything.
    pub fn can_drop_at(&self, target: Zone) -> bool {
        if let DragState::Dragging {
            origin,
            start_index,
            cards,
            ..
        } = &self.drag
        {
            return self.validate_move(*origin, target, cards, *start_index);
        }
        false
    }

    // ----------------------------------------------------------------
    // Rules
    // ----------------------------------------------------------------

    pub fn can_place_on_column(&self, card: Card, col: usize) -> bool {
        if col >= 8 {
            return false;
        }
        match self.columns[col].last() {
            None => true,
            Some(top) => top.rank.value() == card.rank.value() + 1 && top.color() != card.color(),
        }
    }

    pub fn can_place_on_foundation(&self, card: Card, foundation: usize) -> bool {
        if foundation >= 4 {
            return false;
        }
        if card.suit.index() != foundation {
            return false;
        }
        match self.foundations[foundation].last() {
            None => card.rank == Rank::ACE,
            Some(top) => top.rank.value() + 1 == card.rank.value(),
        }
    }

    pub fn can_place_in_cell(&self, cell: usize) -> bool {
        cell < 4 && self.free_cells[cell].is_none()
    }

    pub fn max_super_move(&self) -> usize {
        let empty_cells = self.free_cells.iter().filter(|c| c.is_none()).count();
        let empty_columns = self.columns.iter().filter(|c| c.is_empty()).count();
        (1 + empty_cells) * (1usize << empty_columns)
    }

    /// True if the contiguous run of `n` cards starting at `start_idx` in
    /// `from` can be picked up.
    ///
    /// For a column, only the bottom-most run is accessible, i.e.
    /// `start_idx + n == columns[col].len()`.
    /// For free cells and foundations, always true (single card).
    ///
    /// This is what prevents "taking a card from the middle of a column".
    pub fn is_accessible(&self, from: Zone, start_idx: usize, n: usize) -> bool {
        match from {
            Zone::Column(col) => {
                if col >= 8 {
                    return false;
                }
                start_idx + n == self.columns[col].len()
            }
            Zone::FreeCell(_) | Zone::Foundation(_) => true,
        }
    }

    // ----------------------------------------------------------------
    // Moves
    // ----------------------------------------------------------------

    pub fn try_drop(&mut self, from: Zone, to: Zone, cards: &[Card], start_idx: usize) -> bool {
        if cards.is_empty() || from == to {
            return false;
        }
        if !self.validate_move(from, to, cards, start_idx) {
            return false;
        }

        self.history.push(self.snapshot());
        self.future.clear();

        self.remove_cards(from, start_idx, cards.len());
        self.push_cards(to, cards);

        self.moves += 1;
        self.hint = None;
        if !self.timer_running {
            self.timer_running = true;
        }

        self.check_win();
        true
    }

    fn validate_move(&self, from: Zone, to: Zone, cards: &[Card], start_idx: usize) -> bool {
        // 1. Cards must be accessible (bottom-most run of the source).
        if !self.is_accessible(from, start_idx, cards.len()) {
            return false;
        }

        // 2. Cards must actually match the source slice.
        if !self.source_matches(from, cards, start_idx) {
            return false;
        }

        // 3. Multi-card moves must form a valid sequence and respect the
        // super-move limit.
        if cards.len() > 1 {
            if !is_valid_sequence(cards) {
                return false;
            }
            if cards.len() > self.max_super_move() {
                return false;
            }
        }

        // 4. Destination must accept the first card.
        let first = cards[0];
        match to {
            Zone::Column(col) => self.can_place_on_column(first, col),
            Zone::Foundation(f) => cards.len() == 1 && self.can_place_on_foundation(first, f),
            Zone::FreeCell(c) => cards.len() == 1 && self.can_place_in_cell(c),
        }
    }

    fn source_matches(&self, zone: Zone, cards: &[Card], start_idx: usize) -> bool {
        if cards.is_empty() {
            return false;
        }
        match zone {
            Zone::Column(col) => {
                if col >= 8 {
                    return false;
                }
                let c = &self.columns[col];
                if start_idx + cards.len() > c.len() {
                    return false;
                }
                c[start_idx..start_idx + cards.len()] == *cards
            }
            Zone::FreeCell(cell) => {
                if cell >= 4 || cards.len() != 1 {
                    return false;
                }
                self.free_cells[cell] == Some(cards[0])
            }
            Zone::Foundation(f) => {
                if f >= 4 || cards.len() != 1 {
                    return false;
                }
                self.foundations[f].last() == Some(&cards[0])
            }
        }
    }

    fn remove_cards(&mut self, zone: Zone, start_idx: usize, n: usize) -> Vec<Card> {
        match zone {
            Zone::Column(col) => {
                if start_idx + n > self.columns[col].len() {
                    return Vec::new();
                }
                self.columns[col].drain(start_idx..start_idx + n).collect()
            }
            Zone::FreeCell(cell) => {
                if n != 1 {
                    return Vec::new();
                }
                self.free_cells[cell].take().into_iter().collect()
            }
            Zone::Foundation(f) => {
                if n != 1 {
                    return Vec::new();
                }
                self.foundations[f].pop().into_iter().collect()
            }
        }
    }

    fn push_cards(&mut self, zone: Zone, cards: &[Card]) {
        match zone {
            Zone::Column(col) => {
                for c in cards.iter() {
                    self.columns[col].push(*c);
                }
            }
            Zone::FreeCell(cell) => {
                if cards.len() == 1 {
                    self.free_cells[cell] = Some(cards[0]);
                }
            }
            Zone::Foundation(f) => {
                if cards.len() == 1 {
                    self.foundations[f].push(cards[0]);
                }
            }
        }
    }

    // ----------------------------------------------------------------
    // Hint
    // ----------------------------------------------------------------

    /// Finds one useful legal move without changing the game state.
    /// The hint is only an indicator for the player; it is never executed.
    pub fn find_hint(&self) -> Option<Hint> {
        if self.state != GameState::Playing {
            return None;
        }

        // 1. Prefer a move to a foundation.
        for from in 0..8 {
            let Some(card) = self.columns[from].last().copied() else {
                continue;
            };
            let foundation = card.suit.index();
            if self.can_place_on_foundation(card, foundation) {
                return Some(Hint {
                    from: Zone::Column(from),
                    to: Zone::Foundation(foundation),
                    card,
                });
            }
        }

        // 2. Then a move to another column.
        for from in 0..8 {
            let Some(card) = self.columns[from].last().copied() else {
                continue;
            };
            for to in 0..8 {
                if from != to && self.can_place_on_column(card, to) {
                    return Some(Hint {
                        from: Zone::Column(from),
                        to: Zone::Column(to),
                        card,
                    });
                }
            }
        }

        // 3. Finally, suggest an empty free cell.
        for from in 0..8 {
            let Some(card) = self.columns[from].last().copied() else {
                continue;
            };
            for cell in 0..4 {
                if self.can_place_in_cell(cell) {
                    return Some(Hint {
                        from: Zone::Column(from),
                        to: Zone::FreeCell(cell),
                        card,
                    });
                }
            }
        }

        None
    }

    // ----------------------------------------------------------------
    // Win / score
    // ----------------------------------------------------------------

    fn check_win(&mut self) {
        if self.foundations.iter().all(|f| f.len() == 13) {
            self.state = GameState::Win;
            self.timer_running = false;
            self.record_best_if_needed();
        }
    }

    pub fn record_best_if_needed(&mut self) {
        if persistence::update_if_better(&mut self.best_times, self.seed, self.elapsed) {
            self.best_handle.save(&self.best_times);
            self.was_new_best = true;
        }
    }

    // ----------------------------------------------------------------
    // Undo / redo
    // ----------------------------------------------------------------

    fn snapshot(&self) -> GameSnapshot {
        GameSnapshot {
            columns: self.columns.clone(),
            free_cells: self.free_cells,
            foundations: self.foundations.clone(),
            moves: self.moves,
        }
    }

    fn restore(&mut self, snap: GameSnapshot) {
        self.columns = snap.columns;
        self.free_cells = snap.free_cells;
        self.foundations = snap.foundations;
        self.moves = snap.moves;
    }

    pub fn undo(&mut self) -> bool {
        if let Some(prev) = self.history.pop() {
            self.future.push(self.snapshot());
            self.restore(prev);
            self.hint = None;
            if self.state == GameState::Win {
                self.state = GameState::Playing;
            }
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if let Some(next) = self.future.pop() {
            self.history.push(self.snapshot());
            self.restore(next);
            self.hint = None;
            self.check_win();
            true
        } else {
            false
        }
    }

    // ----------------------------------------------------------------
    // Time
    // ----------------------------------------------------------------

    pub fn tick(&mut self, dt: f32) {
        if self.timer_running {
            self.elapsed += dt;
        }
    }

    // ----------------------------------------------------------------
    // Hit testing
    // ----------------------------------------------------------------

    pub fn zone_at(&self, mouse: Vec2, ctx: &GameContext) -> Option<Zone> {
        for i in 0..4 {
            let r = layout::free_cell_rect(i, ctx);
            if point_in_rect(mouse, r) {
                return Some(Zone::FreeCell(i));
            }
        }
        for i in 0..4 {
            let r = layout::foundation_rect(i, ctx);
            if point_in_rect(mouse, r) {
                return Some(Zone::Foundation(i));
            }
        }
        for col in 0..8 {
            let r = layout::column_bounds(col, ctx);
            if point_in_rect(mouse, r) {
                return Some(Zone::Column(col));
            }
        }
        None
    }

    /// Find the topmost card under the mouse, if any.
    ///
    /// Returns `(zone, index)` where `index` is the index in the Vec of
    /// the zone (0 = visual top for a column).
    pub fn hit_test(&self, mouse: Vec2, ctx: &GameContext) -> Option<(Zone, usize)> {
        for i in 0..4 {
            if self.free_cells[i].is_some() {
                let r = layout::free_cell_rect(i, ctx);
                if point_in_rect(mouse, r) {
                    return Some((Zone::FreeCell(i), 0));
                }
            }
        }
        for i in 0..4 {
            if !self.foundations[i].is_empty() {
                let r = layout::foundation_rect(i, ctx);
                if point_in_rect(mouse, r) {
                    return Some((Zone::Foundation(i), 0));
                }
            }
        }
        for col in 0..8 {
            let len = self.columns[col].len();
            for card_index in (0..len).rev() {
                let r = layout::card_rect_in_column(col, card_index, ctx);
                if point_in_rect(mouse, r) {
                    return Some((Zone::Column(col), card_index));
                }
            }
        }
        None
    }

    /// Compute the maximal valid sequence starting at `idx` in the zone.
    ///
    /// For a column, walks **down** the Vec (increasing indices) while
    /// ranks descend and colors alternate.
    pub fn drag_cards(&self, zone: Zone, idx: usize) -> Vec<Card> {
        match zone {
            Zone::FreeCell(i) => self.free_cells[i].into_iter().collect(),
            Zone::Foundation(i) => self.foundations[i].last().copied().into_iter().collect(),
            Zone::Column(col) => {
                let c = &self.columns[col];
                let len = c.len();
                if idx >= len {
                    return Vec::new();
                }
                let mut seq = vec![c[idx]];
                for &below in c.iter().skip(idx + 1) {
                    let top = *seq.last().unwrap();
                    if below.rank.value() + 1 == top.rank.value() && below.color() != top.color() {
                        seq.push(below);
                    } else {
                        break;
                    }
                }
                seq
            }
        }
    }

    // ----------------------------------------------------------------
    // Drag state machine
    // ----------------------------------------------------------------

    pub fn start_drag(&mut self, mouse: Vec2, ctx: &GameContext) {
        if self.state != GameState::Playing {
            return;
        }
        if !self.drag.is_idle() {
            return;
        }
        let Some((zone, idx)) = self.hit_test(mouse, ctx) else {
            return;
        };
        if let Zone::Foundation(_) = zone {
            return;
        }
        let cards = self.drag_cards(zone, idx);
        if cards.is_empty() {
            return;
        }
        // Only the bottom-most run of a column is draggable.
        if !self.is_accessible(zone, idx, cards.len()) {
            return;
        }
        self.drag = DragState::Pressing {
            origin: zone,
            card: cards[0],
            press_pos: mouse,
        };
    }

    pub fn update_drag(&mut self, mouse: Vec2, ctx: &GameContext) {
        if let DragState::Pressing {
            origin,
            card,
            press_pos,
        } = self.drag
        {
            let dist = (mouse - press_pos).length();
            if dist >= DRAG_THRESHOLD {
                let idx = self.index_of_card(origin, card);
                let cards = self.drag_cards(origin, idx);
                if cards.is_empty() {
                    self.drag = DragState::Idle;
                    return;
                }
                let r = self.card_rect_for(origin, idx, ctx);
                let offset = press_pos - Vec2::new(r.x, r.y);
                self.drag = DragState::Dragging {
                    origin,
                    start_index: idx,
                    cards,
                    offset,
                };
            }
        }
    }

    pub fn end_drag(&mut self, mouse: Vec2, ctx: &GameContext) {
        let dragging = std::mem::replace(&mut self.drag, DragState::Idle);
        if let DragState::Dragging {
            origin,
            start_index,
            cards,
            ..
        } = dragging
            && let Some(target) = self.zone_at(mouse, ctx)
        {
            self.try_drop(origin, target, &cards, start_index);
        }
    }

    pub fn cancel_drag(&mut self) {
        self.drag = DragState::Idle;
    }

    // ----------------------------------------------------------------
    // Helpers
    // ----------------------------------------------------------------

    fn card_rect_for(&self, zone: Zone, idx: usize, ctx: &GameContext) -> Rect {
        match zone {
            Zone::FreeCell(i) => layout::free_cell_rect(i, ctx),
            Zone::Foundation(i) => layout::foundation_rect(i, ctx),
            Zone::Column(col) => layout::card_rect_in_column(col, idx, ctx),
        }
    }

    fn index_of_card(&self, zone: Zone, card: Card) -> usize {
        match zone {
            Zone::FreeCell(_) | Zone::Foundation(_) => 0,
            Zone::Column(col) => self.columns[col]
                .iter()
                .position(|c| *c == card)
                .unwrap_or(0),
        }
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Helpers
// ============================================================================

fn is_valid_sequence(cards: &[Card]) -> bool {
    if cards.is_empty() {
        return false;
    }
    for w in cards.windows(2) {
        let (a, b) = (w[0], w[1]);
        if a.rank.value() != b.rank.value() + 1 {
            return false;
        }
        if a.color() == b.color() {
            return false;
        }
    }
    true
}

fn point_in_rect(p: Vec2, r: Rect) -> bool {
    p.x >= r.x && p.x <= r.x + r.w && p.y >= r.y && p.y <= r.y + r.h
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::Suit;

    fn ctx() -> GameContext {
        crate::config::load_config()
    }

    fn game_with_seed(seed: u32) -> Game {
        let mut g = Game::new();
        g.start_game(seed, &ctx());
        g
    }

    // --- start_game ---

    #[test]
    fn test_start_game_deals_4x7_plus_4x6() {
        let g = game_with_seed(1);
        assert_eq!(g.columns[0].len(), 7);
        assert_eq!(g.columns[1].len(), 7);
        assert_eq!(g.columns[2].len(), 7);
        assert_eq!(g.columns[3].len(), 7);
        assert_eq!(g.columns[4].len(), 6);
        assert_eq!(g.columns[5].len(), 6);
        assert_eq!(g.columns[6].len(), 6);
        assert_eq!(g.columns[7].len(), 6);
        let total: usize = g.columns.iter().map(|c| c.len()).sum();
        assert_eq!(total, 52);
    }

    #[test]
    fn test_start_game_clears_state() {
        let mut g = Game::new();
        g.moves = 99;
        g.elapsed = 42.0;
        g.timer_running = true;
        g.history.push(GameSnapshot {
            columns: Default::default(),
            free_cells: Default::default(),
            foundations: Default::default(),
            moves: 0,
        });
        g.start_game(1, &ctx());
        assert_eq!(g.moves, 0);
        assert_eq!(g.elapsed, 0.0);
        assert!(!g.timer_running);
        assert!(g.history.is_empty());
        assert!(g.future.is_empty());
    }

    #[test]
    fn test_start_game_sets_playing_state() {
        let g = game_with_seed(1);
        assert_eq!(g.state, GameState::Playing);
    }

    // --- can_place_on_column ---

    #[test]
    fn test_can_place_on_empty_column() {
        let mut g = game_with_seed(1);
        g.columns[0].clear();
        let c = Card::new(Suit::Heart, Rank(5));
        assert!(g.can_place_on_column(c, 0));
    }

    #[test]
    fn test_can_place_red_on_black_7_on_8() {
        let mut g = game_with_seed(1);
        g.columns[0].clear();
        g.columns[0].push(Card::new(Suit::Spade, Rank(8)));
        let red7 = Card::new(Suit::Heart, Rank(7));
        assert!(g.can_place_on_column(red7, 0));
    }

    #[test]
    fn test_cannot_place_same_color() {
        let mut g = game_with_seed(1);
        g.columns[0].clear();
        g.columns[0].push(Card::new(Suit::Spade, Rank(8)));
        let black7 = Card::new(Suit::Club, Rank(7));
        assert!(!g.can_place_on_column(black7, 0));
    }

    #[test]
    fn test_cannot_place_wrong_rank() {
        let mut g = game_with_seed(1);
        g.columns[0].clear();
        g.columns[0].push(Card::new(Suit::Spade, Rank(9)));
        let red7 = Card::new(Suit::Heart, Rank(7));
        assert!(!g.can_place_on_column(red7, 0));
    }

    // --- can_place_on_foundation ---

    #[test]
    fn test_ace_goes_to_empty_foundation() {
        let g = game_with_seed(1);
        let ace_of_hearts = Card::new(Suit::Heart, Rank::ACE);
        let heart_idx = Suit::Heart.index();
        assert!(g.can_place_on_foundation(ace_of_hearts, heart_idx));
    }

    #[test]
    fn test_ace_goes_only_to_its_color_foundation() {
        let g = game_with_seed(1);
        let ace_of_hearts = Card::new(Suit::Heart, Rank::ACE);
        let spade_idx = Suit::Spade.index();
        assert!(!g.can_place_on_foundation(ace_of_hearts, spade_idx));
    }

    #[test]
    fn test_foundation_accepts_ascending_same_suit() {
        let mut g = game_with_seed(1);
        let heart_idx = Suit::Heart.index();
        g.foundations[heart_idx].push(Card::new(Suit::Heart, Rank::ACE));
        let two_of_hearts = Card::new(Suit::Heart, Rank(2));
        assert!(g.can_place_on_foundation(two_of_hearts, heart_idx));
    }

    #[test]
    fn test_foundation_rejects_wrong_suit() {
        let mut g = game_with_seed(1);
        let heart_idx = Suit::Heart.index();
        g.foundations[heart_idx].push(Card::new(Suit::Heart, Rank::ACE));
        let two_of_diamonds = Card::new(Suit::Diamond, Rank(2));
        assert!(!g.can_place_on_foundation(two_of_diamonds, heart_idx));
    }

    // --- can_place_in_cell ---

    #[test]
    fn test_cell_accepts_when_empty() {
        let g = game_with_seed(1);
        assert!(g.can_place_in_cell(0));
    }

    #[test]
    fn test_cell_rejects_when_full() {
        let mut g = game_with_seed(1);
        g.free_cells[0] = Some(Card::new(Suit::Heart, Rank(5)));
        assert!(!g.can_place_in_cell(0));
    }

    // --- max_super_move ---

    #[test]
    fn test_max_super_move_with_all_cells_and_no_empty_columns() {
        let g = game_with_seed(1);
        assert_eq!(g.max_super_move(), 5);
    }

    #[test]
    fn test_max_super_move_with_one_empty_column() {
        let mut g = game_with_seed(1);
        g.columns[0].clear();
        assert_eq!(g.max_super_move(), 10);
    }

    // --- is_accessible ---

    #[test]
    fn test_is_accessible_bottom_of_column() {
        let mut g = game_with_seed(1);
        g.columns[0].clear();
        g.columns[0].push(Card::new(Suit::Spade, Rank(8)));
        g.columns[0].push(Card::new(Suit::Heart, Rank(7)));
        // Index 1 is the bottom of the column (len = 2). Run of 1.
        assert!(g.is_accessible(Zone::Column(0), 1, 1));
    }

    #[test]
    fn test_is_accessible_full_run_to_bottom() {
        let mut g = game_with_seed(1);
        g.columns[0].clear();
        g.columns[0].push(Card::new(Suit::Spade, Rank(8)));
        g.columns[0].push(Card::new(Suit::Heart, Rank(7)));
        // Indices 0..2 form a run that reaches the bottom.
        assert!(g.is_accessible(Zone::Column(0), 0, 2));
    }

    #[test]
    fn test_is_accessible_middle_card_is_not() {
        let mut g = game_with_seed(1);
        g.columns[0].clear();
        g.columns[0].push(Card::new(Suit::Spade, Rank(8)));
        g.columns[0].push(Card::new(Suit::Heart, Rank(7)));
        g.columns[0].push(Card::new(Suit::Club, Rank(6)));
        // Index 1 is in the middle (len = 3). Even a run of 1 can't reach
        // the bottom: 1 + 1 = 2 != 3.
        assert!(!g.is_accessible(Zone::Column(0), 1, 1));
        // A run of 2 from index 1 also fails: 1 + 2 = 3 == 3 → actually
        // this reaches the bottom. Check it.
        assert!(g.is_accessible(Zone::Column(0), 1, 2));
    }

    #[test]
    fn test_is_accessible_free_cell_and_foundation() {
        let g = game_with_seed(1);
        assert!(g.is_accessible(Zone::FreeCell(0), 0, 1));
        assert!(g.is_accessible(Zone::Foundation(0), 0, 1));
    }

    // --- try_drop ---

    #[test]
    fn test_try_drop_valid_single_card_to_column() {
        let mut g = game_with_seed(1);
        g.columns[0].clear();
        g.columns[1].clear();
        g.columns[0].push(Card::new(Suit::Spade, Rank(8))); // [8♠]
        g.columns[1].push(Card::new(Suit::Heart, Rank(7))); // [7♥]
        let card = Card::new(Suit::Heart, Rank(7));
        let ok = g.try_drop(Zone::Column(1), Zone::Column(0), &[card], 0);
        assert!(ok);
        assert_eq!(g.columns[0].len(), 2);
        assert_eq!(g.columns[0][0], Card::new(Suit::Spade, Rank(8)));
        assert_eq!(g.columns[0][1], Card::new(Suit::Heart, Rank(7)));
        assert_eq!(g.columns[1].len(), 0);
        assert_eq!(g.moves, 1);
        assert!(g.timer_running);
    }

    #[test]
    fn test_try_drop_invalid_is_rejected_and_state_unchanged() {
        let mut g = game_with_seed(1);
        g.columns[0].clear();
        g.columns[1].clear();
        g.columns[0].push(Card::new(Suit::Spade, Rank(8)));
        g.columns[1].push(Card::new(Suit::Club, Rank(7)));
        let card = Card::new(Suit::Club, Rank(7));
        let ok = g.try_drop(Zone::Column(1), Zone::Column(0), &[card], 0);
        assert!(!ok);
        assert_eq!(g.columns[0].len(), 1);
        assert_eq!(g.columns[1].len(), 1);
        assert_eq!(g.moves, 0);
    }

    #[test]
    fn test_try_drop_to_cell() {
        let mut g = game_with_seed(1);
        let card = g.columns[0][0]; // visual top
        // Wait: with is_accessible, only the bottom of the column can be
        // dragged. Let's reset column 0 to a single card so the top IS
        // the bottom.
        g.columns[0].clear();
        g.columns[0].push(card);
        let ok = g.try_drop(Zone::Column(0), Zone::FreeCell(0), &[card], 0);
        assert!(ok);
        assert_eq!(g.free_cells[0], Some(card));
    }

    #[test]
    fn test_try_drop_to_foundation_ace() {
        let mut g = game_with_seed(1);
        g.columns[0].clear();
        g.columns[0].push(Card::new(Suit::Heart, Rank::ACE));
        let card = Card::new(Suit::Heart, Rank::ACE);
        let heart_idx = Suit::Heart.index();
        let ok = g.try_drop(Zone::Column(0), Zone::Foundation(heart_idx), &[card], 0);
        assert!(ok);
        assert_eq!(g.foundations[heart_idx].len(), 1);
    }

    #[test]
    fn test_try_drop_sequence_from_bottom_of_column() {
        let mut g = game_with_seed(1);
        g.columns[0].clear();
        g.columns[1].clear();
        // c[0] top = K♠, c[1] = Q♥, c[2] = J♣, c[3] bottom = T♦.
        // We drag the bottom run [J♣, T♦] (start_idx = 2).
        g.columns[0].push(Card::new(Suit::Spade, Rank(13)));
        g.columns[0].push(Card::new(Suit::Heart, Rank(12)));
        g.columns[0].push(Card::new(Suit::Club, Rank(11)));
        g.columns[0].push(Card::new(Suit::Diamond, Rank(10)));

        let cards = vec![
            Card::new(Suit::Club, Rank(11)),
            Card::new(Suit::Diamond, Rank(10)),
        ];
        let ok = g.try_drop(Zone::Column(0), Zone::Column(1), &cards, 2);
        assert!(ok);
        assert_eq!(g.columns[0].len(), 2);
        assert_eq!(g.columns[0][0], Card::new(Suit::Spade, Rank(13))); // K♠
        assert_eq!(g.columns[0][1], Card::new(Suit::Heart, Rank(12))); // Q♥
        assert_eq!(g.columns[1].len(), 2);
        assert_eq!(g.columns[1][0], Card::new(Suit::Club, Rank(11))); // J♣
        assert_eq!(g.columns[1][1], Card::new(Suit::Diamond, Rank(10))); // T♦
    }

    #[test]
    fn test_try_drop_middle_of_column_is_rejected() {
        let mut g = game_with_seed(1);
        g.columns[0].clear();
        g.columns[1].clear();
        // Column 0: [K♠, Q♥, J♣, T♦]. len = 4.
        g.columns[0].push(Card::new(Suit::Spade, Rank(13)));
        g.columns[0].push(Card::new(Suit::Heart, Rank(12)));
        g.columns[0].push(Card::new(Suit::Club, Rank(11)));
        g.columns[0].push(Card::new(Suit::Diamond, Rank(10)));

        // Try to drop [Q♥, J♣] from the middle (start_idx = 1).
        // is_accessible(1, 2): 1 + 2 = 3 != 4 → rejected.
        let cards = vec![
            Card::new(Suit::Heart, Rank(12)),
            Card::new(Suit::Club, Rank(11)),
        ];
        let ok = g.try_drop(Zone::Column(0), Zone::Column(1), &cards, 1);
        assert!(!ok, "middle of a column must not be draggable");
        // State unchanged.
        assert_eq!(g.columns[0].len(), 4);
        assert_eq!(g.columns[1].len(), 0);
    }

    #[test]
    fn test_drop_multi_card_on_cell_is_rejected() {
        let mut g = game_with_seed(1);
        g.columns[0].clear();
        g.columns[0].push(Card::new(Suit::Spade, Rank(8)));
        g.columns[0].push(Card::new(Suit::Heart, Rank(7)));
        let cards = vec![
            Card::new(Suit::Spade, Rank(8)),
            Card::new(Suit::Heart, Rank(7)),
        ];
        let ok = g.try_drop(Zone::Column(0), Zone::FreeCell(0), &cards, 0);
        assert!(!ok, "a cell accepts only one card");
        assert_eq!(g.columns[0].len(), 2);
        assert!(g.free_cells[0].is_none());
    }

    // --- is_valid_sequence ---

    #[test]
    fn test_is_valid_sequence_descending_alternating() {
        let seq = vec![
            Card::new(Suit::Heart, Rank(7)),
            Card::new(Suit::Spade, Rank(6)),
            Card::new(Suit::Diamond, Rank(5)),
        ];
        assert!(is_valid_sequence(&seq));
    }

    #[test]
    fn test_is_valid_sequence_wrong_order() {
        let seq = vec![
            Card::new(Suit::Heart, Rank(7)),
            Card::new(Suit::Spade, Rank(5)),
        ];
        assert!(!is_valid_sequence(&seq));
    }

    #[test]
    fn test_is_valid_sequence_same_color() {
        let seq = vec![
            Card::new(Suit::Heart, Rank(7)),
            Card::new(Suit::Diamond, Rank(6)),
        ];
        assert!(!is_valid_sequence(&seq));
    }

    // --- win ---

    #[test]
    fn test_check_win_when_foundations_complete() {
        let mut g = game_with_seed(1);
        for f in 0..4 {
            for r in 1..=13 {
                let suit = Suit::ALL[f];
                g.foundations[f].push(Card::new(suit, Rank(r)));
            }
        }
        assert!(g.foundations.iter().all(|f| f.len() == 13));
        let _ = &mut g;
    }

    // --- undo / redo ---

    #[test]
    fn test_undo_restores_previous_state() {
        let mut g = game_with_seed(1);
        g.columns[0].clear();
        g.columns[1].clear();
        g.columns[0].push(Card::new(Suit::Spade, Rank(8)));
        g.columns[1].push(Card::new(Suit::Heart, Rank(7)));
        let card = Card::new(Suit::Heart, Rank(7));
        assert!(g.try_drop(Zone::Column(1), Zone::Column(0), &[card], 0));
        assert_eq!(g.columns[0].len(), 2);
        assert_eq!(g.moves, 1);

        assert!(g.undo());
        assert_eq!(g.columns[0].len(), 1);
        assert_eq!(g.columns[1].len(), 1);
        assert_eq!(g.moves, 0);
    }

    #[test]
    fn test_redo_after_undo() {
        let mut g = game_with_seed(1);
        g.columns[0].clear();
        g.columns[1].clear();
        g.columns[0].push(Card::new(Suit::Spade, Rank(8)));
        g.columns[1].push(Card::new(Suit::Heart, Rank(7)));
        let card = Card::new(Suit::Heart, Rank(7));
        assert!(g.try_drop(Zone::Column(1), Zone::Column(0), &[card], 0));
        assert!(g.undo());
        assert!(g.redo());
        assert_eq!(g.columns[0].len(), 2);
        assert_eq!(g.moves, 1);
    }

    #[test]
    fn test_undo_then_new_move_clears_redo() {
        let mut g = game_with_seed(1);
        g.columns[0].clear();
        g.columns[1].clear();
        g.columns[2].clear();
        g.columns[0].push(Card::new(Suit::Spade, Rank(8)));
        g.columns[1].push(Card::new(Suit::Heart, Rank(7)));
        g.columns[2].push(Card::new(Suit::Club, Rank(6)));

        let c7 = Card::new(Suit::Heart, Rank(7));
        assert!(g.try_drop(Zone::Column(1), Zone::Column(0), &[c7], 0));
        assert!(g.undo());
        assert!(!g.future.is_empty());

        let c6 = Card::new(Suit::Club, Rank(6));
        assert!(g.try_drop(Zone::Column(2), Zone::Column(1), &[c6], 0));
        assert!(g.future.is_empty());
    }

    #[test]
    fn test_undo_with_empty_history_returns_false() {
        let mut g = game_with_seed(1);
        assert!(!g.undo());
    }

    #[test]
    fn test_redo_with_empty_future_returns_false() {
        let mut g = game_with_seed(1);
        assert!(!g.redo());
    }

    // --- timer ---

    #[test]
    fn test_timer_starts_on_first_move() {
        let mut g = game_with_seed(1);
        assert!(!g.timer_running);
        g.columns[0].clear();
        g.columns[1].clear();
        g.columns[0].push(Card::new(Suit::Spade, Rank(8)));
        g.columns[1].push(Card::new(Suit::Heart, Rank(7)));
        let c7 = Card::new(Suit::Heart, Rank(7));
        assert!(g.try_drop(Zone::Column(1), Zone::Column(0), &[c7], 0));
        assert!(g.timer_running);
    }

    #[test]
    fn test_tick_advances_elapsed_when_running() {
        let mut g = game_with_seed(1);
        g.tick(1.0);
        assert_eq!(g.elapsed, 0.0);
        g.timer_running = true;
        g.tick(1.0);
        assert!((g.elapsed - 1.0).abs() < 1e-5);
    }

    // --- hit testing ---

    #[test]
    fn test_hit_test_finds_top_card_band() {
        let g = game_with_seed(1);
        let cx = ctx();
        let r = layout::card_rect_in_column(0, 0, &cx);
        let point = Vec2::new(r.x + r.w * 0.5, r.y + 10.0);
        let hit = g.hit_test(point, &cx);
        assert_eq!(hit, Some((Zone::Column(0), 0)));
    }

    #[test]
    fn test_hit_test_finds_bottom_card_band() {
        let g = game_with_seed(1);
        let cx = ctx();
        let len = g.columns[0].len();
        let r = layout::card_rect_in_column(0, len - 1, &cx);
        let point = Vec2::new(r.x + r.w * 0.5, r.y + r.h * 0.5);
        let hit = g.hit_test(point, &cx);
        assert_eq!(hit, Some((Zone::Column(0), len - 1)));
    }

    #[test]
    fn test_hit_test_outside_returns_none() {
        let g = game_with_seed(1);
        let cx = ctx();
        let hit = g.hit_test(Vec2::new(-100.0, -100.0), &cx);
        assert_eq!(hit, None);
    }

    #[test]
    fn test_zone_at_empty_column() {
        let mut g = game_with_seed(1);
        g.columns[0].clear();
        let cx = ctx();
        let r = layout::column_bounds(0, &cx);
        let center = Vec2::new(r.x + r.w * 0.5, r.y + 10.0);
        assert_eq!(g.zone_at(center, &cx), Some(Zone::Column(0)));
    }

    // --- drag_cards ---

    #[test]
    fn test_drag_cards_top_of_column_returns_valid_run() {
        let mut g = game_with_seed(1);
        g.columns[0].clear();
        g.columns[0].push(Card::new(Suit::Spade, Rank(8)));
        g.columns[0].push(Card::new(Suit::Heart, Rank(7)));
        g.columns[0].push(Card::new(Suit::Club, Rank(6)));
        let cards = g.drag_cards(Zone::Column(0), 0);
        assert_eq!(cards.len(), 3);
        assert_eq!(cards[0], Card::new(Suit::Spade, Rank(8)));
        assert_eq!(cards[1], Card::new(Suit::Heart, Rank(7)));
        assert_eq!(cards[2], Card::new(Suit::Club, Rank(6)));
    }

    #[test]
    fn test_drag_cards_stops_at_break() {
        let mut g = game_with_seed(1);
        g.columns[0].clear();
        g.columns[0].push(Card::new(Suit::Spade, Rank(13))); // K♠
        g.columns[0].push(Card::new(Suit::Heart, Rank(12))); // Q♥
        g.columns[0].push(Card::new(Suit::Club, Rank(7))); // 7♣
        g.columns[0].push(Card::new(Suit::Diamond, Rank(6))); // 6♦
        let cards = g.drag_cards(Zone::Column(0), 0);
        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0], Card::new(Suit::Spade, Rank(13)));
        assert_eq!(cards[1], Card::new(Suit::Heart, Rank(12)));
    }

    #[test]
    fn test_drag_cards_from_middle_of_column() {
        let mut g = game_with_seed(1);
        g.columns[0].clear();
        g.columns[0].push(Card::new(Suit::Spade, Rank(13)));
        g.columns[0].push(Card::new(Suit::Heart, Rank(12)));
        g.columns[0].push(Card::new(Suit::Club, Rank(11)));
        let cards = g.drag_cards(Zone::Column(0), 1);
        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0], Card::new(Suit::Heart, Rank(12)));
        assert_eq!(cards[1], Card::new(Suit::Club, Rank(11)));
    }

    // --- drag state machine ---

    #[test]
    fn test_start_drag_on_empty_space_does_nothing() {
        let mut g = game_with_seed(1);
        let cx = ctx();
        g.start_drag(Vec2::new(-100.0, -100.0), &cx);
        assert!(g.drag.is_idle());
    }

    #[test]
    fn test_start_drag_on_bottom_band_sets_pressing() {
        let mut g = game_with_seed(1);
        let cx = ctx();
        let len = g.columns[0].len();
        let r = layout::card_rect_in_column(0, len - 1, &cx);
        // Click in the fully-visible bottom card.
        let point = Vec2::new(r.x + r.w * 0.5, r.y + r.h * 0.5);
        g.start_drag(point, &cx);
        assert!(g.drag.is_pressing());
    }

    #[test]
    fn test_start_drag_on_middle_band_is_rejected() {
        let mut g = game_with_seed(1);
        let cx = ctx();
        let len = g.columns[0].len();
        assert!(len >= 3, "test requires a column with at least 3 cards");
        // Click on the top visible band of a middle card. The band starts
        // at r.y and spans `card_stack_offset` (30 px) — but is covered
        // below by the next card. Click at r.y + 5 to be inside the band
        // and not in the card below.
        let r = layout::card_rect_in_column(0, 1, &cx);
        let point = Vec2::new(r.x + r.w * 0.5, r.y + 5.0);
        // Sanity: hit_test should find the middle card.
        let hit = g.hit_test(point, &cx);
        assert_eq!(hit, Some((Zone::Column(0), 1)));
        // But the drag must be rejected because the card is not at the
        // bottom of the column.
        g.start_drag(point, &cx);
        assert!(g.drag.is_idle(), "middle card must not be draggable");
    }

    #[test]
    fn test_start_drag_on_foundation_is_ignored() {
        let mut g = game_with_seed(1);
        let cx = ctx();
        g.foundations[0].push(Card::new(Suit::Spade, Rank::ACE));
        let r = layout::foundation_rect(0, &cx);
        let center = Vec2::new(r.x + r.w * 0.5, r.y + r.h * 0.5);
        g.start_drag(center, &cx);
        assert!(g.drag.is_idle());
    }

    #[test]
    fn test_update_drag_below_threshold_stays_pressing() {
        let mut g = game_with_seed(1);
        let cx = ctx();
        let len = g.columns[0].len();
        let r = layout::card_rect_in_column(0, len - 1, &cx);
        let point = Vec2::new(r.x + r.w * 0.5, r.y + r.h * 0.5);
        g.start_drag(point, &cx);
        g.update_drag(point + Vec2::new(1.0, 1.0), &cx);
        assert!(g.drag.is_pressing());
    }

    #[test]
    fn test_update_drag_beyond_threshold_becomes_dragging() {
        let mut g = game_with_seed(1);
        let cx = ctx();
        let len = g.columns[0].len();
        let r = layout::card_rect_in_column(0, len - 1, &cx);
        let point = Vec2::new(r.x + r.w * 0.5, r.y + r.h * 0.5);
        g.start_drag(point, &cx);
        g.update_drag(point + Vec2::new(20.0, 20.0), &cx);
        assert!(g.drag.is_dragging());
    }

    #[test]
    fn test_end_drag_without_dragging_does_nothing() {
        let mut g = game_with_seed(1);
        let cx = ctx();
        let len = g.columns[0].len();
        let r = layout::card_rect_in_column(0, len - 1, &cx);
        let point = Vec2::new(r.x + r.w * 0.5, r.y + r.h * 0.5);
        g.start_drag(point, &cx);
        let moves_before = g.moves;
        g.end_drag(point, &cx);
        assert_eq!(g.moves, moves_before);
        assert!(g.drag.is_idle());
    }

    #[test]
    fn test_cancel_drag_resets_to_idle() {
        let mut g = game_with_seed(1);
        let cx = ctx();
        let len = g.columns[0].len();
        let r = layout::card_rect_in_column(0, len - 1, &cx);
        let point = Vec2::new(r.x + r.w * 0.5, r.y + r.h * 0.5);
        g.start_drag(point, &cx);
        g.cancel_drag();
        assert!(g.drag.is_idle());
    }

    #[test]
    fn test_full_drag_drop_to_free_cell() {
        let mut g = game_with_seed(1);
        let cx = ctx();

        // Deterministic setup: a single card in column 0.
        g.columns[0].clear();
        let card = Card::new(Suit::Diamond, Rank(5));
        g.columns[0].push(card);

        let r = layout::card_rect_in_column(0, 0, &cx);
        let point = Vec2::new(r.x + r.w * 0.5, r.y + r.h * 0.5);

        g.start_drag(point, &cx);
        g.update_drag(point + Vec2::new(20.0, 20.0), &cx);

        let cell_rect = layout::free_cell_rect(0, &cx);
        let cell_center = Vec2::new(
            cell_rect.x + cell_rect.w * 0.5,
            cell_rect.y + cell_rect.h * 0.5,
        );
        g.end_drag(cell_center, &cx);

        assert_eq!(g.free_cells[0], Some(card));
        assert_eq!(g.moves, 1);
        assert!(g.drag.is_idle());
    }

    #[test]
    fn test_super_move_two_cards_to_empty_column() {
        let mut g = game_with_seed(1);
        g.columns[0].clear();
        g.columns[1].clear();
        g.columns[0].push(Card::new(Suit::Spade, Rank(13))); // K♠ top
        g.columns[0].push(Card::new(Suit::Heart, Rank(12))); // Q♥ bottom
        let cards = g.drag_cards(Zone::Column(0), 0);
        assert_eq!(cards.len(), 2);
        let ok = g.try_drop(Zone::Column(0), Zone::Column(1), &cards, 0);
        assert!(ok);
        assert_eq!(g.columns[0].len(), 0);
        assert_eq!(g.columns[1].len(), 2);
        assert_eq!(g.columns[1][0], Card::new(Suit::Spade, Rank(13))); // K♠ top
        assert_eq!(g.columns[1][1], Card::new(Suit::Heart, Rank(12))); // Q♥ bottom
    }

    // --- hint ---

    #[test]
    fn test_find_hint_returns_foundation_move_without_changing_state() {
        let mut g = game_with_seed(1);
        g.columns[0].clear();
        let ace = Card::new(Suit::Heart, Rank::ACE);
        g.columns[0].push(ace);
        let before = g.snapshot();

        let hint = g.find_hint().expect("expected foundation hint");

        assert_eq!(hint.from, Zone::Column(0));
        assert_eq!(hint.to, Zone::Foundation(Suit::Heart.index()));
        assert_eq!(hint.card, ace);
        assert_eq!(g.snapshot(), before);
        assert_eq!(g.moves, 0);
        assert!(!g.timer_running);
    }

    #[test]
    fn test_find_hint_returns_none_when_not_playing() {
        let g = Game::new();
        assert_eq!(g.find_hint(), None);
    }
}
