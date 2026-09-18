//! Tiles, melds, and players. Pure data + tiny helpers, no macroquad.

/// The three numeric suits. No honors in Guiyang Mahjong.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Suit {
    Wan,
    Tiao,
    Tong,
}

impl Suit {
    pub const ALL: [Suit; 3] = [Suit::Wan, Suit::Tiao, Suit::Tong];

    pub fn label(self) -> &'static str {
        match self {
            Suit::Wan => "W",
            Suit::Tiao => "T",
            Suit::Tong => "D",
        }
    }
}

/// A single tile. Ranks are always in `1..=9`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Tile {
    pub suit: Suit,
    pub rank: u8,
}

impl Tile {
    pub fn new(suit: Suit, rank: u8) -> Self {
        debug_assert!((1..=9).contains(&rank), "tile rank out of range: {rank}");
        Self { suit, rank }
    }

    pub fn label(self) -> String {
        format!("{}{}", self.rank, self.suit.label())
    }
}

/// How a Gang was formed (the "Dou" type in Guiyang scoring).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GangSource {
    An,
    Bu,
    Ming { from: usize },
}

/// A player's exposed set. No Chi in Guiyang Mahjong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Meld {
    Peng { tile: Tile, from: usize },
    Gang { tile: Tile, from: GangSource },
}

impl Meld {
    pub fn tile_count(self) -> usize {
        match self {
            Meld::Peng { .. } => 3,
            Meld::Gang { .. } => 4,
        }
    }
}

/// A claim a player can make on another player's discard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimKind {
    Peng,
    /// Ming Gang (点豆) — claim a discard to complete a quad.
    Gang,
    Hu,
}

/// One of the four seats.
///
/// `concealed` always holds the "base" hand (13 - 3*melds tiles, sorted).
/// `drawn` holds the 14th tile only while it is this player's turn.
/// The drawn tile is rendered separately, on the right, with a gap.
#[derive(Debug, Clone, Default)]
pub struct Player {
    pub concealed: Vec<Tile>,
    pub drawn: Option<Tile>,
    pub melds: Vec<Meld>,
    pub discards: Vec<Tile>,
    pub is_ai: bool,
    pub score: i32,
}

impl Player {
    pub fn new(is_ai: bool) -> Self {
        Self { is_ai, ..Default::default() }
    }

    /// Merged and sorted view of concealed + drawn. Used by all
    /// tile-counting logic (win detection, shanten, AI decisions).
    pub fn all_tiles(&self) -> Vec<Tile> {
        let mut v = self.concealed.clone();
        if let Some(t) = self.drawn {
            v.push(t);
            v.sort();
        }
        v
    }

    /// Total held tiles (concealed + drawn + melds).
    pub fn tile_count(&self) -> usize {
        self.concealed.len()
            + self.drawn.is_some() as usize
            + self.melds.iter().map(|m| m.tile_count()).sum::<usize>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_label() {
        assert_eq!(Tile::new(Suit::Wan, 1).label(), "1W");
        assert_eq!(Tile::new(Suit::Tiao, 5).label(), "5T");
        assert_eq!(Tile::new(Suit::Tong, 9).label(), "9D");
    }

    #[test]
    fn test_tile_ordering_groups_by_suit() {
        let mut tiles = [
            Tile::new(Suit::Tong, 1),
            Tile::new(Suit::Wan, 9),
            Tile::new(Suit::Wan, 1),
            Tile::new(Suit::Tiao, 5),
        ];
        tiles.sort();
        assert_eq!(tiles[0], Tile::new(Suit::Wan, 1));
        assert_eq!(tiles[1], Tile::new(Suit::Wan, 9));
        assert_eq!(tiles[2], Tile::new(Suit::Tiao, 5));
        assert_eq!(tiles[3], Tile::new(Suit::Tong, 1));
    }

    #[test]
    fn test_suit_all_covers_three() {
        assert_eq!(Suit::ALL.len(), 3);
    }

    #[test]
    fn test_meld_tile_count() {
        assert_eq!(Meld::Peng { tile: Tile::new(Suit::Wan, 1), from: 1 }.tile_count(), 3);
        assert_eq!(
            Meld::Gang { tile: Tile::new(Suit::Tiao, 5), from: GangSource::An }.tile_count(),
            4
        );
    }

    #[test]
    fn test_player_tile_count() {
        let mut p = Player::new(false);
        p.concealed = vec![Tile::new(Suit::Wan, 1); 13];
        assert_eq!(p.tile_count(), 13);
        p.drawn = Some(Tile::new(Suit::Wan, 2));
        assert_eq!(p.tile_count(), 14);
        p.melds.push(Meld::Peng { tile: Tile::new(Suit::Tiao, 5), from: 1 });
        assert_eq!(p.tile_count(), 17);
    }

    #[test]
    fn test_player_all_tiles_merges_drawn() {
        let mut p = Player::new(false);
        p.concealed = vec![
            Tile::new(Suit::Wan, 1),
            Tile::new(Suit::Wan, 3),
        ];
        p.drawn = Some(Tile::new(Suit::Wan, 2));
        let all = p.all_tiles();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0], Tile::new(Suit::Wan, 1));
        assert_eq!(all[1], Tile::new(Suit::Wan, 2));
        assert_eq!(all[2], Tile::new(Suit::Wan, 3));
    }

    #[test]
    fn test_player_new_is_ai() {
        assert!(Player::new(true).is_ai);
        assert!(!Player::new(false).is_ai);
    }

    #[test]
    fn test_claim_kind_is_copy_and_eq() {
        let a = ClaimKind::Peng;
        let b = a;
        assert_eq!(a, b);
        assert_ne!(ClaimKind::Peng, ClaimKind::Hu);
        assert_ne!(ClaimKind::Gang, ClaimKind::Hu);
    }
}