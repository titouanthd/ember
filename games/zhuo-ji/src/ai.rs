//! AI discard logic.
//!
//! V1 is purely offensive: for each candidate discard, compute the
//! shanten of the post-discard hand and keep the minimum. Ties break
//! toward tiles with fewer same-suit "neighbors" (within 2 ranks),
//! which are least useful for forming future melds.
//!
//! No defense yet. The AI ignores what other players are collecting.
//! That arrives in a V2 heuristic once the game is playable end-to-end.

use crate::components::{Meld, Tile};
use crate::hand::shanten;

/// Choose which tile to discard from `hand`. Returns an index into `hand`.
///
/// Preconditions: `hand` is non-empty. If empty, returns 0 (caller must
/// guard — a Mahjong turn always has at least one tile to discard).
///
/// Deterministic: same input always yields the same index. Ties break
/// toward the lowest index, which is also the lowest-ranked tile in a
/// sorted hand — a human-like "throw the smallest useful-adjacent tile"
/// behavior.
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

/// Number of tiles in `hand` (excluding index `i`) that are within
/// 2 ranks of `hand[i]` in the same suit. Higher = more connected =
/// more useful to keep.
fn count_neighbors(hand: &[Tile], i: usize) -> usize {
    let t = hand[i];
    hand.iter()
        .enumerate()
        .filter(|(j, _)| *j != i)
        .filter(|(_, u)| {
            u.suit == t.suit && (u.rank as i32 - t.rank as i32).abs() <= 2
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{Suit, Tile};

    fn w(r: u8) -> Tile { Tile::new(Suit::Wan, r) }
    fn ti(r: u8) -> Tile { Tile::new(Suit::Tiao, r) }
    fn d(r: u8) -> Tile { Tile::new(Suit::Tong, r) }

    fn sorted(mut v: Vec<Tile>) -> Vec<Tile> {
        v.sort();
        v
    }

    /// Discard index i, return the resulting hand.
    fn discard_at(hand: &[Tile], i: usize) -> Vec<Tile> {
        let mut h = hand.to_vec();
        h.remove(i);
        h
    }

    #[test]
    fn test_discards_isolated_tile_from_complete_shapes() {
        // 3 Wan sequences + Tiao triplet + lone 5D. The lone tile is
        // index 13 in a sorted hand (all W < all T < all D).
        let hand = sorted(vec![
            w(1), w(2), w(3),
            w(4), w(5), w(6),
            w(7), w(8), w(9),
            ti(1), ti(1), ti(1),
            d(5),
            d(9), // second isolated tile
        ]);
        let idx = decide_discard(&hand, &[]);
        // Both d5 and d9 are isolated. Tie-break goes to the earlier
        // index (d5) → index 12 in the sorted 14-tile hand.
        assert_eq!(hand[idx], d(5));
    }

    #[test]
    fn test_unique_optimal_discard_to_stay_tenpai() {
        // Hand: 123W, 456W, 789W (3 melds) + 78T (partial) + 11D (pair)
        //       + a SECOND 9W as the extra tile.
        //
        // Only discarding the extra 9W keeps tenpai (waiting on 6T or 9T).
        // Every other discard loses something structural:
        //   - discard 1D → no pair → 1-shanten
        //   - discard 7T or 8T → no partial → 1-shanten
        //   - discard any W tile that's part of a sequence → breaks a meld
        let hand = sorted(vec![
            w(1), w(2), w(3),
            w(4), w(5), w(6),
            w(7), w(8), w(9), w(9), // two 9W: one in 789W, one extra
            ti(7), ti(8),
            d(1), d(1),
        ]);
        assert_eq!(hand.len(), 14);

        let idx = decide_discard(&hand, &[]);
        assert_eq!(hand[idx], w(9), "should discard the extra 9W");

        let after = discard_at(&hand, idx);
        assert_eq!(shanten(&after, &[]), 0, "hand must remain tenpai");

        // Sanity: the discarded tile really was the extra one, not a
        // sequence member. The 13-tile remainder should contain 3 W
        // sequences + 78T + 11D and no extra 9W.
        let nines_in_hand_before = hand.iter().filter(|&&t| t == w(9)).count();
        let nines_after = after.iter().filter(|&&t| t == w(9)).count();
        assert_eq!(nines_in_hand_before, 2);
        assert_eq!(nines_after, 1);
    }

    #[test]
    fn test_keeps_pair_rather_than_breaking_it() {
        // The pair (5T) is the only pair. Discarding one of them loses
        // the pair, so the AI should discard something else.
        let hand = sorted(vec![
            ti(5), ti(5),
            w(1), w(3), w(5), w(7), w(9),
            d(1), d(3), d(5), d(7),
            ti(1), ti(3),
            w(2), // partial with w1 or w3
        ]);
        let idx = decide_discard(&hand, &[]);
        let discarded = hand[idx];
        // AI should not discard 5T (it's the only pair).
        assert_ne!(discarded, ti(5));
    }

    #[test]
    fn test_deterministic() {
        let hand = sorted(vec![
            w(1), w(4), w(7),
            ti(1), ti(4), ti(7),
            d(1), d(4), d(7),
            w(2), w(5), ti(2), d(5),
        ]);
        let a = decide_discard(&hand, &[]);
        let b = decide_discard(&hand, &[]);
        assert_eq!(a, b);
    }

    #[test]
    fn test_handles_melds() {
        // Two Pengs called. Concealed: 6 tiles → need 2 more melds.
        // 1-2-3W, 4-5-6W are already 2 melds, so concealed is 4 melds
        // worth of tiles minus 2. Wait: with 2 melds, need 2 more +
        // pair, so concealed should be 3*2+2 = 8.
        // Let's use 8 concealed: 1-2-3W, 4-5-6W, 2D, 2D.
        // The AI should keep them (already winning shape).
        let concealed = sorted(vec![
            w(1), w(2), w(3), w(4), w(5), w(6), d(2), d(2),
        ]);
        let melds = vec![
            Meld::Peng { tile: ti(1), from: 1 },
            Meld::Peng { tile: ti(5), from: 2 },
        ];
        let idx = decide_discard(&concealed, &melds);
        // Discarding anything breaks a complete hand. The AI should
        // still return a valid index (0 is fine as last resort).
        assert!(idx < concealed.len());
    }

    #[test]
    fn test_returns_valid_index_on_empty_safe_default() {
        let idx = decide_discard(&[], &[]);
        assert_eq!(idx, 0);
    }

    #[test]
    fn test_isolated_honor_like_tile_discarded_first() {
        // Guiyang has no honors, but a lone terminal far from everything
        // is the closest analog.
        let hand = sorted(vec![
            w(1), w(2), w(3), w(4), w(5), w(6), w(7), w(8), w(9),
            ti(2), ti(3), ti(4),
            d(1), // lone 1D, far from anything else
            d(9), // lone 9D
        ]);
        let idx = decide_discard(&hand, &[]);
        // Both d1 and d9 are isolated. Tie-break: lowest index → d1.
        assert_eq!(hand[idx], d(1));
    }
}