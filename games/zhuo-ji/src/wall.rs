//! Wall creation, shuffle, and deal.

use ember_core::rng::Rng;

use crate::components::{Player, Suit, Tile};

pub const WALL_SIZE: usize = 108;
pub const HAND_SIZE: usize = 13;

pub fn build_wall() -> Vec<Tile> {
    let mut wall = Vec::with_capacity(WALL_SIZE);
    for suit in Suit::ALL {
        for rank in 1..=9 {
            for _ in 0..4 {
                wall.push(Tile::new(suit, rank));
            }
        }
    }
    wall
}

pub fn shuffle(wall: &mut [Tile], rng: &mut Rng) {
    let n = wall.len();
    if n < 2 { return; }
    for i in (1..n).rev() {
        let j = rng.next_range(i + 1);
        wall.swap(i, j);
    }
}

/// Deal 13 tiles to each player. The dealer's extra (14th) tile goes
/// into `drawn`, so it can be rendered separately from the hand.
pub fn deal(wall: &mut Vec<Tile>, players: &mut [Player; 4], dealer: usize) {
    for _ in 0..HAND_SIZE {
        for p in players.iter_mut() {
            if let Some(t) = wall.pop() {
                p.concealed.push(t);
            }
        }
    }
    if let Some(t) = wall.pop() {
        players[dealer].drawn = Some(t);
    }
    for p in players.iter_mut() {
        p.concealed.sort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_build_wall_has_108_tiles() {
        assert_eq!(build_wall().len(), WALL_SIZE);
    }

    #[test]
    fn test_wall_has_four_of_each_of_27_unique_tiles() {
        let wall = build_wall();
        let mut counts: HashMap<Tile, usize> = HashMap::new();
        for t in &wall {
            *counts.entry(*t).or_insert(0) += 1;
        }
        assert_eq!(counts.len(), 27);
        for c in counts.values() {
            assert_eq!(*c, 4);
        }
    }

    #[test]
    fn test_shuffle_is_deterministic_given_seed() {
        let mut a = build_wall();
        let mut b = build_wall();
        shuffle(&mut a, &mut Rng::new(42));
        shuffle(&mut b, &mut Rng::new(42));
        assert_eq!(a, b);
    }

    #[test]
    fn test_different_seeds_produce_different_shuffles() {
        let mut a = build_wall();
        let mut b = build_wall();
        shuffle(&mut a, &mut Rng::new(1));
        shuffle(&mut b, &mut Rng::new(2));
        assert_ne!(a, b);
    }

    #[test]
    fn test_shuffle_preserves_multiset() {
        let mut wall = build_wall();
        let mut original: HashMap<Tile, usize> = HashMap::new();
        for t in &wall {
            *original.entry(*t).or_insert(0) += 1;
        }
        shuffle(&mut wall, &mut Rng::new(7));
        let mut after: HashMap<Tile, usize> = HashMap::new();
        for t in &wall {
            *after.entry(*t).or_insert(0) += 1;
        }
        assert_eq!(original, after);
    }

    #[test]
    fn test_shuffle_of_empty_is_noop() {
        let mut empty: Vec<Tile> = Vec::new();
        shuffle(&mut empty, &mut Rng::new(1));
        assert!(empty.is_empty());
    }

    #[test]
    fn test_deal_sizes() {
        let mut wall = build_wall();
        shuffle(&mut wall, &mut Rng::new(1));
        let mut players = [
            Player::new(false),
            Player::new(true),
            Player::new(true),
            Player::new(true),
        ];
        deal(&mut wall, &mut players, 0);

        assert_eq!(wall.len(), 55);
        assert_eq!(players[0].concealed.len(), 13);
        assert!(players[0].drawn.is_some());
        assert_eq!(players[1].concealed.len(), 13);
        assert!(players[1].drawn.is_none());
        assert_eq!(players[2].concealed.len(), 13);
        assert_eq!(players[3].concealed.len(), 13);
    }

    #[test]
    fn test_dealt_hands_are_sorted() {
        let mut wall = build_wall();
        shuffle(&mut wall, &mut Rng::new(7));
        let mut players = [
            Player::new(false),
            Player::new(true),
            Player::new(true),
            Player::new(true),
        ];
        deal(&mut wall, &mut players, 0);
        for p in &players {
            let mut sorted = p.concealed.clone();
            sorted.sort();
            assert_eq!(p.concealed, sorted);
        }
    }

    #[test]
    fn test_deal_with_different_dealer_gives_extra_to_that_seat() {
        let mut wall = build_wall();
        shuffle(&mut wall, &mut Rng::new(9));
        let mut players = [
            Player::new(false),
            Player::new(true),
            Player::new(true),
            Player::new(true),
        ];
        deal(&mut wall, &mut players, 2);
        assert!(players[2].drawn.is_some());
        assert!(players[0].drawn.is_none());
        assert!(players[1].drawn.is_none());
    }
}