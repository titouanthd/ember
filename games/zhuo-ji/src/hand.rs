//! Hand evaluation: win detection, tenpai, shanten.
//!
//! Pure functions on tiles. No macroquad, no game state.

use std::collections::HashMap;

use crate::components::{Meld, Tile};

/// `true` if `concealed` + `melds` forms a complete hand:
/// 4 sets (triplets or sequences) + 1 pair.
pub fn is_winning_hand(concealed: &[Tile], melds: &[Meld]) -> bool {
    let needed = 4usize.saturating_sub(melds.len());
    if concealed.len() != 3 * needed + 2 {
        return false;
    }

    let mut sorted = concealed.to_vec();
    sorted.sort();

    let mut i = 0;
    while i < sorted.len() {
        let count = sorted[i..].iter().take_while(|&&t| t == sorted[i]).count();
        if count >= 2 {
            let mut rest = sorted.clone();
            rest.remove(i);
            rest.remove(i);
            if can_form_melds(&rest, needed) {
                return true;
            }
        }
        i += count;
    }
    false
}

/// `true` if `tiles` can be fully partitioned into exactly `needed`
/// melds (triplets or sequences).
fn can_form_melds(tiles: &[Tile], needed: usize) -> bool {
    if tiles.is_empty() {
        return needed == 0;
    }
    if needed == 0 {
        return false;
    }

    let first = tiles[0];

    if tiles.len() >= 3 && tiles[1] == first && tiles[2] == first {
        let mut rest = tiles.to_vec();
        rest.drain(0..3);
        if can_form_melds(&rest, needed - 1) {
            return true;
        }
    }

    if first.rank <= 7 {
        let t2 = Tile::new(first.suit, first.rank + 1);
        let t3 = Tile::new(first.suit, first.rank + 2);
        let pos2 = tiles.iter().position(|&t| t == t2);
        let pos3 = tiles.iter().position(|&t| t == t3);
        if let (Some(i2), Some(i3)) = (pos2, pos3) {
            let mut rest = tiles.to_vec();
            rest.remove(i3);
            rest.remove(i2);
            rest.remove(0);
            if can_form_melds(&rest, needed - 1) {
                return true;
            }
        }
    }

    false
}

/// Distance to tenpai. `-1` means the hand is already complete.
/// `0` means tenpai (one tile away). Larger values are worse.
pub fn shanten(concealed: &[Tile], melds: &[Meld]) -> i32 {
    let needed = 4usize.saturating_sub(melds.len());
    if concealed.len() > 3 * needed + 2 {
        return i32::MAX;
    }

    let mut tiles = concealed.to_vec();
    tiles.sort();

    let mut best = i32::MAX;

    let mut i = 0;
    while i < tiles.len() {
        let count = tiles[i..].iter().take_while(|&&t| t == tiles[i]).count();
        if count >= 2 {
            let mut rest = tiles.clone();
            rest.remove(i);
            rest.remove(i);
            let (m, p) = max_melds_partials(&rest, needed);
            let s = (needed as i32 - m as i32) * 2 - p as i32 - 1;
            if s < best {
                best = s;
            }
        }
        i += count;
    }

    {
        let (m, p) = max_melds_partials(&tiles, needed);
        let s = (needed as i32 - m as i32) * 2 - p as i32;
        if s < best {
            best = s;
        }
    }

    best.max(-1)
}

/// `true` if the hand is one tile away from winning.
pub fn is_tenpai(concealed: &[Tile], melds: &[Meld]) -> bool {
    shanten(concealed, melds) == 0
}

fn max_melds_partials(tiles: &[Tile], max_sets: usize) -> (usize, usize) {
    let mut memo: HashMap<(Vec<Tile>, usize), (usize, usize)> = HashMap::new();
    best_mp(tiles, max_sets, &mut memo)
}

