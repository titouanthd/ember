//! Tile kinds, resources, and tile HP per DESIGN §9.
//!
//! Pure data — no `macroquad`. Testable headless.

use serde::{Deserialize, Serialize};

/// Tile size in pixels. The design is tile-based; rendering is
/// pixel-perfect (see DESIGN §1 and §19).
pub const TILE_SIZE: f32 = 16.0;

// ---------- Resources ----------

/// What a broken tile drops. DESIGN §15, retuned for the six-tile
/// balance pass.
///
/// One resource per mineable `TileKind` — Dirt, Stone, Coal, Iron,
/// Rock, Crystal. Bedrock and Capsule drop nothing (Bedrock is
/// indestructible; Capsule frees a robot).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceKind {
    Dirt,
    Stone,
    Coal,
    Iron,
    Rock,
    Crystal,
}

impl ResourceKind {
    /// Sale value in dollars per unit.
    pub fn value(self) -> u32 {
        match self {
            ResourceKind::Dirt => 1,
            ResourceKind::Stone => 2,
            ResourceKind::Coal => 4,
            ResourceKind::Iron => 8,
            ResourceKind::Rock => 12,
            ResourceKind::Crystal => 25,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            ResourceKind::Dirt => "Dirt",
            ResourceKind::Stone => "Stone",
            ResourceKind::Coal => "Coal",
            ResourceKind::Iron => "Iron",
            ResourceKind::Rock => "Rock",
            ResourceKind::Crystal => "Crystal",
        }
    }
}

// ---------- Tiles ----------

/// Every tile type that can appear in a mountain. DESIGN §9.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TileKind {
    Dirt,
    Stone,
    Coal,
    Iron,
    Crystal,
    HardRock,
    Bedrock,
    Capsule,
}

impl TileKind {
    /// Starting HP per DESIGN §9.
    pub fn hp(self) -> f32 {
        match self {
            TileKind::Dirt => 1.0,
            TileKind::Stone => 2.0,
            TileKind::Coal => 3.0,
            TileKind::Iron => 4.0,
            TileKind::Crystal => 6.0,
            TileKind::HardRock => 20.0,
            TileKind::Bedrock => f32::INFINITY,
            TileKind::Capsule => 5.0,
        }
    }

    /// Bedrock can never be mined. DESIGN §9.
    pub fn is_indestructible(self) -> bool {
        matches!(self, TileKind::Bedrock)
    }

    /// What drops when the tile breaks. One resource per tile kind.
    pub fn drop(self) -> Option<ResourceKind> {
        match self {
            TileKind::Dirt => Some(ResourceKind::Dirt),
            TileKind::Stone => Some(ResourceKind::Stone),
            TileKind::Coal => Some(ResourceKind::Coal),
            TileKind::Iron => Some(ResourceKind::Iron),
            TileKind::HardRock => Some(ResourceKind::Rock),
            TileKind::Crystal => Some(ResourceKind::Crystal),
            TileKind::Bedrock => None,
            TileKind::Capsule => None,
        }
    }

    /// True if this tile is a robot capsule.
    pub fn is_capsule(self) -> bool {
        matches!(self, TileKind::Capsule)
    }

    /// Base RGB (0-255). Earthy palette, per DESIGN §1.
    pub fn color(self) -> (u8, u8, u8) {
        match self {
            TileKind::Dirt => (0x6B, 0x4A, 0x2A),
            TileKind::Stone => (0x5A, 0x5A, 0x5A),
            TileKind::Coal => (0x1E, 0x20, 0x2A),
            TileKind::Iron => (0x8B, 0x6F, 0x4A),
            TileKind::Crystal => (0x5F, 0xD4, 0xD4),
            TileKind::HardRock => (0x4A, 0x2E, 0x24),
            TileKind::Bedrock => (0x1A, 0x1A, 0x1A),
            TileKind::Capsule => (0xC4, 0x7A, 0x3A),
        }
    }
}

// ---------- Tile instance ----------

/// A tile in the world. `hp` decreases as Archie or robots mine it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Tile {
    pub kind: TileKind,
    pub hp: f32,
}

impl Tile {
    pub fn new(kind: TileKind) -> Self {
        Self { kind, hp: kind.hp() }
    }

    pub fn is_broken(&self) -> bool {
        self.hp <= 0.0
    }

    /// Applies damage. No-op on Bedrock.
    pub fn damage(&mut self, amount: f32) {
        if self.kind.is_indestructible() {
            return;
        }
        self.hp = (self.hp - amount).max(0.0);
    }

    /// Fraction of HP remaining, in [0, 1].
    pub fn hp_fraction(&self) -> f32 {
        let max = self.kind.hp();
        if max.is_infinite() || max <= 0.0 {
            return 1.0;
        }
        (self.hp / max).clamp(0.0, 1.0)
    }

