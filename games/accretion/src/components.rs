//! Pure game types. No `macroquad`. Headless-testable.

use serde::{Deserialize, Serialize};

/// The eight resources of the game, in **production order**.
///
/// `Ord` is derived so tier comparisons work: `Dust < Pebbles < ...`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ResourceId {
    Dust,
    Pebbles,
    Planetesimals,
    Metals,
    Water,
    Gas,
    Light,
    DarkMatter,
}

impl ResourceId {
    /// All resources, in production order. Indexed by enum discriminant.
    pub const ALL: [ResourceId; 8] = [
        ResourceId::Dust,
        ResourceId::Pebbles,
        ResourceId::Planetesimals,
        ResourceId::Metals,
        ResourceId::Water,
        ResourceId::Gas,
        ResourceId::Light,
        ResourceId::DarkMatter,
    ];

    /// Human-readable name for the HUD.
    pub fn name(self) -> &'static str {
        match self {
            ResourceId::Dust => "Dust",
            ResourceId::Pebbles => "Pebbles",
            ResourceId::Planetesimals => "Planetesimals",
            ResourceId::Metals => "Metals",
            ResourceId::Water => "Water",
            ResourceId::Gas => "Gas",
            ResourceId::Light => "Light",
            ResourceId::DarkMatter => "Dark Matter",
        }
    }
}

/// The six major game stages. `Ord` is derived from declaration order
/// so `current < next` works naturally.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Paradigm {
    Dust,
    Pebbles,
    Planetesimals,
    Planets,
    GasGiant,
    Star,
}

impl Paradigm {
    /// All paradigms, in progression order.
    pub const ALL: [Paradigm; 6] = [
        Paradigm::Dust,
        Paradigm::Pebbles,
        Paradigm::Planetesimals,
        Paradigm::Planets,
        Paradigm::GasGiant,
        Paradigm::Star,
    ];

    /// Human-readable name for the HUD.
    pub fn name(self) -> &'static str {
        match self {
            Paradigm::Dust => "Dust",
            Paradigm::Pebbles => "Pebbles",
            Paradigm::Planetesimals => "Planetesimals",
            Paradigm::Planets => "Planets",
            Paradigm::GasGiant => "Gas Giant",
            Paradigm::Star => "Star",
        }
    }

    /// The next paradigm in the chain, or `None` at the end.
    pub fn next(self) -> Option<Paradigm> {
        Paradigm::ALL.iter().copied().skip_while(|&p| p <= self).next()
    }
}

/// One of the two endgame choices. Serialized into `Game::endings_seen`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Ending {
    NeutronStar,
    BlackHole,
}

impl Ending {
    pub const ALL: [Ending; 2] = [Ending::NeutronStar, Ending::BlackHole];

    pub fn name(self) -> &'static str {
        match self {
            Ending::NeutronStar => "Neutron Star",
            Ending::BlackHole => "Black Hole",
        }
    }

    pub fn description(self) -> &'static [&'static str] {
        match self {
            Ending::NeutronStar => &[
                "I no longer fall.",
                "I shine.",
                "I have always shone.",
                "I will always shine.",
            ],
            Ending::BlackHole => &[
                "I no longer shine.",
                "I curve.",
                "I am curvature.",
                "I am the space that remembers me.",
            ],
        }
    }
}

/// The twelve buildings. Order matters: `BUILDINGS` must be sorted by
/// discriminant (verified by a test in `economy`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum BuildingId {
    DustCollector,
    AccretionRing,
    PebbleFoundry,
    ResonanceEngine,
    MetalFoundry,
    WaterCondenser,
    OrbitalAnchor,
    GasScoop,
    MagnetosphereCoil,
    FusionCatalyst,
    JetInjector,
    VoidWeaver,
}

