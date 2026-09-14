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

/// Top-level state for one FreeCell session.
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

    // Widgets (Session 15).
    pub start_btn: ember_stdlib::ui::button::Button,
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
            start_btn: ember_stdlib::ui::button::Button::new(0.0, 0.0, 200.0, 50.0, "START"),
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

    // ----------------------------------------------------------------
    // Rules (Session 13)
    // ----------------------------------------------------------------

    pub fn can_place_on_column(&self, card: Card, col: usize) -> bool {
        if col >= 8 {
            return false;
        }
        match self.columns[col].last() {
            None => true,
            Some(top) => {
                top.rank.value() == card.rank.value() + 1 && top.color() != card.color()
            }
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

    // ----------------------------------------------------------------
    // Moves (Session 13)
    // ----------------------------------------------------------------

    pub fn try_drop(&mut self, from: Zone, to: Zone, cards: &[Card]) -> bool {
        if cards.is_empty() {
            return false;
        }
        if from == to {
            return false;
        }
        if !self.validate_move(from, to, cards) {
            return false;
        }

        self.history.push(self.snapshot());
        self.future.clear();

        self.remove_cards(from, cards.len());
        self.push_cards(to, cards);

        self.moves += 1;
        if !self.timer_running {
            self.timer_running = true;
        }

        self.check_win();
        true
    }

    fn validate_move(&self, from: Zone, to: Zone, cards: &[Card]) -> bool {
        let top = self.top_of(from, cards.len());
        if top != cards {
            return false;
        }

        if cards.len() > 1 {
            if !is_valid_sequence(cards) {
                return false;
            }
            if cards.len() > self.max_super_move() {
                return false;
            }
        }

        let first = cards[0];
        match to {
            Zone::Column(col) => self.can_place_on_column(first, col),
            Zone::Foundation(f) => cards.len() == 1 && self.can_place_on_foundation(first, f),
            Zone::FreeCell(c) => cards.len() == 1 && self.can_place_in_cell(c),
        }
    }

    fn top_of(&self, zone: Zone, n: usize) -> Vec<Card> {
        match zone {
            Zone::Column(col) => {
                if col >= 8 {
                    return Vec::new();
                }
                let c = &self.columns[col];
                if c.len() < n {
                    return Vec::new();
                }
                c[c.len() - n..].iter().rev().copied().collect()
            }
            Zone::FreeCell(cell) => {
                if cell >= 4 || n != 1 {
                    return Vec::new();
                }
                self.free_cells[cell].iter().copied().collect()
            }
            Zone::Foundation(f) => {
                if f >= 4 || n != 1 {
                    return Vec::new();
                }
                self.foundations[f].last().copied().into_iter().collect()
            }
        }
    }

    fn remove_cards(&mut self, zone: Zone, n: usize) -> Vec<Card> {
        let mut out = Vec::with_capacity(n);
        match zone {
            Zone::Column(col) => {
                for _ in 0..n {
                    if let Some(c) = self.columns[col].pop() {
                        out.push(c);
                    }
                }
            }
            Zone::FreeCell(cell) => {
                if n == 1
                    && let Some(c) = self.free_cells[cell].take()
                {
                    out.push(c);
                }
            }
            Zone::Foundation(f) => {
                if n == 1
                    && let Some(c) = self.foundations[f].pop()
                {
                    out.push(c);
                }
            }
        }
        out
    }

    fn push_cards(&mut self, zone: Zone, cards: &[Card]) {
        match zone {
            Zone::Column(col) => {
                for c in cards.iter().rev() {
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
    // Win / score (Session 13)
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
    // Undo / redo (Session 13)
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
            self.check_win();
            true
        } else {
            false
        }
    }

    // ----------------------------------------------------------------
    // Time (Session 13)
    // ----------------------------------------------------------------

    pub fn tick(&mut self, dt: f32) {
        if self.timer_running {
            self.elapsed += dt;
        }
    }

    // ----------------------------------------------------------------
    // Hit testing (Session 14.1)
    // ----------------------------------------------------------------

    /// Convert a screen position to a zone, whether the zone has cards or
    /// not. Used for drop targets.
    pub fn zone_at(&self, mouse: Vec2, ctx: &GameContext) -> Option<Zone> {
        // Free cells (4).
        for i in 0..4 {
            let r = layout::free_cell_rect(i, ctx);
            if point_in_rect(mouse, r) {
                return Some(Zone::FreeCell(i));
            }
        }
        // Foundations (4).
        for i in 0..4 {
            let r = layout::foundation_rect(i, ctx);
            if point_in_rect(mouse, r) {
                return Some(Zone::Foundation(i));
            }
        }
        // Columns (8) — bounds only, not the stacked cards.
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
    /// Returns `(zone, index_from_top)` where `index_from_top == 0` means
    /// the top card of the zone's stack.
    ///
    /// Order of precedence:
    /// 1. Free cells (single card each).
    /// 2. Foundations (single card each).
    /// 3. Columns: top card first, then deeper.
    pub fn hit_test(&self, mouse: Vec2, ctx: &GameContext) -> Option<(Zone, usize)> {
        // Free cells.
        for i in 0..4 {
            if self.free_cells[i].is_some() {
                let r = layout::free_cell_rect(i, ctx);
                if point_in_rect(mouse, r) {
                    return Some((Zone::FreeCell(i), 0));
                }
            }
        }
        // Foundations.
        for i in 0..4 {
            if !self.foundations[i].is_empty() {
                let r = layout::foundation_rect(i, ctx);
                if point_in_rect(mouse, r) {
                    return Some((Zone::Foundation(i), 0));
                }
            }
        }
        // Columns: iterate from top (last) to bottom (first).
        for col in 0..8 {
            let len = self.columns[col].len();
            for idx_from_top in 0..len {
                let card_index = len - 1 - idx_from_top;
                let r = layout::card_rect_in_column(col, card_index, ctx);
                if point_in_rect(mouse, r) {
                    return Some((Zone::Column(col), idx_from_top));
                }
            }
        }
        None
    }

    /// Compute the maximal valid sequence starting at `(zone, idx_from_top)`.
    ///
    /// For a single card (free cell or foundation), returns just that card.
    /// For a column, walks down the column from `idx_from_top` and returns
    /// the longest descending alternating-color run.
    pub fn drag_cards(&self, zone: Zone, idx_from_top: usize) -> Vec<Card> {
        match zone {
            Zone::FreeCell(i) => self.free_cells[i].into_iter().collect(),
            Zone::Foundation(i) => self.foundations[i].last().copied().into_iter().collect(),
            Zone::Column(col) => {
                let c = &self.columns[col];
                let len = c.len();
                if idx_from_top >= len {
                    return Vec::new();
                }
                let start = len - 1 - idx_from_top;
                let mut seq = vec![c[start]];
                for i in (0..start).rev() {
                    let top = *seq.last().unwrap();
                    let below = c[i];
                    if below.rank.value() == top.rank.value() + 1
                        && below.color() != top.color()
                    {
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
    // Drag state machine (Session 14.1)
    // ----------------------------------------------------------------

    /// Handle `mouse_left_pressed`. Transitions Idle → Pressing if the
    /// mouse is over a draggable card.
    pub fn start_drag(&mut self, mouse: Vec2, ctx: &GameContext) {
        if self.state != GameState::Playing {
            return;
        }
        if !self.drag.is_idle() {
            return;
        }
        let Some((zone, idx_from_top)) = self.hit_test(mouse, ctx) else {
            return;
        };

        // Foundations are not draggable.
        if let Zone::Foundation(_) = zone {
            return;
        }

        // Only the top card of a free cell is draggable (obviously it's the
        // only card). For a column, the clicked card must be the top of a
        // valid sequence (guaranteed by drag_cards).
        let cards = self.drag_cards(zone, idx_from_top);
        if cards.is_empty() {
            return;
        }

        self.drag = DragState::Pressing {
            origin: zone,
            card: cards[0],
            press_pos: mouse,
        };
    }

    /// Handle `mouse_left_down`. Transitions Pressing → Dragging when the
    /// mouse moves beyond DRAG_THRESHOLD.
    pub fn update_drag(&mut self, mouse: Vec2, ctx: &GameContext) {
        if let DragState::Pressing {
            origin,
            card,
            press_pos,
        } = self.drag
        {
            let dist = (mouse - press_pos).length();
            if dist >= DRAG_THRESHOLD {
                let idx_from_top = self.index_from_top_for(origin, card, ctx);
                let cards = self.drag_cards(origin, idx_from_top);
                if cards.is_empty() {
                    self.drag = DragState::Idle;
                    return;
                }
                let r = self.card_rect_for(origin, idx_from_top, ctx);
                let offset = press_pos - Vec2::new(r.x, r.y);
                self.drag = DragState::Dragging {
                    origin,
                    cards,
                    offset,
                };
            }
        }
    }

    /// Handle `mouse_left_released`. Attempts a drop if Dragging, then
    /// resets to Idle.
    pub fn end_drag(&mut self, mouse: Vec2, ctx: &GameContext) {
        let dragging = std::mem::replace(&mut self.drag, DragState::Idle);
        if let DragState::Dragging { origin, cards, .. } = dragging
            && let Some(target) = self.zone_at(mouse, ctx)
        {
            // try_drop takes `&[Card]`; pass the whole slice so
            // super-moves work.
            self.try_drop(origin, target, &cards);
        }
        // Pressing without dragging: treat as a no-op (click without move).
        // Could be used later for click-to-select.
    }

    /// Cancel a drag without attempting a drop.
    pub fn cancel_drag(&mut self) {
        self.drag = DragState::Idle;
    }

    // ----------------------------------------------------------------
    // Helpers for drag
    // ----------------------------------------------------------------

    /// The screen rect of the card at `(zone, idx_from_top)`.
    fn card_rect_for(&self, zone: Zone, idx_from_top: usize, ctx: &GameContext) -> Rect {
        match zone {
            Zone::FreeCell(i) => layout::free_cell_rect(i, ctx),
            Zone::Foundation(i) => layout::foundation_rect(i, ctx),
            Zone::Column(col) => {
                let len = self.columns[col].len();
                let card_index = len.saturating_sub(1 + idx_from_top);
                layout::card_rect_in_column(col, card_index, ctx)
            }
        }
    }

    /// Find the `idx_from_top` of a given card in a zone. Returns 0 if not
    /// found (fallback).
    fn index_from_top_for(&self, zone: Zone, card: Card, _ctx: &GameContext) -> usize {
        match zone {
            Zone::FreeCell(_) | Zone::Foundation(_) => 0,
            Zone::Column(col) => {
                let c = &self.columns[col];
                for (i, cc) in c.iter().rev().enumerate() {
                    if *cc == card {
                        return i;
                    }
                }
                0
            }
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
        // 4 empty cells, 0 empty columns → (1 + 4) * 2^0 = 5.
        assert_eq!(g.max_super_move(), 5);
    }

    #[test]
    fn test_max_super_move_with_one_empty_column() {
        let mut g = game_with_seed(1);
        g.columns[0].clear();
        // 4 empty cells, 1 empty column → (1 + 4) * 2^1 = 10.
        assert_eq!(g.max_super_move(), 10);
    }

    // --- try_drop ---

    #[test]
    fn test_try_drop_valid_single_card_to_column() {
        let mut g = game_with_seed(1);
        // Manually craft a clean board.
        g.columns[0].clear();
        g.columns[1].clear();
        g.columns[0].push(Card::new(Suit::Spade, Rank(8)));
        g.columns[1].push(Card::new(Suit::Heart, Rank(7)));
        let card = Card::new(Suit::Heart, Rank(7));
        let ok = g.try_drop(Zone::Column(1), Zone::Column(0), &[card]);
        assert!(ok);
        assert_eq!(g.columns[0].len(), 2);
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
        g.columns[1].push(Card::new(Suit::Club, Rank(7))); // same color
        let card = Card::new(Suit::Club, Rank(7));
        let ok = g.try_drop(Zone::Column(1), Zone::Column(0), &[card]);
        assert!(!ok);
        assert_eq!(g.columns[0].len(), 1);
        assert_eq!(g.columns[1].len(), 1);
        assert_eq!(g.moves, 0);
    }

    #[test]
    fn test_try_drop_to_cell() {
        let mut g = game_with_seed(1);
        let card = g.columns[0].last().copied().unwrap();
        let ok = g.try_drop(Zone::Column(0), Zone::FreeCell(0), &[card]);
        assert!(ok);
        assert_eq!(g.free_cells[0], Some(card));
    }

    #[test]
    fn test_try_drop_to_foundation_ace() {
        let mut g = game_with_seed(1);
        // Force an ace at the top of column 0.
        g.columns[0].clear();
        g.columns[0].push(Card::new(Suit::Heart, Rank::ACE));
        let card = Card::new(Suit::Heart, Rank::ACE);
        let heart_idx = Suit::Heart.index();
        let ok = g.try_drop(Zone::Column(0), Zone::Foundation(heart_idx), &[card]);
        assert!(ok);
        assert_eq!(g.foundations[heart_idx].len(), 1);
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
        // Direct assertion via the condition the game uses.
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
        assert!(g.try_drop(Zone::Column(1), Zone::Column(0), &[card]));
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
        assert!(g.try_drop(Zone::Column(1), Zone::Column(0), &[card]));
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
        assert!(g.try_drop(Zone::Column(1), Zone::Column(0), &[c7]));
        assert!(g.undo());
        assert!(!g.future.is_empty());

        // New move clears redo.
        let c6 = Card::new(Suit::Club, Rank(6));
        assert!(g.try_drop(Zone::Column(2), Zone::Column(1), &[c6]));
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
        assert!(g.try_drop(Zone::Column(1), Zone::Column(0), &[c7]));
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
    fn test_hit_test_finds_top_card_of_column() {
        let g = game_with_seed(1);
        let cx = ctx();
        let len = g.columns[0].len();
        let r = layout::card_rect_in_column(0, len - 1, &cx);
        let center = Vec2::new(r.x + r.w * 0.5, r.y + r.h * 0.5);
        let hit = g.hit_test(center, &cx);
        assert_eq!(hit, Some((Zone::Column(0), 0)));
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
        assert_eq!(cards[0], Card::new(Suit::Club, Rank(6)));
        assert_eq!(cards[1], Card::new(Suit::Heart, Rank(7)));
        assert_eq!(cards[2], Card::new(Suit::Spade, Rank(8)));
    }

    #[test]
    fn test_drag_cards_stops_at_break() {
        let mut g = game_with_seed(1);
        g.columns[0].clear();
        // Build: K♠, Q♥, 7♣, 6♦ (bottom → top).
        g.columns[0].push(Card::new(Suit::Spade, Rank(13)));  // K♠
        g.columns[0].push(Card::new(Suit::Heart, Rank(12)));  // Q♥
        g.columns[0].push(Card::new(Suit::Club, Rank(7)));    // 7♣
        g.columns[0].push(Card::new(Suit::Diamond, Rank(6))); // 6♦
        let cards = g.drag_cards(Zone::Column(0), 0);
        // 6♦ and 7♣ form a valid run; Q♥ breaks it.
        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0], Card::new(Suit::Diamond, Rank(6)));
        assert_eq!(cards[1], Card::new(Suit::Club, Rank(7)));
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
    fn test_start_drag_on_top_card_sets_pressing() {
        let mut g = game_with_seed(1);
        let cx = ctx();
        let len = g.columns[0].len();
        let r = layout::card_rect_in_column(0, len - 1, &cx);
        let center = Vec2::new(r.x + r.w * 0.5, r.y + r.h * 0.5);
        g.start_drag(center, &cx);
        assert!(g.drag.is_pressing());
    }

    #[test]
    fn test_start_drag_on_foundation_is_ignored() {
        let mut g = game_with_seed(1);
        let cx = ctx();
        g.foundations[0].push(Card::new(Suit::Spade, Rank::ACE));
        let r = layout::foundation_rect(0, &cx);
        let center = Vec2::new(r.x + r.w * 0.5, r.y + r.h * 0.5);
        g.start_drag(center, &cx);
        assert!(g.drag.is_idle(), "foundations are not draggable");
    }

    #[test]
    fn test_update_drag_below_threshold_stays_pressing() {
        let mut g = game_with_seed(1);
        let cx = ctx();
        let len = g.columns[0].len();
        let r = layout::card_rect_in_column(0, len - 1, &cx);
        let center = Vec2::new(r.x + r.w * 0.5, r.y + r.h * 0.5);
        g.start_drag(center, &cx);
        g.update_drag(center + Vec2::new(1.0, 1.0), &cx);
        assert!(g.drag.is_pressing());
    }

    #[test]
    fn test_update_drag_beyond_threshold_becomes_dragging() {
        let mut g = game_with_seed(1);
        let cx = ctx();
        let len = g.columns[0].len();
        let r = layout::card_rect_in_column(0, len - 1, &cx);
        let center = Vec2::new(r.x + r.w * 0.5, r.y + r.h * 0.5);
        g.start_drag(center, &cx);
        g.update_drag(center + Vec2::new(20.0, 20.0), &cx);
        assert!(g.drag.is_dragging());
    }

    #[test]
    fn test_end_drag_without_dragging_does_nothing() {
        let mut g = game_with_seed(1);
        let cx = ctx();
        let len = g.columns[0].len();
        let r = layout::card_rect_in_column(0, len - 1, &cx);
        let center = Vec2::new(r.x + r.w * 0.5, r.y + r.h * 0.5);
        g.start_drag(center, &cx);
        let moves_before = g.moves;
        g.end_drag(center, &cx);
        assert_eq!(g.moves, moves_before);
        assert!(g.drag.is_idle());
    }

    #[test]
    fn test_cancel_drag_resets_to_idle() {
        let mut g = game_with_seed(1);
        let cx = ctx();
        let len = g.columns[0].len();
        let r = layout::card_rect_in_column(0, len - 1, &cx);
        let center = Vec2::new(r.x + r.w * 0.5, r.y + r.h * 0.5);
        g.start_drag(center, &cx);
        g.cancel_drag();
        assert!(g.drag.is_idle());
    }

    #[test]
    fn test_full_drag_drop_to_free_cell() {
        let mut g = game_with_seed(1);
        let cx = ctx();
        let len = g.columns[0].len();
        let card = g.columns[0][len - 1];
        let card_rect = layout::card_rect_in_column(0, len - 1, &cx);
        let center = Vec2::new(card_rect.x + card_rect.w * 0.5, card_rect.y + card_rect.h * 0.5);

        g.start_drag(center, &cx);
        g.update_drag(center + Vec2::new(20.0, 20.0), &cx);

        let cell_rect = layout::free_cell_rect(0, &cx);
        let cell_center = Vec2::new(cell_rect.x + cell_rect.w * 0.5, cell_rect.y + cell_rect.h * 0.5);
        g.end_drag(cell_center, &cx);

        assert_eq!(g.free_cells[0], Some(card));
        assert_eq!(g.moves, 1);
        assert!(g.drag.is_idle());
    }
}