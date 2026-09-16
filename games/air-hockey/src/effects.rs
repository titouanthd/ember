//! Screen shake, hitstop, and goal particles.
//!
//! Everything here is presentation-only. No module depends on `effects` to
//! run physics; `Game` owns these and `main.rs` reads them for the draw.
//!
//! `Shake` computes its offset in `update()` and stores it. Calling `current()`
//! multiple times per frame returns the same value, which keeps rendering
//! deterministic regardless of draw order.

use ember_core::rng::Rng;
use glam::Vec2;

use crate::components::Particle;

/// Screen shake. Bounded by `amplitude` at trigger time, decays linearly
/// to zero over `duration`.
#[derive(Debug, Clone)]
pub struct Shake {
    time_left: f32,
    duration: f32,
    amplitude: f32,
    current: Vec2,
    rng: Rng,
}

impl Shake {
    pub fn new(seed: u32) -> Self {
        Self {
            time_left: 0.0,
            duration: 0.0,
            amplitude: 0.0,
            current: Vec2::ZERO,
            rng: Rng::new(seed),
        }
    }

    /// Start (or restart) a shake. `amplitude` is the max offset in world
    /// units at t = 0.
    pub fn trigger(&mut self, amplitude: f32, duration: f32) {
        self.amplitude = amplitude.max(0.0);
        self.duration = duration.max(0.0);
        self.time_left = self.duration;
        self.current = Vec2::ZERO;
    }

    /// Advance the shake. Computes a fresh random offset for this frame.
    pub fn update(&mut self, dt: f32) {
        if self.time_left <= 0.0 {
            self.current = Vec2::ZERO;
            return;
        }
        self.time_left -= dt;
        if self.time_left <= 0.0 {
            self.time_left = 0.0;
            self.current = Vec2::ZERO;
            return;
        }
        let frac = self.time_left / self.duration;
        let mag = self.amplitude * frac;
        self.current = Vec2::new(
            self.rng.next_f32_range(-mag, mag),
            self.rng.next_f32_range(-mag, mag),
        );
    }

    /// Offset to add to every drawn position this frame.
    #[inline]
    pub fn current(&self) -> Vec2 {
        self.current
    }

    #[inline]
    pub fn is_active(&self) -> bool {
        self.time_left > 0.0
    }
}

impl Default for Shake {
    fn default() -> Self {
        Self::new(0x1234_5678)
    }
}

/// Freeze frame on a hard impact. Presentation only: `Game` reads
/// `active()` and passes `0.0` as the effective `dt` to physics.
#[derive(Debug, Clone, Copy, Default)]
pub struct Hitstop {
    time_left: f32,
}

impl Hitstop {
    pub fn new() -> Self {
        Self { time_left: 0.0 }
    }

    pub fn trigger(&mut self, duration: f32) {
        // Never shorten an ongoing hitstop.
        let d = duration.max(0.0);
        if d > self.time_left {
            self.time_left = d;
        }
    }

    pub fn update(&mut self, dt: f32) {
        if self.time_left > 0.0 {
            self.time_left = (self.time_left - dt).max(0.0);
        }
    }

    #[inline]
    pub fn active(&self) -> bool {
        self.time_left > 0.0
    }

    #[inline]
    pub fn remaining(&self) -> f32 {
        self.time_left
    }
}

/// A short-lived radial burst. No gravity, linear decay.
#[derive(Debug, Clone, Default)]
pub struct Particles {
    items: Vec<Particle>,
}

impl Particles {
    /// Default particle lifetime in seconds.
    pub const LIFETIME: f32 = 0.6;

    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Emit `count` particles from `origin`, each with random direction and
    /// speed in `[0.5 * speed, speed]`. `rng` is caller-owned so bursts are
    /// reproducible across tests.
    pub fn burst(&mut self, origin: Vec2, count: usize, speed: f32, rng: &mut Rng) {
        for _ in 0..count {
            let angle = rng.next_f32_range(0.0, std::f32::consts::TAU);
            let s = rng.next_f32_range(0.5 * speed, speed);
            let vel = Vec2::new(angle.cos(), angle.sin()) * s;
            self.items
                .push(Particle::new(origin, vel, Self::LIFETIME));
        }
    }