impl BuildingId {
    /// All building ids, in `BUILDINGS` table order.
    pub const ALL: [BuildingId; 12] = [
        BuildingId::DustCollector,
        BuildingId::AccretionRing,
        BuildingId::PebbleFoundry,
        BuildingId::ResonanceEngine,
        BuildingId::MetalFoundry,
        BuildingId::WaterCondenser,
        BuildingId::OrbitalAnchor,
        BuildingId::GasScoop,
        BuildingId::MagnetosphereCoil,
        BuildingId::FusionCatalyst,
        BuildingId::JetInjector,
        BuildingId::VoidWeaver,
    ];
}

/// The visual kind of a body on the disk. Purely cosmetic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BodyKind {
    Dust,
    Pebble,
    Planetesimal,
    Planet,
    GasGiant,
    Star,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_all_matches_discriminants() {
        for (i, r) in ResourceId::ALL.iter().enumerate() {
            assert_eq!(*r as usize, i);
        }
    }

    #[test]
    fn paradigm_all_matches_discriminants() {
        for (i, p) in Paradigm::ALL.iter().enumerate() {
            assert_eq!(*p as usize, i);
        }
    }

    #[test]
    fn building_all_matches_discriminants() {
        for (i, b) in BuildingId::ALL.iter().enumerate() {
            assert_eq!(*b as usize, i);
        }
    }

    #[test]
    fn paradigm_ordering_is_progression() {
        assert!(Paradigm::Dust < Paradigm::Pebbles);
        assert!(Paradigm::Pebbles < Paradigm::Planetesimals);
        assert!(Paradigm::Planetesimals < Paradigm::Planets);
        assert!(Paradigm::Planets < Paradigm::GasGiant);
        assert!(Paradigm::GasGiant < Paradigm::Star);
    }

    #[test]
    fn paradigm_next_walks_the_chain() {
        assert_eq!(Paradigm::Dust.next(), Some(Paradigm::Pebbles));
        assert_eq!(Paradigm::Pebbles.next(), Some(Paradigm::Planetesimals));
        assert_eq!(Paradigm::Planetesimals.next(), Some(Paradigm::Planets));
        assert_eq!(Paradigm::Planets.next(), Some(Paradigm::GasGiant));
        assert_eq!(Paradigm::GasGiant.next(), Some(Paradigm::Star));
        assert_eq!(Paradigm::Star.next(), None);
    }

    #[test]
    fn resource_ordering_is_production_chain() {
        assert!(ResourceId::Dust < ResourceId::Pebbles);
        assert!(ResourceId::Pebbles < ResourceId::Planetesimals);
        assert!(ResourceId::Planetesimals < ResourceId::Metals);
        assert!(ResourceId::Metals < ResourceId::Water);
        assert!(ResourceId::Water < ResourceId::Gas);
        assert!(ResourceId::Gas < ResourceId::Light);
        assert!(ResourceId::Light < ResourceId::DarkMatter);
    }

    #[test]
    fn resource_names_are_non_empty() {
        for r in ResourceId::ALL {
            assert!(!r.name().is_empty());
        }
    }

    #[test]
    fn paradigm_names_are_non_empty() {
        for p in Paradigm::ALL {
            assert!(!p.name().is_empty());
        }
    }

    #[test]
    fn dark_matter_is_the_last_resource() {
        assert_eq!(ResourceId::ALL.last(), Some(&ResourceId::DarkMatter));
    }

    #[test]
    fn star_is_the_last_paradigm() {
        assert_eq!(Paradigm::ALL.last(), Some(&Paradigm::Star));
    }

    #[test]
    fn void_weaver_is_the_last_building() {
        assert_eq!(BuildingId::ALL.last(), Some(&BuildingId::VoidWeaver));
    }

    #[test]
    fn ending_all_has_two_entries() {
        assert_eq!(Ending::ALL.len(), 2);
    }

    #[test]
    fn endings_are_distinct() {
        assert_ne!(Ending::NeutronStar, Ending::BlackHole);
    }

    #[test]
    fn endings_have_names_and_text() {
        for e in Ending::ALL {
            assert!(!e.name().is_empty());
            assert!(!e.description().is_empty());
        }
    }
}