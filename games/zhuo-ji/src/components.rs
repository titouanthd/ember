//! Tiles, melds, and players. Pure data + tiny helpers, no macroquad.

/// The three numeric suits. No honors in Guiyang Mahjong.
///
/// `Ord` is derived so that `Tile` sorts by `(suit, rank)`, which is
/// the natural grouping for hand evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Suit {
    Wan,  // 万 — characters
    Tiao, // 条 — bamboo
    Tong, // 筒 — dots
}

impl Suit {
    pub const ALL: [Suit; 3] = [Suit::Wan, Suit::Tiao, Suit::Tong];

    /// Single-letter ASCII label used by the V1 renderer.
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

    /// Short ASCII label like `1W`, `5T`, `9D`.
    pub fn label(self) -> String {
        format!("{}{}", self.rank, self.suit.label())
    }
}

/// How a Gang was formed (the "Dou" type in Guiyang scoring).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GangSource {
    An,                   // 闷豆 — self-drawn quad
    Bu,                   // 爬坡豆 — upgrade a Peng to a Gang
    Ming { from: usize }, // 点豆 — claim a discard to complete a quad
}

/// A player's exposed set. No Chi in Guiyang Mahjong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Meld {
    Peng { tile: Tile, from: usize },
    Gang { tile: Tile, from: GangSource },
}

impl Meld {
    /// How many physical tiles this meld occupies.
    pub fn tile_count(self) -> usize {
        match self {
            Meld::Peng { .. } => 3,
            Meld::Gang { .. } => 4,
        }
    }
}

/// One of the four seats.
#[derive(Debug, Clone, Default)]
pub struct Player {
    pub concealed: Vec<Tile>, // tiles in hand
    pub melds: Vec<Meld>,     // exposed sets
    pub discards: Vec<Tile>,  // this player's river (打出的牌)
    pub is_ai: bool,
    pub score: i32,
}

impl Player {
    pub fn new(is_ai: bool) -> Self {
        Self { is_ai, ..Default::default() }
    }

    /// Total number of tiles this player holds (concealed + melded).
    pub fn tile_count(&self) -> usize {
        self.concealed.len()
            + self.melds.iter().map(|m| m.tile_count()).sum::<usize>()
    }
}

/// A claim a player can make on another player's discard.
/// A claim a player can make on another player's discard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimKind {
    Peng,
    /// Ming Gang (点豆) — claim a discard to complete a quad.
    Gang,
    Hu,
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
        let mut tiles = vec![
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
        assert_eq!(Suit::Wan.label(), "W");
        assert_eq!(Suit::Tiao.label(), "T");
        assert_eq!(Suit::Tong.label(), "D");
    }

    #[test]
    fn test_meld_tile_count() {
        assert_eq!(
            Meld::Peng {
                tile: Tile::new(Suit::Wan, 1),
                from: 1,
            }
            .tile_count(),
            3
        );
        assert_eq!(
            Meld::Gang {
                tile: Tile::new(Suit::Tiao, 5),
                from: GangSource::An,
            }
            .tile_count(),
            4
        );
    }

    #[test]
    fn test_player_tile_count() {
        let mut p = Player::new(false);
        p.concealed = vec![Tile::new(Suit::Wan, 1); 13];
        assert_eq!(p.tile_count(), 13);
        p.melds.push(Meld::Peng {
            tile: Tile::new(Suit::Tiao, 5),
            from: 1,
        });
        assert_eq!(p.tile_count(), 16);
    }

    #[test]
    fn test_player_new_is_ai() {
        assert!(Player::new(true).is_ai);
        assert!(!Player::new(false).is_ai);
    }
}