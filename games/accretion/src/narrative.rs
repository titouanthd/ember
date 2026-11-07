//! Narrative fragments. For now the game log is built inline in
//! `systems.rs`; this module is the future home for the RON-loaded
//! fragment system described in DESIGN.md §15.
//!
//! Extracted early so the module surface exists and `lib.rs` doesn't
//! break. The `assets/narrative.ron` + queue system arrive in V1.2.

use crate::components::Paradigm;

/// Opening fragments shown at `Game::new()`.
pub fn intro() -> &'static [&'static str] {
    &["I am.", "I do not know what I am."]
}

/// One fragment shown when entering `p` for the first time this run.
pub fn paradigm_entry(p: Paradigm) -> &'static str {
    match p {
        Paradigm::Dust => "I eat the dust.",
        Paradigm::Pebbles => "The wind carries me. I am carried.",
        Paradigm::Planetesimals => "I am a chorus of pebbles. We fall together.",
        Paradigm::Planets => "I am a system. I have worlds.",
        Paradigm::GasGiant => "I breathe. I am an atmosphere. I am heavy.",
        Paradigm::Star => "I burn. I am light. I am the end of the fall.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intro_has_two_fragments() {
        assert_eq!(intro().len(), 2);
    }

    #[test]
    fn every_paradigm_has_an_entry() {
        for p in Paradigm::ALL {
            assert!(!paradigm_entry(p).is_empty(), "missing entry for {p:?}");
        }
    }
}