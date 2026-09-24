//! Snapshot of the table state at the end of a hand.
//! Used by the "Table" tab of the hand-end modal.

use crate::components::{Meld, Tile};
use crate::scoring::{self, DouScore, JiInfo};
use crate::systems::NUM_PLAYERS;

#[derive(Debug, Clone)]
pub struct HandSnapshot {
    pub concealed: [Vec<Tile>; NUM_PLAYERS],
    pub melds: [Vec<Meld>; NUM_PLAYERS],
    pub discards: [Vec<Tile>; NUM_PLAYERS],
    pub ji_info: JiInfo,
    pub dou: [DouScore; NUM_PLAYERS],
    pub ji_count: [i32; NUM_PLAYERS],
}

impl HandSnapshot {
    pub fn capture(
        concealed: &[Vec<Tile>; NUM_PLAYERS],
        melds: &[Vec<Meld>; NUM_PLAYERS],
        discards: &[Vec<Tile>; NUM_PLAYERS],
        ji_info: JiInfo,
    ) -> Self {
        let dou = std::array::from_fn(|i| scoring::count_dou(&melds[i]));
        let ji_count = std::array::from_fn(|i| {
            let mut fan = 0;
            for &t in concealed[i].iter().chain(discards[i].iter()) {
                fan += scoring::ji_fan_for_tile(t, &ji_info);
            }
            for m in &melds[i] {
                let tile = scoring::meld_tile(m);
                let copies = if matches!(m, Meld::Gang { .. }) { 4 } else { 3 };
                fan += copies * scoring::ji_fan_for_tile(tile, &ji_info);
            }
            fan
        });
        Self {
            concealed: concealed.clone(),
            melds: melds.clone(),
            discards: discards.clone(),
            ji_info,
            dou,
            ji_count,
        }
    }

    pub fn is_ji(&self, t: Tile) -> bool {
        scoring::ji_fan_for_tile(t, &self.ji_info) > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{GangSource, Suit};

    fn w(r: u8) -> Tile { Tile::new(Suit::Wan, r) }
    fn ti(r: u8) -> Tile { Tile::new(Suit::Tiao, r) }

    #[test]
    fn test_empty_snapshot_has_no_ji_or_dou() {
        let concealed: [Vec<Tile>; NUM_PLAYERS] = std::array::from_fn(|_| Vec::new());
        let melds: [Vec<Meld>; NUM_PLAYERS] = std::array::from_fn(|_| Vec::new());
        let discards: [Vec<Tile>; NUM_PLAYERS] = std::array::from_fn(|_| Vec::new());
        let snap = HandSnapshot::capture(&concealed, &melds, &discards, JiInfo::none());
        for i in 0..NUM_PLAYERS {
            assert_eq!(snap.ji_count[i], 0);
            assert!(snap.dou[i].is_empty());
        }
    }

    #[test]
    fn test_ji_counted_from_concealed() {
        let mut concealed: [Vec<Tile>; NUM_PLAYERS] = std::array::from_fn(|_| Vec::new());
        // 2x 1 Tiao (permanent Ji), 1x 7 Wan (not a Ji).
        concealed[0] = vec![ti(1), ti(1), w(7)];
        let melds: [Vec<Meld>; NUM_PLAYERS] = std::array::from_fn(|_| Vec::new());
        let discards: [Vec<Tile>; NUM_PLAYERS] = std::array::from_fn(|_| Vec::new());
        let ji = scoring::determine_ji(w(4));
        let snap = HandSnapshot::capture(&concealed, &melds, &discards, ji);
        assert_eq!(snap.ji_count[0], 2);
        assert_eq!(snap.ji_count[1], 0);
    }

    #[test]
    fn test_ji_counted_from_river() {
        let concealed: [Vec<Tile>; NUM_PLAYERS] = std::array::from_fn(|_| Vec::new());
        let melds: [Vec<Meld>; NUM_PLAYERS] = std::array::from_fn(|_| Vec::new());
        let mut discards: [Vec<Tile>; NUM_PLAYERS] = std::array::from_fn(|_| Vec::new());
        discards[1] = vec![ti(1)];
        let snap = HandSnapshot::capture(&concealed, &melds, &discards, JiInfo::none());
        assert_eq!(snap.ji_count[1], 1);
    }

    #[test]
    fn test_ji_counted_from_gang_meld() {
        let concealed: [Vec<Tile>; NUM_PLAYERS] = std::array::from_fn(|_| Vec::new());
        let mut melds: [Vec<Meld>; NUM_PLAYERS] = std::array::from_fn(|_| Vec::new());
        melds[2] = vec![Meld::Gang { tile: ti(1), from: GangSource::An }];
        let discards: [Vec<Tile>; NUM_PLAYERS] = std::array::from_fn(|_| Vec::new());
        let snap = HandSnapshot::capture(&concealed, &melds, &discards, JiInfo::none());
        assert_eq!(snap.ji_count[2], 4);
    }

    #[test]
    fn test_dou_counted_from_melds() {
        let concealed: [Vec<Tile>; NUM_PLAYERS] = std::array::from_fn(|_| Vec::new());
        let mut melds: [Vec<Meld>; NUM_PLAYERS] = std::array::from_fn(|_| Vec::new());
        melds[0] = vec![
            Meld::Gang { tile: w(1), from: GangSource::An },
            Meld::Gang { tile: w(2), from: GangSource::Bu },
            Meld::Gang { tile: w(3), from: GangSource::Ming { from: 2 } },
        ];
        let discards: [Vec<Tile>; NUM_PLAYERS] = std::array::from_fn(|_| Vec::new());
        let snap = HandSnapshot::capture(&concealed, &melds, &discards, JiInfo::none());
        assert_eq!(snap.dou[0].an, 1);
        assert_eq!(snap.dou[0].bu, 1);
        assert_eq!(snap.dou[0].ming, 1);
    }

    #[test]
    fn test_is_ji_detects_permanent_ji() {
        let concealed: [Vec<Tile>; NUM_PLAYERS] = std::array::from_fn(|_| Vec::new());
        let melds: [Vec<Meld>; NUM_PLAYERS] = std::array::from_fn(|_| Vec::new());
        let discards: [Vec<Tile>; NUM_PLAYERS] = std::array::from_fn(|_| Vec::new());
        let snap = HandSnapshot::capture(&concealed, &melds, &discards, JiInfo::none());
        assert!(snap.is_ji(ti(1)));
        assert!(snap.is_ji(Tile::new(Suit::Tong, 8)));
        assert!(!snap.is_ji(w(5)));
    }
}