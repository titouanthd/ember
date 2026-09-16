//! Card model: suits, ranks, cards, colors, zones.

/// The four suits. Color is derived.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Suit {
    Spade,
    Heart,
    Diamond,
    Club,
}

impl Suit {
    pub const ALL: [Suit; 4] = [Suit::Spade, Suit::Heart, Suit::Diamond, Suit::Club];

    /// Red or black.
    pub fn color(self) -> Color {
        match self {
            Suit::Heart | Suit::Diamond => Color::Red,
            Suit::Spade | Suit::Club => Color::Black,
        }
    }

    /// Unicode symbol. Must be verified against macroquad's default font.
    pub fn symbol(self) -> char {
        match self {
            Suit::Spade => '♠',
            Suit::Heart => '♥',
            Suit::Diamond => '♦',
            Suit::Club => '♣',
        }
    }

    /// ASCII fallback if Unicode doesn't render.
    pub fn ascii(self) -> char {
        match self {
            Suit::Spade => 'S',
            Suit::Heart => 'H',
            Suit::Diamond => 'D',
            Suit::Club => 'C',
        }
    }

    /// Index for the foundations array (0..4).
    pub fn index(self) -> usize {
        match self {
            Suit::Spade => 0,
            Suit::Heart => 1,
            Suit::Diamond => 2,
            Suit::Club => 3,
        }
    }
}

/// Card color (derived from suit).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color {
    Red,
    Black,
}

/// Rank: 1 = Ace, 2..=10, 11 = Jack, 12 = Queen, 13 = King.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Rank(pub u8);

impl Rank {
    pub const ACE: Rank = Rank(1);
    pub const KING: Rank = Rank(13);

    pub fn value(self) -> u8 {
        self.0
    }

    /// 'A', '2'..'9', '10' -> 'T', 'J', 'Q', 'K'.
    /// Note : '10' n'est pas un char, donc on utilise 'T' (comme dans les
    /// notations de bridge).
    pub fn label(self) -> char {
        match self.0 {
            1 => 'A',
            10 => 'T',
            11 => 'J',
            12 => 'Q',
            13 => 'K',
            n => char::from_digit(n as u32, 10).unwrap_or('?'),
        }
    }
}

/// One card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Card {
    pub suit: Suit,
    pub rank: Rank,
}

impl Card {
    pub fn new(suit: Suit, rank: Rank) -> Self {
        Self { suit, rank }
    }

    pub fn color(&self) -> Color {
        self.suit.color()
    }

    pub fn symbol(&self) -> char {
        self.suit.symbol()
    }

    pub fn ascii(&self) -> char {
        self.suit.ascii()
    }

    pub fn rank_label(&self) -> char {
        self.rank.label()
    }
}

/// A zone on the board. Used for drag-and-drop and move validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Zone {
    Column(usize),     // 0..8
    FreeCell(usize),   // 0..4
    Foundation(usize), // 0..4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suit_colors() {
        assert_eq!(Suit::Heart.color(), Color::Red);
        assert_eq!(Suit::Diamond.color(), Color::Red);
        assert_eq!(Suit::Spade.color(), Color::Black);
        assert_eq!(Suit::Club.color(), Color::Black);
    }

    #[test]
    fn test_rank_labels() {
        assert_eq!(Rank(1).label(), 'A');
        assert_eq!(Rank(2).label(), '2');
        assert_eq!(Rank(9).label(), '9');
        assert_eq!(Rank(10).label(), 'T');
        assert_eq!(Rank(11).label(), 'J');
        assert_eq!(Rank(12).label(), 'Q');
        assert_eq!(Rank(13).label(), 'K');
    }

    #[test]
    fn test_rank_ordering() {
        assert!(Rank(1) < Rank(2));
        assert!(Rank(13) > Rank(12));
        assert!(Rank::ACE < Rank::KING);
    }

    #[test]
    fn test_suit_index_unique() {
        let mut seen = [false; 4];
        for s in Suit::ALL {
            assert!(!seen[s.index()], "suit index collision");
            seen[s.index()] = true;
        }
    }

    #[test]
    fn test_card_symbol_and_ascii() {
        let h = Card::new(Suit::Heart, Rank(5));
        assert_eq!(h.symbol(), '♥');
        assert_eq!(h.ascii(), 'H');
        assert_eq!(h.color(), Color::Red);
        assert_eq!(h.rank_label(), '5');
    }
}
