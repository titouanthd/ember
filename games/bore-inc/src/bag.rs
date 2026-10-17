//! Archie's bag: what he carries between the mine and the truck.
//!
//! Isolated from `player.rs` so the state is testable on its own and
//! the visual (procedural bag sprite) can evolve independently.

use std::collections::HashMap;

use crate::components::ResourceKind;

/// Default capacity, in units. Upgradable via `set_capacity`.
pub const DEFAULT_CAPACITY: u32 = 10;

/// The bag on Archie's back.
#[derive(Debug, Clone)]
pub struct Bag {
    items: HashMap<ResourceKind, u32>,
    capacity: u32,
}

impl Bag {
    pub fn new(capacity: u32) -> Self {
        Self {
            items: HashMap::new(),
            capacity,
        }
    }

    /// Total units currently held.
    pub fn used(&self) -> u32 {
        self.items.values().copied().sum()
    }

    /// Free space, in units.
    pub fn free(&self) -> u32 {
        self.capacity.saturating_sub(self.used())
    }

    pub fn capacity(&self) -> u32 {
        self.capacity
    }

    pub fn set_capacity(&mut self, capacity: u32) {
        self.capacity = capacity;
    }

    pub fn is_full(&self) -> bool {
        self.used() >= self.capacity
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Add `count` units of `resource`. Returns how many were accepted
    /// (may be fewer than `count` if the bag fills up).
    pub fn collect(&mut self, resource: ResourceKind, count: u32) -> u32 {
        let n = count.min(self.free());
        if n > 0 {
            *self.items.entry(resource).or_insert(0) += n;
        }
        n
    }

    /// Empty the bag and return its contents.
    pub fn empty(&mut self) -> HashMap<ResourceKind, u32> {
        std::mem::take(&mut self.items)
    }

    /// Drop all items without returning them. Used by tests.
    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn items(&self) -> &HashMap<ResourceKind, u32> {
        &self.items
    }

    /// Contents in a fixed order (Dirt, Stone, Coal, Iron, Rock,
    /// Crystal), one entry per resource actually held. Used by the
    /// HUD readout so the row order stays stable across frames.
    pub fn contents_in_order(&self) -> Vec<(ResourceKind, u32)> {
        use ResourceKind::*;
        [Dirt, Stone, Coal, Iron, Rock, Crystal]
            .iter()
            .filter_map(|&r| self.items.get(&r).copied().map(|n| (r, n)))
            .collect()
    }

    /// Fill ratio in [0, 1].
    pub fn fill_ratio(&self) -> f32 {
        if self.capacity == 0 {
            return 0.0;
        }
        (self.used() as f32 / self.capacity as f32).clamp(0.0, 1.0)
    }
}

impl Default for Bag {
    fn default() -> Self {
        Self::new(DEFAULT_CAPACITY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_bag_is_empty() {
        let b = Bag::default();
        assert_eq!(b.used(), 0);
        assert_eq!(b.free(), DEFAULT_CAPACITY);
        assert!(b.is_empty());
        assert!(!b.is_full());
    }

    #[test]
    fn collect_adds_up_to_capacity() {
        let mut b = Bag::new(5);
        assert_eq!(b.collect(ResourceKind::Stone, 3), 3);
        assert_eq!(b.collect(ResourceKind::Iron, 5), 2);
        assert_eq!(b.used(), 5);
        assert!(b.is_full());
    }

    #[test]
    fn collect_zero_free_is_noop() {
        let mut b = Bag::new(2);
        b.collect(ResourceKind::Stone, 2);
        assert_eq!(b.collect(ResourceKind::Coal, 5), 0);
        assert_eq!(b.used(), 2);
    }

    #[test]
    fn empty_returns_contents() {
        let mut b = Bag::new(10);
        b.collect(ResourceKind::Stone, 3);
        b.collect(ResourceKind::Crystal, 2);
        let out = b.empty();
        assert_eq!(out.get(&ResourceKind::Stone), Some(&3));
        assert_eq!(out.get(&ResourceKind::Crystal), Some(&2));
        assert!(b.is_empty());
    }

    #[test]
    fn fill_ratio_clamps() {
        let mut b = Bag::new(10);
        assert_eq!(b.fill_ratio(), 0.0);
        b.collect(ResourceKind::Stone, 5);
        assert!((b.fill_ratio() - 0.5).abs() < 1e-6);
        b.collect(ResourceKind::Stone, 100);
        assert_eq!(b.fill_ratio(), 1.0);
    }

    #[test]
    fn fill_ratio_with_zero_capacity_is_zero() {
        let b = Bag::new(0);
        assert_eq!(b.fill_ratio(), 0.0);
    }

    #[test]
    fn contents_in_order_skips_empty_types() {
        let mut b = Bag::new(10);
        b.collect(ResourceKind::Iron, 2);
        b.collect(ResourceKind::Stone, 3);
        let entries = b.contents_in_order();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0], (ResourceKind::Stone, 3));
        assert_eq!(entries[1], (ResourceKind::Iron, 2));
    }

    #[test]
    fn contents_in_order_is_stable() {
        let mut b = Bag::new(20);
        b.collect(ResourceKind::Crystal, 1);
        b.collect(ResourceKind::Coal, 2);
        b.collect(ResourceKind::Stone, 3);
        let entries = b.contents_in_order();
        assert_eq!(entries[0].0, ResourceKind::Stone);
        assert_eq!(entries[1].0, ResourceKind::Coal);
        assert_eq!(entries[2].0, ResourceKind::Crystal);
    }

    #[test]
    fn contents_in_order_includes_dirt_and_rock() {
        let mut b = Bag::new(20);
        b.collect(ResourceKind::Rock, 1);
        b.collect(ResourceKind::Dirt, 4);
        let entries = b.contents_in_order();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0], (ResourceKind::Dirt, 4));
        assert_eq!(entries[1], (ResourceKind::Rock, 1));
    }

    #[test]
    fn contents_in_order_uses_value_order() {
        // Full ordering, one of each — must be Dirt..Crystal.
        let mut b = Bag::new(20);
        b.collect(ResourceKind::Crystal, 1);
        b.collect(ResourceKind::Rock, 1);
        b.collect(ResourceKind::Iron, 1);
        b.collect(ResourceKind::Coal, 1);
        b.collect(ResourceKind::Stone, 1);
        b.collect(ResourceKind::Dirt, 1);
        let kinds: Vec<_> = b.contents_in_order().into_iter().map(|(r, _)| r).collect();
        assert_eq!(
            kinds,
            vec![
                ResourceKind::Dirt,
                ResourceKind::Stone,
                ResourceKind::Coal,
                ResourceKind::Iron,
                ResourceKind::Rock,
                ResourceKind::Crystal,
            ]
        );
    }
}