fn best_mp(
    tiles: &[Tile],
    max_sets: usize,
    memo: &mut HashMap<(Vec<Tile>, usize), (usize, usize)>,
) -> (usize, usize) {
    if tiles.is_empty() || max_sets == 0 {
        return (0, 0);
    }

    let key = (tiles.to_vec(), max_sets);
    if let Some(&cached) = memo.get(&key) {
        return cached;
    }

    let first = tiles[0];
    let same = tiles.iter().take_while(|&&t| t == first).count();

    let score = |m: usize, p: usize| 2 * m + p;
    let mut best = (0usize, 0usize);

    {
        let s = best_mp(&tiles[1..], max_sets, memo);
        if score(s.0, s.1) > score(best.0, best.1) {
            best = s;
        }
    }

    if same >= 3 {
        let mut rest = tiles.to_vec();
        rest.drain(0..3);
        let s = best_mp(&rest, max_sets - 1, memo);
        let cand = (s.0 + 1, s.1);
        if score(cand.0, cand.1) > score(best.0, best.1) {
            best = cand;
        }
    }

    if first.rank <= 7 {
        let t2 = Tile::new(first.suit, first.rank + 1);
        let t3 = Tile::new(first.suit, first.rank + 2);
        let pos2 = tiles.iter().position(|&t| t == t2);
        let pos3 = tiles.iter().position(|&t| t == t3);
        if let (Some(i2), Some(i3)) = (pos2, pos3) {
            let mut rest = tiles.to_vec();
            rest.remove(i3);
            rest.remove(i2);
            rest.remove(0);
            let s = best_mp(&rest, max_sets - 1, memo);
            let cand = (s.0 + 1, s.1);
            if score(cand.0, cand.1) > score(best.0, best.1) {
                best = cand;
            }
        }
    }

    if same >= 2 {
        let mut rest = tiles.to_vec();
        rest.drain(0..2);
        let s = best_mp(&rest, max_sets - 1, memo);
        let cand = (s.0, s.1 + 1);
        if score(cand.0, cand.1) > score(best.0, best.1) {
            best = cand;
        }
    }

    if first.rank <= 8 {
        let t2 = Tile::new(first.suit, first.rank + 1);
        if let Some(i2) = tiles.iter().position(|&t| t == t2) {
            let mut rest = tiles.to_vec();
            rest.remove(i2);
            rest.remove(0);
            let s = best_mp(&rest, max_sets - 1, memo);
            let cand = (s.0, s.1 + 1);
            if score(cand.0, cand.1) > score(best.0, best.1) {
                best = cand;
            }
        }
    }

    memo.insert(key, best);
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{GangSource, Suit, Tile};

    fn t(suit: Suit, rank: u8) -> Tile {
        Tile::new(suit, rank)
    }

    fn w(r: u8) -> Tile { t(Suit::Wan, r) }
    fn ti(r: u8) -> Tile { t(Suit::Tiao, r) }
    fn d(r: u8) -> Tile { t(Suit::Tong, r) }

    fn sorted(mut v: Vec<Tile>) -> Vec<Tile> {
        v.sort();
        v
    }

    #[test]
    fn test_winning_hand_three_sequences_triplet_pair() {
        let hand = sorted(vec![
            w(1), w(2), w(3),
            w(4), w(5), w(6),
            w(7), w(8), w(9),
            ti(1), ti(1), ti(1),
            d(2), d(2),
        ]);
        assert!(is_winning_hand(&hand, &[]));
    }

    #[test]
    fn test_winning_hand_all_triplets() {
        let hand = sorted(vec![
            w(1), w(1), w(1),
            w(3), w(3), w(3),
            ti(5), ti(5), ti(5),
            d(9), d(9), d(9),
            ti(7), ti(7),
        ]);
        assert!(is_winning_hand(&hand, &[]));
    }

    #[test]
    fn test_winning_hand_with_peng_and_gang() {
        let concealed = sorted(vec![w(1), w(2), w(3), d(2), d(2)]);
        let melds = vec![
            Meld::Peng { tile: ti(5), from: 1 },
            Meld::Peng { tile: ti(9), from: 2 },
            Meld::Gang { tile: d(1), from: GangSource::An },
        ];
        assert!(is_winning_hand(&concealed, &melds));
    }

    #[test]
    fn test_winning_hand_rejects_wrong_length() {
        let hand = sorted(vec![
            w(1), w(2), w(3), w(4), w(5), w(6), w(7), w(8), w(9),
            ti(1), ti(1), ti(1), d(2),
        ]);
        assert!(!is_winning_hand(&hand, &[]));
    }

    #[test]
    fn test_winning_hand_rejects_no_pair() {
        let hand = sorted(vec![
            w(1), w(2), w(3),
            w(4), w(5), w(6),
            w(7), w(8), w(9),
            ti(1), ti(2), ti(3),
            d(2), d(3),
        ]);
        assert!(!is_winning_hand(&hand, &[]));
    }

    #[test]
    fn test_winning_hand_rejects_garbage() {
        let hand = sorted(vec![
            w(1), w(4), w(7), ti(1), ti(4), ti(7),
            d(1), d(4), d(7), w(9), ti(9), d(9), w(2), d(5),
        ]);
        assert!(!is_winning_hand(&hand, &[]));
    }

    #[test]
    fn test_winning_hand_accepts_all_melds_called() {
        let concealed = sorted(vec![d(3), d(3)]);
        let melds = vec![
            Meld::Peng { tile: w(1), from: 1 },
            Meld::Peng { tile: ti(5), from: 2 },
            Meld::Peng { tile: d(9), from: 3 },
            Meld::Gang { tile: ti(7), from: GangSource::Ming { from: 1 } },
        ];
        assert!(is_winning_hand(&concealed, &melds));
    }

    #[test]
    fn test_shanten_complete_hand_is_minus_one() {
        let hand = sorted(vec![
            w(1), w(2), w(3),
            w(4), w(5), w(6),
            w(7), w(8), w(9),
            ti(1), ti(1), ti(1),
            d(2), d(2),
        ]);
        assert_eq!(shanten(&hand, &[]), -1);
    }

    #[test]
    fn test_shanten_tenpai_waiting_on_pair() {
        let hand = sorted(vec![
            w(1), w(2), w(3),
            w(4), w(5), w(6),
            w(7), w(8), w(9),
            ti(1), ti(1), ti(1),
            d(2),
        ]);
        assert_eq!(shanten(&hand, &[]), 0);
        assert!(is_tenpai(&hand, &[]));
    }

    #[test]
    fn test_shanten_tenpai_waiting_on_sequence_tile() {
        let hand = sorted(vec![
            w(1), w(2), w(3),
            w(4), w(5), w(6),
            w(7), w(8),
            ti(1), ti(1), ti(1),
            d(1), d(1),
        ]);
        assert_eq!(shanten(&hand, &[]), 0);
    }

    #[test]
    fn test_shanten_one_shanten() {
        let hand = sorted(vec![
            w(1), w(2), w(3),
            w(4), w(5), w(6),
            w(7), w(8),
            ti(1), ti(1), ti(1),
            d(1), d(4),
        ]);
        assert_eq!(shanten(&hand, &[]), 1);
    }

    #[test]
    fn test_shanten_two_shanten() {
        let hand = sorted(vec![
            w(1), w(2), w(3),
            w(4), w(5), w(6),
            ti(1), ti(2), ti(3),
            w(7), w(9), d(1), d(4),
        ]);
        assert_eq!(shanten(&hand, &[]), 2);
    }

    #[test]
    fn test_shanten_garbage_is_high() {
        // Every tile has odd rank, so no two are adjacent within a suit.
        // All distinct, so no pairs and no triplets. 8-shanten.
        let hand = sorted(vec![
            w(1), w(3), w(5), w(7), w(9),
            ti(1), ti(3), ti(5), ti(7), ti(9),
            d(1), d(3), d(5),
        ]);
        assert_eq!(shanten(&hand, &[]), 8);
    }

    #[test]
    fn test_shanten_with_melds() {
        let concealed = sorted(vec![w(1), w(2), w(3), w(4), w(5), w(6), d(2)]);
        let melds = vec![
            Meld::Peng { tile: ti(1), from: 1 },
            Meld::Peng { tile: ti(5), from: 2 },
        ];
        assert_eq!(shanten(&concealed, &melds), 0);
    }

    #[test]
    fn test_tenpai_then_draw_completes_hand() {
        let mut hand = sorted(vec![
            w(1), w(2), w(3),
            w(4), w(5), w(6),
            w(7), w(8), w(9),
            ti(1), ti(1), ti(1),
            d(2),
        ]);
        assert!(is_tenpai(&hand, &[]));
        hand.push(d(2));
        hand.sort();
        assert!(is_winning_hand(&hand, &[]));
    }

    #[test]
    fn test_shanten_never_less_than_minus_one() {
        let hand = sorted(vec![
            w(1), w(1), w(1), w(1),
            ti(1), ti(1), ti(1),
            d(1), d(1), d(1),
            w(2), w(3), w(4),
        ]);
        assert!(shanten(&hand, &[]) >= -1);
    }
}