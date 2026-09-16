//! Deck creation and shuffle.

use ember_core::rng::Rng;

use crate::components::{Card, Rank, Suit};

/// Build a full 52-card deck in canonical order (all spades, then hearts,
/// then diamonds, then clubs, each A..K).
pub fn full_deck() -> Vec<Card> {
    let mut deck = Vec::with_capacity(52);
    for suit in Suit::ALL {
        for r in 1..=13 {
            deck.push(Card::new(suit, Rank(r)));
        }
    }
    deck
}

/// Shuffle a deck in place with Fisher-Yates, using the given RNG.
/// Deterministic given the same seed.
pub fn shuffle(deck: &mut [Card], rng: &mut Rng) {
    let n = deck.len();
    for i in (1..n).rev() {
        let j = rng.next_range(i + 1);
        deck.swap(i, j);
    }
}

/// Build a shuffled 52-card deck from a seed.
pub fn deal(seed: u32) -> Vec<Card> {
    let mut rng = Rng::new(seed);
    let mut deck = full_deck();
    shuffle(&mut deck, &mut rng);
    deck
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_full_deck_has_52_cards() {
        assert_eq!(full_deck().len(), 52);
    }

    #[test]
    fn test_full_deck_has_4_of_each_rank() {
        let deck = full_deck();
        for r in 1..=13 {
            let count = deck.iter().filter(|c| c.rank.0 == r).count();
            assert_eq!(count, 4, "rank {r} appears {count} times");
        }
    }

    #[test]
    fn test_full_deck_has_13_of_each_suit() {
        let deck = full_deck();
        for s in Suit::ALL {
            let count = deck.iter().filter(|c| c.suit == s).count();
            assert_eq!(count, 13, "suit {:?} appears {count} times", s);
        }
    }

    #[test]
    fn test_full_deck_has_no_duplicates() {
        let deck = full_deck();
        let set: HashSet<_> = deck.iter().collect();
        assert_eq!(set.len(), 52, "deck contains duplicates");
    }

    #[test]
    fn test_deal_is_deterministic() {
        let a = deal(42);
        let b = deal(42);
        assert_eq!(a, b);
    }

    #[test]
    fn test_different_seeds_differ() {
        let a = deal(1);
        let b = deal(2);
        assert_ne!(a, b, "different seeds should produce different decks");
    }

    #[test]
    fn test_deal_is_a_permutation() {
        // Any dealt deck must be a permutation of the full deck.
        let full: HashSet<_> = full_deck().into_iter().collect();
        let dealt: HashSet<_> = deal(123).into_iter().collect();
        assert_eq!(full, dealt);
    }
}