    pub fn update(&mut self, dt: f32) {
        for p in self.items.iter_mut() {
            p.pos += p.vel * dt;
            p.life -= dt;
        }
        self.items.retain(|p| p.life > 0.0);
    }

    pub fn iter(&self) -> impl Iterator<Item = &Particle> {
        self.items.iter()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shake_decays_to_zero() {
        let mut s = Shake::new(1);
        s.trigger(10.0, 0.25);
        assert!(s.is_active());
        s.update(0.10);
        assert!(s.is_active());
        s.update(0.20); // total 0.30 > 0.25
        assert!(!s.is_active());
        assert_eq!(s.current(), Vec2::ZERO);
    }

    #[test]
    fn test_shake_offset_bounded_by_amplitude() {
        let mut s = Shake::new(42);
        s.trigger(6.0, 0.25);
        for _ in 0..60 {
            s.update(1.0 / 60.0);
            let o = s.current();
            assert!(o.x.abs() <= 6.0 + 1e-4, "ox = {}", o.x);
            assert!(o.y.abs() <= 6.0 + 1e-4, "oy = {}", o.y);
        }
    }

    #[test]
    fn test_shake_offset_shrinks_over_time() {
        let mut s = Shake::new(7);
        s.trigger(20.0, 0.5);
        s.update(0.05);
        let early = s.current().length();
        // A few frames later the magnitude should be strictly smaller on average.
        for _ in 0..20 {
            s.update(1.0 / 60.0);
        }
        let late = s.current().length();
        // We can only bound the late value by the scaled amplitude, not the
        // early sample (which is one random draw). Assert the *envelope*.
        let frac = (0.5 - (0.05 + 20.0 / 60.0)) / 0.5;
        assert!(frac > 0.0);
        assert!(late <= 20.0 * frac + 1e-3, "late = {}", late);
        let _ = early; // silence unused in release builds
    }

    #[test]
    fn test_hitstop_activates_and_expires() {
        let mut h = Hitstop::new();
        assert!(!h.active());
        h.trigger(0.04);
        assert!(h.active());
        h.update(0.02);
        assert!(h.active());
        h.update(0.03); // total 0.05 > 0.04
        assert!(!h.active());
        assert_eq!(h.remaining(), 0.0);
    }

    #[test]
    fn test_hitstop_does_not_shorten() {
        let mut h = Hitstop::new();
        h.trigger(0.10);
        h.trigger(0.02);
        assert!((h.remaining() - 0.10).abs() < 1e-6);
    }

    #[test]
    fn test_particles_expire_after_lifetime() {
        let mut ps = Particles::new();
        let mut rng = Rng::new(1);
        ps.burst(Vec2::ZERO, 8, 200.0, &mut rng);
        assert_eq!(ps.len(), 8);
        ps.update(Particles::LIFETIME + 0.01);
        assert!(ps.is_empty());
    }

    #[test]
    fn test_particle_burst_count_matches() {
        let mut ps = Particles::new();
        let mut rng = Rng::new(99);
        ps.burst(Vec2::new(10.0, 20.0), 24, 300.0, &mut rng);
        assert_eq!(ps.len(), 24);
        // Every particle starts at the burst origin.
        for p in ps.iter() {
            assert!((p.pos - Vec2::new(10.0, 20.0)).length() < 1e-4);
        }
    }

    #[test]
    fn test_particles_clear() {
        let mut ps = Particles::new();
        let mut rng = Rng::new(3);
        ps.burst(Vec2::ZERO, 5, 100.0, &mut rng);
        ps.clear();
        assert!(ps.is_empty());
    }
}