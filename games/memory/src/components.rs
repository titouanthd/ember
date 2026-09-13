//! Card, CardState, Symbol — the board data model.

/// A single symbol that can appear on a card.
///
/// Just a `char` wrapper so we can add methods later (rendering, equality
/// by symbol value). The actual set used is defined in `difficulties.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Symbol(pub char);

impl Symbol {
    pub fn as_char(self) -> char {
        self.0
    }
}

/// Visual / logical state of a single card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardState {
    /// Face down, clickable.
    Hidden,
    /// Face up, transition in progress (first click or no-match delay).
    /// Not clickable.
    Flipped,
    /// Face up, permanently matched. Not clickable.
    Matched,
}

/// One card on the board.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Card {
    pub symbol: Symbol,
    pub state: CardState,
}

impl Card {
    pub fn new(symbol: Symbol) -> Self {
        Self {
            symbol,
            state: CardState::Hidden,
        }
    }

    pub fn is_hidden(&self) -> bool {
        self.state == CardState::Hidden
    }

    pub fn is_flipped(&self) -> bool {
        self.state == CardState::Flipped
    }

    pub fn is_matched(&self) -> bool {
        self.state == CardState::Matched
    }

    /// True if the card can be clicked (hidden, not flipped/matched).
    pub fn is_clickable(&self) -> bool {
        self.state == CardState::Hidden
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_card_starts_hidden() {
        let c = Card::new(Symbol('$'));
        assert!(c.is_hidden());
        assert!(c.is_clickable());
        assert!(!c.is_flipped());
        assert!(!c.is_matched());
    }

    #[test]
    fn test_symbol_roundtrip() {
        let s = Symbol('>');
        assert_eq!(s.as_char(), '>');
    }

    #[test]
    fn test_card_state_transitions() {
        let mut c = Card::new(Symbol('§'));
        c.state = CardState::Flipped;
        assert!(c.is_flipped());
        assert!(!c.is_clickable());
        c.state = CardState::Matched;
        assert!(c.is_matched());
        assert!(!c.is_clickable());
    }
}