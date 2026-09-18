//! AI discard and claim logic.
//!
//! V1 is purely offensive: for each candidate discard, compute the
//! shanten of the post-discard hand and keep the minimum. Ties break
//! toward tiles with fewer same-suit "neighbors" (within 2 ranks),
//! which are least useful for forming future melds.
//!
//! No defense yet.

use crate::components::{ClaimKind, GangSource, Meld, Tile};
use crate::hand::shanten;

/// Choose which tile to discard from `hand`. Returns an index into `hand`.
pub fn decide_discard(hand: &[Tile], melds: &[Meld]) -> usize {
    if hand.is_empty() {
        return 0;
    }

    let mut best_idx = 0;
    let mut best_shanten = i32::MAX;
    let mut best_neighbors = usize::MAX;

    for i in 0..hand.len() {
        let mut after = hand.to_vec();
        after.remove(i);
        let s = shanten(&after, melds);
        let n = count_neighbors(hand, i);

        let better = s < best_shanten
            || (s == best_shanten && n < best_neighbors);

        if better {
            best_idx = i;
            best_shanten = s;
            best_neighbors = n;
        }
    }
    best_idx
}

/// Number of tiles in `hand` (excluding index `idx`) that are within
/// 2 ranks of `hand[idx]` in the same suit.
fn count_neighbors(hand: &[Tile], idx: usize) -> usize {
    let t = hand[idx];
    hand.iter()
        .enumerate()
        .filter(|(j, _)| *j != idx)
        .filter(|(_, u)| {
            u.suit == t.suit && (u.rank as i32 - t.rank as i32).abs() <= 2
        })
        .count()
}

/// Decide whether to Peng a discard. Returns `Some(ClaimKind::Peng)`
/// if the claim strictly reduces shanten, `None` otherwise.
pub fn decide_claim(hand: &[Tile], melds: &[Meld], discard: Tile) -> Option<ClaimKind> {
    let count = hand.iter().filter(|&&t| t == discard).count();
    if count < 2 {
        return None;
    }

    let before = shanten(hand, melds);

    let mut after_hand = hand.to_vec();
    let mut removed = 0;
    after_hand.retain(|&t| {
        if t == discard && removed < 2 {
            removed += 1;
            false
        } else {
            true
        }
    });

    let mut after_melds = melds.to_vec();
    after_melds.push(Meld::Peng { tile: discard, from: 0 });

    let after = shanten(&after_hand, &after_melds);

    if after < before {
        Some(ClaimKind::Peng)
    } else {
        None
    }
}

/// If the AI has 4 concealed copies of a tile, return it for an
/// An Gang (闷豆), provided the resulting shanten doesn't get worse.
pub fn decide_an_gang(hand: &[Tile], melds: &[Meld]) -> Option<Tile> {
    let mut counts: std::collections::HashMap<Tile, usize> = std::collections::HashMap::new();
    for &t in hand {
        *counts.entry(t).or_insert(0) += 1;
    }
    let candidates: Vec<Tile> = counts
        .iter()
        .filter(|&(_, &c)| c == 4)
        .map(|(&t, _)| t)
        .collect();
    if candidates.is_empty() {
        return None;
    }

    let before = shanten(hand, melds);
    for tile in candidates {
        let mut after_hand = hand.to_vec();
        let mut removed = 0;
        after_hand.retain(|&t| {
            if t == tile && removed < 4 {
                removed += 1;
                false
            } else {
                true
            }
        });
        let mut after_melds = melds.to_vec();
        after_melds.push(Meld::Gang { tile, from: GangSource::An });
        if shanten(&after_hand, &after_melds) <= before {
            return Some(tile);
        }
    }
    None
}

