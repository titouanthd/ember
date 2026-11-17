//! Touch input. v0.1 models per-point lifecycle so games that drag
//! (Freecell, jade-garden) can track a finger; games that just tap
//! should prefer `Input::primary_action()`.

use glam::Vec2;

/// The current lifecycle state of a touch point.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TouchPhase {
    /// The finger touched the screen this frame.
    Started,
    /// The finger is still down and has moved since last frame.
    Moved,
    /// The finger is still down and has not moved this frame.
    Stationary,
    /// The finger lifted off this frame.
    Ended,
    /// The system cancelled tracking (e.g. an incoming call).
    Cancelled,
}

/// One touch point, keyed by `id` across frames.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TouchPoint {
    pub id: u64,
    pub position: Vec2,
    pub phase: TouchPhase,
}

impl TouchPoint {
    pub fn new(id: u64, position: Vec2, phase: TouchPhase) -> Self {
        Self {
            id,
            position,
            phase,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn touch_point_carries_id_position_phase() {
        let t = TouchPoint::new(7, Vec2::new(10.0, 20.0), TouchPhase::Started);
        assert_eq!(t.id, 7);
        assert_eq!(t.position, Vec2::new(10.0, 20.0));
        assert_eq!(t.phase, TouchPhase::Started);
    }

    #[test]
    fn phases_are_distinct() {
        let all = [
            TouchPhase::Started,
            TouchPhase::Moved,
            TouchPhase::Stationary,
            TouchPhase::Ended,
            TouchPhase::Cancelled,
        ];
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }
}
