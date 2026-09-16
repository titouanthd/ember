//! Game entities. Pure data + tiny helpers, no macroquad.
//!
//! Coordinate convention: origin top-left, x right, y down (Ember default).
//! The only struct with a non-standard origin would be a paddle *visual*,
//! and we do not introduce one here — `Paddle::pos` is the circle centre.

use std::collections::VecDeque;

use glam::Vec2;

/// Which half of the table a paddle defends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

impl Side {
    #[inline]
    pub fn opponent(self) -> Self {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }

    /// Direction the side pushes the puck towards the *opponent's* goal.
    /// Left player attacks toward +x, right player toward -x.
    #[inline]
    pub fn attack_sign(self) -> f32 {
        match self {
            Side::Left => 1.0,
            Side::Right => -1.0,
        }
    }

    /// The goal this side *defends*. Left defends the left goal.
    #[inline]
    pub fn own_goal_sign(self) -> f32 {
        match self {
            Side::Left => -1.0,
            Side::Right => 1.0,
        }
    }
}

/// A paddle controlled by one player.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Paddle {
    /// Circle centre in world units.
    pub pos: Vec2,
    /// Velocity in world units per second.
    pub vel: Vec2,
    /// Circle radius.
    pub radius: f32,
    /// Which half this paddle belongs to.
    pub side: Side,
}

impl Paddle {
    pub fn new(pos: Vec2, radius: f32, side: Side) -> Self {
        Self { pos, vel: Vec2::ZERO, radius, side }
    }

    /// Speed below which we treat the paddle as at rest.
    /// Prevents the puck from picking up numerical dust from a "still" paddle.
    pub const REST_EPSILON: f32 = 5.0;

    #[inline]
    pub fn is_at_rest(&self) -> bool {
        self.vel.length_squared() < Self::REST_EPSILON * Self::REST_EPSILON
    }
}

/// The puck.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Puck {
    pub pos: Vec2,
    pub vel: Vec2,
    pub radius: f32,
    /// Visual spin angle in radians (never affects physics).
    pub spin: f32,
}

impl Puck {
    pub fn new(pos: Vec2, radius: f32) -> Self {
        Self { pos, vel: Vec2::ZERO, radius, spin: 0.0 }
    }
}

/// Rolling trail behind the puck. Bounded ring of recent positions.
///
/// We sample once per *frame*, not per sub-step, so the visual trail
/// matches what the eye sees — sub-step sampling would just cluster
/// points on top of each other at low puck speed.
#[derive(Debug, Clone, Default)]
pub struct Trail {
    points: VecDeque<(Vec2, f32)>,
    /// Age in seconds, accumulates so each point can fade independently.
    age: f32,
}

impl Trail {
    /// Max points kept. Matches `TRAIL_LEN` in DESIGN.md §12.
    pub const MAX_LEN: usize = 6;
    /// Seconds a point takes to fade to zero alpha.
    pub const LIFETIME: f32 = 0.20;

    pub fn new() -> Self {
        Self { points: VecDeque::with_capacity(Self::MAX_LEN), age: 0.0 }
    }

    /// Push the current puck position. Drops the oldest point when full.
    pub fn push(&mut self, pos: Vec2) {
        if self.points.len() == Self::MAX_LEN {
            self.points.pop_front();
        }
        self.points.push_back((pos, 0.0));
    }

    /// Advance all point ages by `dt` and drop expired points.
    pub fn update(&mut self, dt: f32) {
        self.age += dt;
        while let Some(&(_, age)) = self.points.front() {
            if age + dt < Self::LIFETIME {
                break;
            }
            self.points.pop_front();
        }
        for (_, age) in self.points.iter_mut() {
            *age += dt;
        }
    }

    /// Drop all points (used when the puck resets after a goal).
    pub fn clear(&mut self) {
        self.points.clear();
        self.age = 0.0;
    }

    /// Iterate newest-first. `t` is 0 at the newest point and grows towards 1
    /// for older points, matching the draw-side fade-out.
    pub fn iter(&self) -> impl Iterator<Item = (Vec2, f32)> + '_ {
        let len = self.points.len();
        self.points.iter().enumerate().rev().map(move |(i, &(p, age))| {
            let from_front = (len - 1 - i) as f32 / Self::MAX_LEN.max(1) as f32;
            let life = (age / Self::LIFETIME).clamp(0.0, 1.0);
            let t = (from_front + life).clamp(0.0, 1.0);
            (p, t)
        })
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.points.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }
}

/// One visual particle (goal burst).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Particle {
    pub pos: Vec2,
    pub vel: Vec2,
    /// Seconds remaining before it disappears.
    pub life: f32,
    /// Original lifetime, used to compute the fade fraction.
    pub max_life: f32,
}

impl Particle {
    pub fn new(pos: Vec2, vel: Vec2, life: f32) -> Self {
        Self { pos, vel, life, max_life: life }
    }

    /// Fade fraction in [0, 1]: 1 = just spawned, 0 = about to die.
    #[inline]
    pub fn alive_fraction(&self) -> f32 {
        (self.life / self.max_life).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_side_opponent() {
        assert_eq!(Side::Left.opponent(), Side::Right);
        assert_eq!(Side::Right.opponent(), Side::Left);
    }

    #[test]
    fn test_side_signs_are_opposite() {
        assert_eq!(Side::Left.attack_sign(), -Side::Right.attack_sign());
        assert_eq!(Side::Left.own_goal_sign(), -Side::Right.own_goal_sign());
        // A side attacks the goal it does not defend.
        assert_eq!(Side::Left.attack_sign(), Side::Right.own_goal_sign());
    }

    #[test]
    fn test_trail_caps_at_len() {
        let mut t = Trail::new();
        for i in 0..20 {
            t.push(Vec2::new(i as f32, 0.0));
        }
        assert_eq!(t.len(), Trail::MAX_LEN);
        // Newest-first: the last pushed point must appear first.
        let newest = t.iter().next().unwrap();
        assert!((newest.0.x - 19.0).abs() < 1e-6);
    }

    #[test]
    fn test_trail_clear() {
        let mut t = Trail::new();
        t.push(Vec2::ONE);
        t.push(Vec2::ZERO);
        assert!(!t.is_empty());
        t.clear();
        assert!(t.is_empty());
    }

    #[test]
    fn test_trail_ages_out() {
        let mut t = Trail::new();
        t.push(Vec2::ZERO);
        // Advance past the lifetime: point must be dropped.
        t.update(Trail::LIFETIME + 0.01);
        assert!(t.is_empty());
    }

    #[test]
    fn test_particle_fade_fraction() {
        let mut p = Particle::new(Vec2::ZERO, Vec2::ZERO, 1.0);
        assert!((p.alive_fraction() - 1.0).abs() < 1e-6);
        p.life = 0.5;
        assert!((p.alive_fraction() - 0.5).abs() < 1e-6);
        p.life = 0.0;
        assert!(p.alive_fraction().abs() < 1e-6);
        // Negative life clamps to 0 (should never happen, but cheap to guard).
        p.life = -0.1;
        assert!(p.alive_fraction().abs() < 1e-6);
    }

    #[test]
    fn test_paddle_at_rest() {
        let mut p = Paddle::new(Vec2::ZERO, 28.0, Side::Left);
        assert!(p.is_at_rest());
        p.vel = Vec2::new(Paddle::REST_EPSILON - 0.1, 0.0);
        assert!(p.is_at_rest());
        p.vel = Vec2::new(Paddle::REST_EPSILON + 0.1, 0.0);
        assert!(!p.is_at_rest());
    }
}