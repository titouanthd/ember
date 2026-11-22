//! `FocusManager`: which control receives keyboard input (spec §6).

use crate::ControlRegistry;
use ember_scene::{NodeId, NodeTree};

/// One focused control at a time, moving in tree order (depth first).
/// Owned by `UiRoot`; games ask for focus with `UiRoot::request_focus`.
#[derive(Debug, Clone, Default)]
pub struct FocusManager {
    focused: Option<NodeId>,
}

impl FocusManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn focused(&self) -> Option<NodeId> {
        self.focused
    }

    /// The control after (or before) the focused one in `focusables`,
    /// wrapping around. With nothing focused, the first (or last).
    pub fn neighbour(&self, focusables: &[NodeId], forward: bool) -> Option<NodeId> {
        if focusables.is_empty() {
            return None;
        }
        let n = focusables.len();
        let next = match self
            .focused
            .and_then(|f| focusables.iter().position(|&x| x == f))
        {
            Some(i) if forward => (i + 1) % n,
            Some(i) => (i + n - 1) % n,
            None if forward => 0,
            None => n - 1,
        };
        Some(focusables[next])
    }

    /// Move focus to `target`: `on_blur` on the old control, `on_focus`
    /// on the new one, `focused` flags updated. Returns `true` if focus
    /// changed.
    pub fn set(
        &mut self,
        tree: &mut NodeTree,
        registry: &ControlRegistry,
        target: Option<NodeId>,
    ) -> bool {
        if target == self.focused {
            return false;
        }
        if let Some(old) = self.focused
            && let Some(c) = tree.get_mut(old).and_then(|n| registry.cast_mut(n))
        {
            c.base_mut().focused = false;
            c.on_blur();
        }
        self.focused = target;
        if let Some(new) = target
            && let Some(c) = tree.get_mut(new).and_then(|n| registry.cast_mut(n))
        {
            c.base_mut().focused = true;
            c.on_focus();
        }
        true
    }

    /// Drop focus if the focused control is gone or no longer focusable
    /// (removed, hidden, disabled). Returns `true` if focus was dropped.
    pub fn validate(&mut self, focusables: &[NodeId]) -> bool {
        match self.focused {
            Some(f) if !focusables.contains(&f) => {
                self.focused = None;
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(n: usize) -> Vec<NodeId> {
        // NodeIds only come from a tree; a throwaway tree hands out n of them.
        let mut tree = NodeTree::new();
        (0..n).map(|_| tree.insert(crate::Label::new(""))).collect()
    }

    #[test]
    fn neighbour_wraps_both_ways() {
        let v = ids(3);
        let mut f = FocusManager::new();
        assert_eq!(f.neighbour(&v, true), Some(v[0]));
        assert_eq!(f.neighbour(&v, false), Some(v[2]));
        f.focused = Some(v[2]);
        assert_eq!(f.neighbour(&v, true), Some(v[0]));
        f.focused = Some(v[0]);
        assert_eq!(f.neighbour(&v, false), Some(v[2]));
    }

    #[test]
    fn neighbour_of_nothing_is_none() {
        assert_eq!(FocusManager::new().neighbour(&[], true), None);
    }

    #[test]
    fn validate_drops_a_vanished_focus() {
        let v = ids(2);
        let mut f = FocusManager::new();
        f.focused = Some(v[1]);
        assert!(!f.validate(&v));
        assert!(f.validate(&v[..1]));
        assert_eq!(f.focused(), None);
    }
}
