//! Clip-rect stack shared by rendering backends (ADR-031).
//!
//! Backends keep one [`ClipStack`] and translate [`ClipStack::current`]
//! into their own scissor call. The nesting rules live here so every
//! backend (and `MockRenderer`) behaves the same:
//!
//! - rects are in **screen pixels**, top-left origin, not affected by the
//!   camera;
//! - a push intersects with the current clip, so a child never draws
//!   outside its parent;
//! - a push that does not overlap the current clip yields an **empty**
//!   rect (zero size): nothing draws until the matching pop;
//! - popping an empty stack is ignored.

use ember_core::math::Rect;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ClipStack {
    stack: Vec<Rect>,
}

impl ClipStack {
    pub fn new() -> Self {
        Self::default()
    }

    /// Push `rect` and return the new effective clip.
    pub fn push(&mut self, rect: Rect) -> Rect {
        let rect = Rect::new(rect.x, rect.y, rect.w.max(0.0), rect.h.max(0.0));
        let effective = match self.current() {
            None => rect,
            Some(current) => current
                .intersect(&rect)
                .unwrap_or(Rect::new(rect.x, rect.y, 0.0, 0.0)),
        };
        self.stack.push(effective);
        effective
    }

    /// Pop the last push and return the clip now in effect (`None` when
    /// unclipped). Popping an empty stack does nothing.
    pub fn pop(&mut self) -> Option<Rect> {
        self.stack.pop();
        self.current()
    }

    /// The clip in effect, `None` when nothing is pushed.
    pub fn current(&self) -> Option<Rect> {
        self.stack.last().copied()
    }

    /// Number of active pushes.
    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    pub fn clear(&mut self) {
        self.stack.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_stack_is_unclipped() {
        let s = ClipStack::new();
        assert_eq!(s.current(), None);
        assert_eq!(s.depth(), 0);
    }

    #[test]
    fn nested_push_intersects_with_parent() {
        let mut s = ClipStack::new();
        s.push(Rect::new(0.0, 0.0, 100.0, 100.0));
        let inner = s.push(Rect::new(50.0, 50.0, 100.0, 100.0));
        assert_eq!(inner, Rect::new(50.0, 50.0, 50.0, 50.0));
        assert_eq!(s.current(), Some(inner));
    }

    #[test]
    fn pop_restores_the_parent_clip() {
        let mut s = ClipStack::new();
        let outer = s.push(Rect::new(0.0, 0.0, 100.0, 100.0));
        s.push(Rect::new(10.0, 10.0, 10.0, 10.0));
        assert_eq!(s.pop(), Some(outer));
        assert_eq!(s.pop(), None);
    }

    #[test]
    fn disjoint_push_clips_everything() {
        let mut s = ClipStack::new();
        s.push(Rect::new(0.0, 0.0, 10.0, 10.0));
        let r = s.push(Rect::new(50.0, 50.0, 10.0, 10.0));
        assert_eq!((r.w, r.h), (0.0, 0.0));
    }

    #[test]
    fn negative_size_is_treated_as_empty() {
        let mut s = ClipStack::new();
        let r = s.push(Rect::new(5.0, 5.0, -3.0, 4.0));
        assert_eq!(r, Rect::new(5.0, 5.0, 0.0, 4.0));
    }

    #[test]
    fn pop_on_empty_stack_is_ignored() {
        let mut s = ClipStack::new();
        assert_eq!(s.pop(), None);
        assert_eq!(s.depth(), 0);
    }
}
