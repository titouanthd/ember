//! Hand-end scoring: pattern detection, Dou (Gang bonuses), and Ji
//! (Chicken) scoring, per DESIGN.md §7 and the extended rule recap.

use std::collections::HashMap;

use crate::components::{GangSource, Meld, Player, Suit, Tile};
use crate::systems::{HuMethod, NUM_PLAYERS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pattern {
    PingHu,
    DaDuiZi,
    QiDui,
    QingYiSe,
    QingDaDui,
    LongQiDui,
    QingQiDui,
    QingLongBei,
}

impl Pattern {
    pub fn fan(self) -> i32 {
        match self {
            Pattern::PingHu => 1,
            Pattern::DaDuiZi => 5,
            Pattern::QiDui => 10,
            Pattern::QingYiSe => 10,
            Pattern::QingDaDui => 15,
            Pattern::LongQiDui => 20,
            Pattern::QingQiDui => 20,
            Pattern::QingLongBei => 30,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Pattern::PingHu => "PingHu",
            Pattern::DaDuiZi => "DaDuiZi",
            Pattern::QiDui => "QiDui",
            Pattern::QingYiSe => "QingYiSe",
            Pattern::QingDaDui => "QingDaDui",
            Pattern::LongQiDui => "LongQiDui",
            Pattern::QingQiDui => "QingQiDui",
            Pattern::QingLongBei => "QingLongBei",
        }
    }
}

/// Per-player decomposition of a hand's score change, split into the
/// three contributing buckets. Used by the hand-end modal.
#[derive(Debug, Clone, Copy, Default)]
pub struct HandBreakdown {
    pub hu: [i32; NUM_PLAYERS],
    pub dou: [i32; NUM_PLAYERS],
    pub ji: [i32; NUM_PLAYERS],
}

impl HandBreakdown {
    pub fn total(&self, player: usize) -> i32 {
        self.hu[player] + self.dou[player] + self.ji[player]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SevenPairsKind {
    Standard,
    Long,
}

fn detect_seven_pairs(concealed: &[Tile], melds: &[Meld]) -> Option<SevenPairsKind> {
    if !melds.is_empty() {
        return None;
    }
    if concealed.len() != 14 {
        return None;
    }
    let mut counts: HashMap<Tile, usize> = HashMap::new();
    for &t in concealed {
        *counts.entry(t).or_insert(0) += 1;
    }
    let mut quads = 0;
    for &c in counts.values() {
        match c {
            2 => {}
            4 => quads += 1,
            _ => return None,
        }
    }
    Some(if quads == 0 {
        SevenPairsKind::Standard
    } else {
        SevenPairsKind::Long
    })
}

pub fn detect_pattern(concealed: &[Tile], melds: &[Meld]) -> Pattern {
    let one_suit = is_all_one_suit(concealed, melds);

    if let Some(sp) = detect_seven_pairs(concealed, melds) {
        return match (sp, one_suit) {
            (SevenPairsKind::Standard, false) => Pattern::QiDui,
            (SevenPairsKind::Standard, true) => Pattern::QingQiDui,
            (SevenPairsKind::Long, false) => Pattern::LongQiDui,
            (SevenPairsKind::Long, true) => Pattern::QingLongBei,
        };
    }

    let all_triplets = is_all_triplets_hand(concealed, melds);

    if one_suit && all_triplets {
        Pattern::QingDaDui
    } else if one_suit {
        Pattern::QingYiSe
    } else if all_triplets {
        Pattern::DaDuiZi
    } else {
        Pattern::PingHu
    }
}

fn is_all_one_suit(concealed: &[Tile], melds: &[Meld]) -> bool {
    let mut suit: Option<Suit> = None;
    let mut check = |t: Tile| -> bool {
        match suit {
            None => {
                suit = Some(t.suit);
                true
            }
            Some(s) => s == t.suit,
        }
    };
    for &t in concealed {
        if !check(t) {
            return false;
        }
    }
    for m in melds {
        if !check(meld_tile(m)) {
            return false;
        }
    }
    suit.is_some()
}

fn is_all_triplets_hand(concealed: &[Tile], melds: &[Meld]) -> bool {
    let needed = 4usize.saturating_sub(melds.len());
    if concealed.len() != 3 * needed + 2 {
        return false;
    }

    let mut sorted = concealed.to_vec();
    sorted.sort();

    let mut i = 0;
    while i < sorted.len() {
        let t = sorted[i];
        let count = sorted[i..].iter().take_while(|&&u| u == t).count();
        if count >= 2 {
            let mut rest = sorted.clone();
            rest.remove(i);
            rest.remove(i);
            if can_form_triplets_only(&rest, needed) {
                return true;
            }
        }
        i += count;
    }
    false
}

fn can_form_triplets_only(tiles: &[Tile], needed: usize) -> bool {
    if tiles.is_empty() {
        return needed == 0;
    }
    if needed == 0 {
        return false;
    }
    let first = tiles[0];
    let same = tiles.iter().take_while(|&&t| t == first).count();
    if same < 3 {
        return false;
    }
    let mut rest = tiles.to_vec();
    rest.drain(0..3);
    can_form_triplets_only(&rest, needed - 1)
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DouScore {
    pub an: u32,
    pub bu: u32,
    pub ming: u32,
}

impl DouScore {
    pub fn fan_from_each(self) -> i32 {
        (self.an as i32) * 2 + (self.bu as i32) * 3
    }

    pub fn fan_from_discarder(self) -> i32 {
        self.ming as i32
    }

    pub fn is_empty(self) -> bool {
        self.an == 0 && self.bu == 0 && self.ming == 0
    }
}

pub fn count_dou(melds: &[Meld]) -> DouScore {
    let mut d = DouScore::default();
    for m in melds {
        if let Meld::Gang { from, .. } = m {
            match from {
                GangSource::An => d.an += 1,
                GangSource::Bu => d.bu += 1,
                GangSource::Ming { .. } => d.ming += 1,
            }
        }
    }
    d
}

#[derive(Debug, Clone, Copy)]
pub struct JiInfo {
    pub primary: Tile,
    pub jiao_golden: bool,
    pub ba_tong_golden: bool,
}

impl JiInfo {
    pub const fn none() -> Self {
        Self {
            primary: Tile { suit: Suit::Wan, rank: 0 },
            jiao_golden: false,
            ba_tong_golden: false,
        }
    }
}

pub fn determine_ji(flipped: Tile) -> JiInfo {
    let primary = next_in_sequence(flipped);
    JiInfo {
        primary,
        jiao_golden: primary == Tile::new(Suit::Tiao, 1),
        ba_tong_golden: primary == Tile::new(Suit::Tong, 8),
    }
}

fn next_in_sequence(t: Tile) -> Tile {
    if t.rank == 9 {
        Tile::new(t.suit, 1)
    } else {
        Tile::new(t.suit, t.rank + 1)
    }
}

pub fn count_ji_for_player(player: &Player, ji: &JiInfo) -> i32 {
    let mut fan = 0;
    for &t in player.concealed.iter().chain(player.discards.iter()) {
        fan += ji_fan_for_tile(t, ji);
    }
    for m in &player.melds {
        let tile = meld_tile(m);
        let copies = if matches!(m, Meld::Gang { .. }) { 4 } else { 3 };
        fan += copies * ji_fan_for_tile(tile, ji);
    }
    fan
}

pub fn ji_fan_for_tile(tile: Tile, ji: &JiInfo) -> i32 {
    let jiao = Tile::new(Suit::Tiao, 1);
    let ba_tong = Tile::new(Suit::Tong, 8);

    if tile == jiao {
        if ji.jiao_golden { 2 } else { 1 }
    } else if tile == ba_tong {
        if ji.ba_tong_golden { 2 } else { 1 }
    } else if tile == ji.primary {
        1
    } else {
        0
    }
}

/// Apply the full hand-end scoring and return a per-player breakdown
/// splitting the delta into Hu (pattern), Dou (gangs), and Ji buckets.
pub fn apply_hand_scores(
    players: &mut [Player; NUM_PLAYERS],
    winner: Option<(usize, HuMethod)>,
    ji: &JiInfo,
    dealer: usize,
    first_discard: Option<(usize, Tile)>,
) -> HandBreakdown {
    let mut breakdown = HandBreakdown::default();

    if let Some((idx, method)) = winner {
        let concealed = players[idx].concealed.clone();
        let melds = players[idx].melds.clone();

        let pattern = detect_pattern(&concealed, &melds);
        let dou = count_dou(&melds);
        let is_zimo = matches!(method, HuMethod::Zimo);
        let is_dealer = idx == dealer;

        let mult = (if is_zimo { 2 } else { 1 }) * (if is_dealer { 2 } else { 1 });
        let hu_portion = pattern.fan() * mult;
        let dou_portion =
            (dou.fan_from_each() * (NUM_PLAYERS as i32 - 1) + dou.fan_from_discarder()) * mult;

        match method {
            HuMethod::Zimo => {
                breakdown.hu[idx] = (NUM_PLAYERS as i32 - 1) * hu_portion;
                breakdown.dou[idx] = (NUM_PLAYERS as i32 - 1) * dou_portion;
                for i in 0..NUM_PLAYERS {
                    if i != idx {
                        breakdown.hu[i] -= hu_portion;
                        breakdown.dou[i] -= dou_portion;
                    }
                }
            }
            HuMethod::Hu { from } => {
                breakdown.hu[idx] += hu_portion;
                breakdown.hu[from] -= hu_portion;
                breakdown.dou[idx] += dou_portion;
                breakdown.dou[from] -= dou_portion;
            }
        }
    }

    // ─── Ji (chicken) balance ────────────────────────────────────────
    let ji_counts: Vec<i32> = players
        .iter()
        .map(|p| count_ji_for_player(p, ji))
        .collect();
    let total_ji: i32 = ji_counts.iter().sum();
    for (i, &count) in ji_counts.iter().enumerate() {
        breakdown.ji[i] = 4 * count - total_ji;
    }

    // ─── Chong feng ji / Ze ren ji ────────────────────────────────────
    if let Some((discarder, tile)) = first_discard
        && tile == Tile::new(Suit::Tiao, 1)
    {
        let chong_feng_bonus = if ji.jiao_golden { 2 } else { 1 };
        for i in 0..NUM_PLAYERS {
            if i != discarder {
                breakdown.ji[i] -= chong_feng_bonus;
                breakdown.ji[discarder] += chong_feng_bonus;
            }
        }

        let was_claimed = players
            .iter()
            .any(|p| p.melds.iter().any(|m| meld_tile(m) == tile));
        if was_claimed {
            for i in 0..NUM_PLAYERS {
                if i != discarder {
                    breakdown.ji[discarder] -= 1;
                    breakdown.ji[i] += 1;
                }
            }
        }
    }

    // Apply all buckets to the players' scores.
    for (i, p) in players.iter_mut().enumerate() {
        p.score += breakdown.hu[i] + breakdown.dou[i] + breakdown.ji[i];
    }

    breakdown
}

pub fn apply_huangzhuang_scores(
    players: &mut [Player; NUM_PLAYERS],
    tenpai: &[bool; NUM_PLAYERS],
) {
    let tenpai_count = tenpai.iter().filter(|&&b| b).count();
    if tenpai_count == 0 || tenpai_count == NUM_PLAYERS {
        return;
    }
    const NOMINAL: i32 = 2;
    for i in 0..NUM_PLAYERS {
        if !tenpai[i] {
            continue;
        }
        for j in 0..NUM_PLAYERS {
            if !tenpai[j] {
                players[j].score -= NOMINAL;
                players[i].score += NOMINAL;
            }
        }
    }
}

pub fn meld_tile(m: &Meld) -> Tile {
    match *m {
        Meld::Peng { tile, .. } => tile,
        Meld::Gang { tile, .. } => tile,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{GangSource, Suit, Tile};

    fn w(r: u8) -> Tile { Tile::new(Suit::Wan, r) }
    fn ti(r: u8) -> Tile { Tile::new(Suit::Tiao, r) }
    fn d(r: u8) -> Tile { Tile::new(Suit::Tong, r) }
    fn sorted(mut v: Vec<Tile>) -> Vec<Tile> { v.sort(); v }

    #[test]
    fn test_ping_hu_mixed_suits_sequences() {
        let hand = sorted(vec![
            w(1), w(2), w(3),
            w(4), w(5), w(6),
            ti(1), ti(2), ti(3),
            d(7), d(8), d(9),
            d(5), d(5),
        ]);
        assert_eq!(detect_pattern(&hand, &[]), Pattern::PingHu);
    }

    #[test]
    fn test_da_dui_zi_all_triplets() {
        let hand = sorted(vec![
            w(1), w(1), w(1),
            w(3), w(3), w(3),
            ti(5), ti(5), ti(5),
            d(9), d(9), d(9),
            ti(7), ti(7),
        ]);
        assert_eq!(detect_pattern(&hand, &[]), Pattern::DaDuiZi);
    }

    #[test]
    fn test_qing_yi_se_one_suit_with_sequences() {
        let hand = sorted(vec![
            w(1), w(2), w(3),
            w(4), w(5), w(6),
            w(7), w(8), w(9),
            w(1), w(1), w(1),
            w(2), w(2),
        ]);
        assert_eq!(detect_pattern(&hand, &[]), Pattern::QingYiSe);
    }

    #[test]
    fn test_qing_da_dui_all_triplets_one_suit() {
        let hand = sorted(vec![
            w(1), w(1), w(1),
            w(2), w(2), w(2),
            w(3), w(3), w(3),
            w(4), w(4), w(4),
            w(5), w(5),
        ]);
        assert_eq!(detect_pattern(&hand, &[]), Pattern::QingDaDui);
    }

    #[test]
    fn test_qi_dui_seven_pairs_mixed() {
        let hand = sorted(vec![
            w(1), w(1), w(2), w(2), w(3), w(3),
            ti(4), ti(4), ti(5), ti(5),
            d(7), d(7), d(9), d(9),
        ]);
        assert_eq!(detect_pattern(&hand, &[]), Pattern::QiDui);
    }

    #[test]
    fn test_qing_qi_dui_one_suit_seven_pairs() {
        let hand = sorted(vec![
            w(1), w(1), w(2), w(2), w(3), w(3),
            w(4), w(4), w(5), w(5), w(7), w(7), w(9), w(9),
        ]);
        assert_eq!(detect_pattern(&hand, &[]), Pattern::QingQiDui);
    }

    #[test]
    fn test_long_qi_dui_five_pairs_and_quad() {
        let hand = sorted(vec![
            w(1), w(1), w(1), w(1),
            w(2), w(2), w(3), w(3), w(4), w(4),
            ti(5), ti(5), d(7), d(7),
        ]);
        assert_eq!(detect_pattern(&hand, &[]), Pattern::LongQiDui);
    }

    #[test]
    fn test_qing_long_bei_one_suit_long_qi_dui() {
        let hand = sorted(vec![
            w(1), w(1), w(1), w(1),
            w(2), w(2), w(3), w(3), w(4), w(4),
            w(5), w(5), w(7), w(7),
        ]);
        assert_eq!(detect_pattern(&hand, &[]), Pattern::QingLongBei);
    }

    #[test]
    fn test_qi_dui_priority_over_ping_hu() {
        let hand = sorted(vec![
            w(1), w(1), w(2), w(2), w(3), w(3), w(4),
            w(4), w(5), w(5), w(6), w(6), w(7), w(7),
        ]);
        assert_eq!(detect_pattern(&hand, &[]), Pattern::QingQiDui);
    }

    #[test]
    fn test_pattern_with_pengs() {
        let concealed = sorted(vec![
            w(1), w(2), w(3),
            w(4), w(5), w(6),
            d(5), d(5),
        ]);
        let melds = vec![
            Meld::Peng { tile: ti(1), from: 1 },
            Meld::Peng { tile: ti(5), from: 2 },
        ];
        assert_eq!(detect_pattern(&concealed, &melds), Pattern::PingHu);
    }

    #[test]
    fn test_dou_empty() {
        let melds = vec![Meld::Peng { tile: w(1), from: 0 }];
        let d = count_dou(&melds);
        assert!(d.is_empty());
    }

    #[test]
    fn test_dou_an_gang() {
        let melds = vec![Meld::Gang { tile: w(1), from: GangSource::An }];
        let d = count_dou(&melds);
        assert_eq!(d.an, 1);
        assert_eq!(d.fan_from_each(), 2);
    }

    #[test]
    fn test_dou_bu_gang() {
        let melds = vec![Meld::Gang { tile: w(1), from: GangSource::Bu }];
        let d = count_dou(&melds);
        assert_eq!(d.bu, 1);
        assert_eq!(d.fan_from_each(), 3);
    }

    #[test]
    fn test_dou_ming_gang() {
        let melds = vec![Meld::Gang { tile: w(1), from: GangSource::Ming { from: 2 } }];
        let d = count_dou(&melds);
        assert_eq!(d.ming, 1);
        assert_eq!(d.fan_from_discarder(), 1);
        assert_eq!(d.fan_from_each(), 0);
    }

    #[test]
    fn test_ji_flip_9_tiao_makes_jiao_golden() {
        let ji = determine_ji(Tile::new(Suit::Tiao, 9));
        assert_eq!(ji.primary, Tile::new(Suit::Tiao, 1));
        assert!(ji.jiao_golden);
        assert!(!ji.ba_tong_golden);
    }

    #[test]
    fn test_ji_flip_7_tong_makes_8_tong_golden() {
        let ji = determine_ji(Tile::new(Suit::Tong, 7));
        assert_eq!(ji.primary, Tile::new(Suit::Tong, 8));
        assert!(!ji.jiao_golden);
        assert!(ji.ba_tong_golden);
    }

    #[test]
    fn test_ji_flip_5_wan_makes_6_wan_primary() {
        let ji = determine_ji(Tile::new(Suit::Wan, 5));
        assert_eq!(ji.primary, Tile::new(Suit::Wan, 6));
        assert!(!ji.jiao_golden);
        assert!(!ji.ba_tong_golden);
    }

    #[test]
    fn test_ji_none_matches_nothing() {
        let ji = JiInfo::none();
        assert_eq!(ji_fan_for_tile(Tile::new(Suit::Wan, 5), &ji), 0);
        assert_eq!(ji_fan_for_tile(Tile::new(Suit::Wan, 1), &ji), 0);
        assert_eq!(ji_fan_for_tile(Tile::new(Suit::Wan, 9), &ji), 0);
    }

    #[test]
    fn test_ji_counting_basic() {
        let mut p = Player::new(false);
        p.concealed = vec![
            Tile::new(Suit::Tiao, 1),
            Tile::new(Suit::Tiao, 1),
            Tile::new(Suit::Wan, 5),
        ];
        let ji = determine_ji(Tile::new(Suit::Wan, 4));
        assert_eq!(count_ji_for_player(&p, &ji), 3);
    }

    #[test]
    fn test_ji_counting_golden() {
        let mut p = Player::new(false);
        p.concealed = vec![Tile::new(Suit::Tiao, 1)];
        let ji = determine_ji(Tile::new(Suit::Tiao, 9));
        assert_eq!(count_ji_for_player(&p, &ji), 2);
    }

    #[test]
    fn test_apply_hand_scores_zimo_pays_all() {
        let mut players = [
            Player::new(false),
            Player::new(true),
            Player::new(true),
            Player::new(true),
        ];
        players[0].concealed = sorted(vec![
            w(1), w(2), w(3),
            w(4), w(5), w(6),
            w(7), w(8), w(9),
            ti(1), ti(1), ti(1),
            d(5), d(5),
        ]);

        let ji = JiInfo::none();
        let _bd = apply_hand_scores(&mut players, Some((0, HuMethod::Zimo)), &ji, 1, None);

        assert!(players[0].score > 0);
        assert_eq!(players[1].score, players[2].score);
        assert_eq!(players[2].score, players[3].score);
        let sum: i32 = players.iter().map(|p| p.score).sum();
        assert_eq!(sum, 0);
    }

    #[test]
    fn test_apply_hand_scores_hu_pays_from_discarder() {
        let mut players = [
            Player::new(false),
            Player::new(true),
            Player::new(true),
            Player::new(true),
        ];
        players[0].concealed = sorted(vec![
            w(1), w(2), w(3),
            w(4), w(5), w(6),
            w(7), w(8), w(9),
            ti(3), ti(4), ti(5),
            d(5), d(5),
        ]);

        let ji = JiInfo::none();
        let _bd = apply_hand_scores(
            &mut players,
            Some((0, HuMethod::Hu { from: 2 })),
            &ji,
            1,
            None,
        );

        assert!(players[0].score > 0);
        assert!(players[2].score < 0);
        assert_eq!(players[1].score, 0);
        assert_eq!(players[3].score, 0);
    }

    #[test]
    fn test_apply_huangzhuang_tenpai_paid() {
        let mut players = [
            Player::new(false),
            Player::new(true),
            Player::new(true),
            Player::new(true),
        ];
        let tenpai = [true, false, false, false];
        apply_huangzhuang_scores(&mut players, &tenpai);
        assert_eq!(players[0].score, 6);
        assert_eq!(players[1].score, -2);
    }

    #[test]
    fn test_apply_huangzhuang_all_or_none_no_payment() {
        let mut players = [
            Player::new(false),
            Player::new(true),
            Player::new(true),
            Player::new(true),
        ];
        apply_huangzhuang_scores(&mut players, &[true, true, true, true]);
        for p in &players {
            assert_eq!(p.score, 0);
        }
    }

    #[test]
    fn test_chong_feng_ji_bonus_goes_to_discarder() {
        let mut players = [
            Player::new(false),
            Player::new(true),
            Player::new(true),
            Player::new(true),
        ];
        players[1].discards = vec![Tile::new(Suit::Tiao, 1)];

        let ji = JiInfo::none();
        let first = Some((1, Tile::new(Suit::Tiao, 1)));
        apply_hand_scores(&mut players, None, &ji, 0, first);

        assert_eq!(players[1].score, 6);
        assert_eq!(players[0].score, -2);
        assert_eq!(players[2].score, -2);
        assert_eq!(players[3].score, -2);
    }

    #[test]
    fn test_chong_feng_ji_golden_bonus() {
        let mut players = [
            Player::new(false),
            Player::new(true),
            Player::new(true),
            Player::new(true),
        ];
        players[1].discards = vec![Tile::new(Suit::Tiao, 1)];

        let ji = determine_ji(Tile::new(Suit::Tiao, 9));
        let first = Some((1, Tile::new(Suit::Tiao, 1)));
        apply_hand_scores(&mut players, None, &ji, 0, first);

        assert_eq!(players[1].score, 12);
        assert_eq!(players[0].score, -4);
    }

    #[test]
    fn test_ze_ren_ji_penalty_when_claimed() {
        let mut players = [
            Player::new(false),
            Player::new(true),
            Player::new(true),
            Player::new(true),
        ];
        players[2].melds = vec![Meld::Peng {
            tile: Tile::new(Suit::Tiao, 1),
            from: 1,
        }];

        let ji = JiInfo::none();
        let first = Some((1, Tile::new(Suit::Tiao, 1)));
        apply_hand_scores(&mut players, None, &ji, 0, first);

        assert_eq!(players[0].score, -3);
        assert_eq!(players[1].score, -3);
        assert_eq!(players[2].score, 9);
        assert_eq!(players[3].score, -3);
        let sum: i32 = players.iter().map(|p| p.score).sum();
        assert_eq!(sum, 0);
    }

    #[test]
    fn test_no_chong_feng_when_first_discard_is_not_1_tiao() {
        let mut players = [
            Player::new(false),
            Player::new(true),
            Player::new(true),
            Player::new(true),
        ];
        players[1].discards = vec![Tile::new(Suit::Tiao, 1)];

        let ji = JiInfo::none();
        let first = Some((1, Tile::new(Suit::Wan, 5)));
        apply_hand_scores(&mut players, None, &ji, 0, first);

        assert_eq!(players[1].score, 3);
        assert_eq!(players[0].score, -1);
    }

    #[test]
    fn test_no_chong_feng_without_first_discard() {
        let mut players = [
            Player::new(false),
            Player::new(true),
            Player::new(true),
            Player::new(true),
        ];
        players[1].discards = vec![Tile::new(Suit::Tiao, 1)];

        let ji = JiInfo::none();
        apply_hand_scores(&mut players, None, &ji, 0, None);

        assert_eq!(players[1].score, 3);
    }

    // ─── HandBreakdown ─────────────────────────────────────────────

    #[test]
    fn test_breakdown_total_matches_score_change() {
        let mut players = [
            Player::new(false),
            Player::new(true),
            Player::new(true),
            Player::new(true),
        ];
        players[0].concealed = sorted(vec![
            w(1), w(2), w(3),
            w(4), w(5), w(6),
            w(7), w(8), w(9),
            ti(3), ti(4), ti(5),
            d(5), d(5),
        ]);

        let ji = JiInfo::none();
        let bd = apply_hand_scores(
            &mut players,
            Some((0, HuMethod::Hu { from: 2 })),
            &ji,
            1,
            None,
        );

        for (i, p) in players.iter().enumerate() {
            assert_eq!(bd.total(i), p.score);
        }
    }

    #[test]
    fn test_breakdown_hu_portion_for_pinghu_hu() {
        // PingHu (1 fan), not dealer, Hu on discard → hu_portion = 1.
        // Winner +1, discarder -1, no dou.
        let mut players = [
            Player::new(false),
            Player::new(true),
            Player::new(true),
            Player::new(true),
        ];
        players[0].concealed = sorted(vec![
            w(1), w(2), w(3),
            w(4), w(5), w(6),
            w(7), w(8), w(9),
            ti(3), ti(4), ti(5),
            d(5), d(5),
        ]);

        let bd = apply_hand_scores(
            &mut players,
            Some((0, HuMethod::Hu { from: 2 })),
            &JiInfo::none(),
            1,
            None,
        );

        assert_eq!(bd.hu[0], 1);
        assert_eq!(bd.hu[2], -1);
        assert_eq!(bd.dou, [0; NUM_PLAYERS]);
    }

    #[test]
    fn test_breakdown_dou_portion_for_zimo_with_an_gang() {
        // An Gang (2 fan from each): total dou from each = 2.
        // Zimo, dealer=1, not dealer → mult=2.
        // dou_portion = (2 * 3 + 0) * 2 = 12.
        // Winner gets 3*12 = 36 in dou bucket, others -12 each.
        let mut players = [
            Player::new(false),
            Player::new(true),
            Player::new(true),
            Player::new(true),
        ];
        // Winning hand: 3 sequences + 1 gang meld + 1 pair
        players[0].concealed = sorted(vec![
            w(1), w(2), w(3),
            w(4), w(5), w(6),
            w(7), w(8), w(9),
            d(5), d(5),
        ]);
        players[0].melds = vec![Meld::Gang {
            tile: ti(1),
            from: GangSource::An,
        }];

        let bd = apply_hand_scores(
            &mut players,
            Some((0, HuMethod::Zimo)),
            &JiInfo::none(),
            1,
            None,
        );

        assert_eq!(bd.dou[0], 36);
        assert_eq!(bd.dou[1], -12);
        assert_eq!(bd.dou[2], -12);
        assert_eq!(bd.dou[3], -12);
    }

    #[test]
    fn test_breakdown_ji_bucket_matches_ji_counting() {
        // No winner, but player 1 has a 1T in river.
        let mut players = [
            Player::new(false),
            Player::new(true),
            Player::new(true),
            Player::new(true),
        ];
        players[1].discards = vec![Tile::new(Suit::Tiao, 1)];

        let bd = apply_hand_scores(
            &mut players,
            None,
            &JiInfo::none(),
            0,
            // first_discard = Some((1, 1T)) triggers chong feng
            Some((1, Tile::new(Suit::Tiao, 1))),
        );

        // Base ji: [-1, 3, -1, -1]
        // Chong feng: [-1, +3, -1, -1] added
        // Total ji bucket: [-2, 6, -2, -2]
        assert_eq!(bd.ji, [-2, 6, -2, -2]);
        assert_eq!(bd.hu, [0; NUM_PLAYERS]);
        assert_eq!(bd.dou, [0; NUM_PLAYERS]);
    }
}