//! Drag-and-drop state machine. Skeleton for Session 14.

use glam::Vec2;

use crate::components::{Card, Zone};

/// The current drag state.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum DragState {
    /// Nothing in progress.
    #[default]
    Idle,
    /// Mouse is down on a card, but not yet dragging.
    Pressing { origin: Zone, card: Card, press_pos: Vec2 },
    /// Mouse is moving with a stack of cards.
    Dragging { origin: Zone, cards: Vec<Card>, offset: Vec2 },
}

impl DragState {
    pub fn is_idle(&self) -> bool {
        matches!(self, DragState::Idle)
    }

    pub fn is_pressing(&self) -> bool {
        matches!(self, DragState::Pressing { .. })
    }

    pub fn is_dragging(&self) -> bool {
        matches!(self, DragState::Dragging { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{Rank, Suit};

    #[test]
    fn test_default_is_idle() {
        let d = DragState::default();
        assert!(d.is_idle());
        assert!(!d.is_pressing());
        assert!(!d.is_dragging());
    }

    #[test]
    fn test_pressing_state() {
        let d = DragState::Pressing {
            origin: Zone::Column(0),
            card: Card::new(Suit::Heart, Rank(5)),
            press_pos: Vec2::new(100.0, 100.0),
        };
        assert!(d.is_pressing());
        assert!(!d.is_dragging());
    }

    #[test]
    fn test_dragging_state() {
        let d = DragState::Dragging {
            origin: Zone::Column(0),
            cards: vec![Card::new(Suit::Heart, Rank(5))],
            offset: Vec2::new(10.0, 10.0),
        };
        assert!(d.is_dragging());
        assert!(!d.is_pressing());
    }
}