    /// Crack stage, 0 (intact) to 3 (almost broken). DESIGN §9.
    pub fn crack_stage(&self) -> u8 {
        let f = self.hp_fraction();
        if f > 0.75 {
            0
        } else if f > 0.50 {
            1
        } else if f > 0.25 {
            2
        } else {
            3
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_hp_matches_design_table() {
        assert_eq!(TileKind::Dirt.hp(), 1.0);
        assert_eq!(TileKind::Stone.hp(), 2.0);
        assert_eq!(TileKind::Coal.hp(), 3.0);
        assert_eq!(TileKind::Iron.hp(), 4.0);
        assert_eq!(TileKind::Crystal.hp(), 6.0);
        assert_eq!(TileKind::HardRock.hp(), 20.0);
        assert_eq!(TileKind::Capsule.hp(), 5.0);
        assert!(TileKind::Bedrock.hp().is_infinite());
    }

    #[test]
    fn bedrock_is_indestructible() {
        let mut t = Tile::new(TileKind::Bedrock);
        t.damage(1_000_000.0);
        assert!(!t.is_broken());
    }

    #[test]
    fn dirt_breaks_after_one_damage() {
        let mut t = Tile::new(TileKind::Dirt);
        assert!(!t.is_broken());
        t.damage(1.0);
        assert!(t.is_broken());
    }

    #[test]
    fn stone_breaks_after_two_hits() {
        let mut t = Tile::new(TileKind::Stone);
        t.damage(1.0);
        assert!(!t.is_broken());
        t.damage(1.0);
        assert!(t.is_broken());
    }

    #[test]
    fn every_mineable_tile_has_its_own_resource() {
        // One resource per tile — this is the whole point of the
        // 6-resource pass. Bedrock and Capsule intentionally drop
        // nothing.
        assert_eq!(TileKind::Dirt.drop(), Some(ResourceKind::Dirt));
        assert_eq!(TileKind::Stone.drop(), Some(ResourceKind::Stone));
        assert_eq!(TileKind::Coal.drop(), Some(ResourceKind::Coal));
        assert_eq!(TileKind::Iron.drop(), Some(ResourceKind::Iron));
        assert_eq!(TileKind::HardRock.drop(), Some(ResourceKind::Rock));
        assert_eq!(TileKind::Crystal.drop(), Some(ResourceKind::Crystal));
    }

    #[test]
    fn bedrock_and_capsule_drop_nothing() {
        assert_eq!(TileKind::Bedrock.drop(), None);
        assert_eq!(TileKind::Capsule.drop(), None);
    }

    #[test]
    fn resource_values_match_balance_table() {
        assert_eq!(ResourceKind::Dirt.value(), 1);
        assert_eq!(ResourceKind::Stone.value(), 2);
        assert_eq!(ResourceKind::Coal.value(), 4);
        assert_eq!(ResourceKind::Iron.value(), 8);
        assert_eq!(ResourceKind::Rock.value(), 12);
        assert_eq!(ResourceKind::Crystal.value(), 25);
    }

    #[test]
    fn resource_values_are_strictly_increasing() {
        // Sanity: the progression must be monotonic so tiers read
        // intuitively.
        let v = [
            ResourceKind::Dirt.value(),
            ResourceKind::Stone.value(),
            ResourceKind::Coal.value(),
            ResourceKind::Iron.value(),
            ResourceKind::Rock.value(),
            ResourceKind::Crystal.value(),
        ];
        for i in 1..v.len() {
            assert!(v[i] > v[i - 1], "value not increasing at index {i}");
        }
    }

    #[test]
    fn resource_names_are_distinct() {
        let names = [
            ResourceKind::Dirt.name(),
            ResourceKind::Stone.name(),
            ResourceKind::Coal.name(),
            ResourceKind::Iron.name(),
            ResourceKind::Rock.name(),
            ResourceKind::Crystal.name(),
        ];
        for (i, a) in names.iter().enumerate() {
            for b in names.iter().skip(i + 1) {
                assert_ne!(a, b, "duplicate resource name");
            }
        }
    }

    #[test]
    fn crack_stage_advances_with_damage() {
        let mut t = Tile::new(TileKind::Stone); // hp = 2.0
        assert_eq!(t.crack_stage(), 0);
        t.damage(0.6); // hp = 1.4 / 2.0 = 0.70
        assert_eq!(t.crack_stage(), 1);
        t.damage(0.6); // hp = 0.8 / 2.0 = 0.40
        assert_eq!(t.crack_stage(), 2);
        t.damage(0.6); // hp = 0.2 / 2.0 = 0.10
        assert_eq!(t.crack_stage(), 3);
    }

    #[test]
    fn tile_serializes_roundtrip() {
        let t = Tile::new(TileKind::Iron);
        let s = ron::to_string(&t).unwrap();
        let back: Tile = ron::from_str(&s).unwrap();
        assert_eq!(t, back);
    }

    #[test]
    fn resource_kind_serializes_roundtrip() {
        for r in [
            ResourceKind::Dirt,
            ResourceKind::Stone,
            ResourceKind::Coal,
            ResourceKind::Iron,
            ResourceKind::Rock,
            ResourceKind::Crystal,
        ] {
            let s = ron::to_string(&r).unwrap();
            let back: ResourceKind = ron::from_str(&s).unwrap();
            assert_eq!(back, r);
        }
    }
}