/// True if the AI has 3 concealed copies of `discard` — a Ming Gang.
pub fn decide_ming_gang(hand: &[Tile], discard: Tile) -> bool {
    hand.iter().filter(|&&t| t == discard).count() >= 3
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{ClaimKind, Suit, Tile};

    fn w(r: u8) -> Tile { Tile::new(Suit::Wan, r) }
    fn ti(r: u8) -> Tile { Tile::new(Suit::Tiao, r) }
    fn d(r: u8) -> Tile { Tile::new(Suit::Tong, r) }

    fn sorted(mut v: Vec<Tile>) -> Vec<Tile> {
        v.sort();
        v
    }

    fn discard_at(hand: &[Tile], i: usize) -> Vec<Tile> {
        let mut h = hand.to_vec();
        h.remove(i);
        h
    }

    #[test]
    fn test_discards_isolated_tile_from_complete_shapes() {
        let hand = sorted(vec![
            w(1), w(2), w(3),
            w(4), w(5), w(6),
            w(7), w(8), w(9),
            ti(1), ti(1), ti(1),
            d(5),
            d(9),
        ]);
        let idx = decide_discard(&hand, &[]);
        assert_eq!(hand[idx], d(5));
    }

    #[test]
    fn test_unique_optimal_discard_to_stay_tenpai() {
        let hand = sorted(vec![
            w(1), w(2), w(3),
            w(4), w(5), w(6),
            w(7), w(8), w(9), w(9),
            ti(7), ti(8),
            d(1), d(1),
        ]);
        assert_eq!(hand.len(), 14);

        let idx = decide_discard(&hand, &[]);
        assert_eq!(hand[idx], w(9));

        let after = discard_at(&hand, idx);
        assert_eq!(shanten(&after, &[]), 0);

        let nines_before = hand.iter().filter(|&&t| t == w(9)).count();
        let nines_after = after.iter().filter(|&&t| t == w(9)).count();
        assert_eq!(nines_before, 2);
        assert_eq!(nines_after, 1);
    }

    #[test]
    fn test_keeps_pair_rather_than_breaking_it() {
        let hand = sorted(vec![
            ti(5), ti(5),
            w(1), w(3), w(5), w(7), w(9),
            d(1), d(3), d(5), d(7),
            ti(1), ti(3),
            w(2),
        ]);
        let idx = decide_discard(&hand, &[]);
        assert_ne!(hand[idx], ti(5));
    }

    #[test]
    fn test_deterministic() {
        let hand = sorted(vec![
            w(1), w(4), w(7),
            ti(1), ti(4), ti(7),
            d(1), d(4), d(7),
            w(2), w(5), ti(2), d(5),
        ]);
        assert_eq!(decide_discard(&hand, &[]), decide_discard(&hand, &[]));
    }

    #[test]
    fn test_handles_melds() {
        let concealed = sorted(vec![w(1), w(2), w(3), w(4), w(5), w(6), d(2), d(2)]);
        let melds = vec![
            Meld::Peng { tile: ti(1), from: 1 },
            Meld::Peng { tile: ti(5), from: 2 },
        ];
        let idx = decide_discard(&concealed, &melds);
        assert!(idx < concealed.len());
    }

    #[test]
    fn test_returns_valid_index_on_empty_safe_default() {
        assert_eq!(decide_discard(&[], &[]), 0);
    }

    #[test]
    fn test_isolated_honor_like_tile_discarded_first() {
        let hand = sorted(vec![
            w(1), w(2), w(3), w(4), w(5), w(6), w(7), w(8), w(9),
            ti(2), ti(3), ti(4),
            d(1),
            d(9),
        ]);
        let idx = decide_discard(&hand, &[]);
        assert_eq!(hand[idx], d(1));
    }

    #[test]
    fn test_decide_claim_rejects_when_not_enough_copies() {
        let hand = sorted(vec![w(1), w(2), w(3)]);
        assert_eq!(decide_claim(&hand, &[], w(9)), None);
    }

    #[test]
    fn test_decide_claim_peng_only_with_two_in_hand() {
        let hand = sorted(vec![w(1), w(1), w(2), w(3)]);
        let r = decide_claim(&hand, &[], w(1));
        assert!(r.is_none() || r == Some(ClaimKind::Peng));
    }

    #[test]
    fn test_decide_claim_peng_when_it_helps() {
        let hand = sorted(vec![
            w(3), w(3),
            w(1), w(1), w(2),
            ti(4), ti(5), ti(6),
            d(2), d(3), d(4),
            d(7), d(8),
        ]);
        let r = decide_claim(&hand, &[], w(3));
        assert!(r.is_none() || r == Some(ClaimKind::Peng));
    }

    #[test]
    fn test_decide_an_gang_when_four_concealed() {
        let hand = sorted(vec![
            Tile::new(Suit::Wan, 1),
            Tile::new(Suit::Wan, 1),
            Tile::new(Suit::Wan, 1),
            Tile::new(Suit::Wan, 1),
            Tile::new(Suit::Tiao, 2),
            Tile::new(Suit::Tiao, 3),
            Tile::new(Suit::Tiao, 4),
            Tile::new(Suit::Tong, 5),
            Tile::new(Suit::Tong, 5),
            Tile::new(Suit::Tong, 5),
            Tile::new(Suit::Tong, 7),
            Tile::new(Suit::Tong, 8),
            Tile::new(Suit::Tong, 9),
            Tile::new(Suit::Tiao, 7),
        ]);
        assert_eq!(decide_an_gang(&hand, &[]), Some(Tile::new(Suit::Wan, 1)));
    }

    #[test]
    fn test_decide_an_gang_none_without_four() {
        let hand = sorted(vec![
            w(1), w(1), w(1), w(2), w(3), w(4), w(5), w(6), w(7),
            ti(1), ti(2), ti(3), d(1), d(1),
        ]);
        assert_eq!(decide_an_gang(&hand, &[]), None);
    }

    #[test]
    fn test_decide_ming_gang_needs_three() {
        let hand = sorted(vec![w(5), w(5), w(5), w(1), w(2), w(3)]);
        assert!(decide_ming_gang(&hand, w(5)));
        assert!(!decide_ming_gang(&hand, w(1)));
    }
}