// crates/ember-core/src/time/tick.rs
//! Timer à pas fixe.

/// Accumulateur de temps pour une simulation à pas fixe.
///
/// Utilisation typique :
/// ```ignore
/// let mut timer = TickTimer::new(0.1);  // un tick toutes les 100 ms
/// let ticks = timer.advance(dt);
/// for _ in 0..ticks {
///     // simulation d'un tick
/// }
/// ```
///
/// Le timer gère un **accumulateur** interne. Chaque appel à `advance(dt)`
/// ajoute `dt` à l'accumulateur, puis consomme autant de ticks que possible.
#[derive(Debug, Clone)]
pub struct TickTimer {
    accumulator: f32,
    tick_duration: f32,
}

impl TickTimer {
    /// Crée un timer avec une durée de tick donnée (en secondes).
    ///
    /// La durée est clampée à un minimum de `1e-6` pour éviter les
    /// divisions par zéro ou les ticks infiniment rapides.
    pub fn new(tick_duration: f32) -> Self {
        Self {
            accumulator: 0.0,
            tick_duration: tick_duration.max(1e-6),
        }
    }

    /// Change la durée du tick. N'affecte pas l'accumulateur en cours.
    pub fn set_duration(&mut self, duration: f32) {
        self.tick_duration = duration.max(1e-6);
    }

    /// Durée actuelle du tick (en secondes).
    pub fn duration(&self) -> f32 {
        self.tick_duration
    }

    /// Avance le timer de `dt` secondes.
    ///
    /// Retourne le nombre de ticks qui doivent être exécutés. Le premier
    /// tick a lieu quand l'accumulateur atteint `tick_duration`.
    ///
    /// Exemple : si `tick_duration = 0.1` et `dt = 0.25`, retourne `2`
    /// (l'accumulateur passe de 0.0 à 0.25, 2 ticks sont consommés, il
    /// reste 0.05 dans l'accumulateur).
    pub fn advance(&mut self, dt: f32) -> u32 {
        if dt < 0.0 {
            return 0;
        }
        self.accumulator += dt;
        let mut ticks = 0;
        while self.accumulator >= self.tick_duration {
            self.accumulator -= self.tick_duration;
            ticks += 1;
        }
        ticks
    }

    /// Réinitialise l'accumulateur à 0. La durée du tick est conservée.
    pub fn reset(&mut self) {
        self.accumulator = 0.0;
    }

    /// Progression dans le tick courant (entre 0.0 et 1.0).
    ///
    /// 0.0 = on vient de faire un tick, 1.0 = on est prêt pour le suivant.
    /// Utile pour interpoler une position entre deux ticks (rendu fluide).
    pub fn progress(&self) -> f32 {
        (self.accumulator / self.tick_duration).clamp(0.0, 1.0)
    }

    /// Temps accumulé depuis le dernier tick (en secondes).
    pub fn accumulator(&self) -> f32 {
        self.accumulator
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_initializes_zero() {
        let t = TickTimer::new(0.1);
        assert_eq!(t.accumulator(), 0.0);
        assert_eq!(t.duration(), 0.1);
        assert_eq!(t.progress(), 0.0);
    }

    #[test]
    fn test_advance_below_tick_duration_returns_zero() {
        let mut t = TickTimer::new(0.1);
        assert_eq!(t.advance(0.05), 0);
        assert!((t.accumulator() - 0.05).abs() < 1e-6);
    }

    #[test]
    fn test_advance_exactly_one_tick() {
        let mut t = TickTimer::new(0.1);
        assert_eq!(t.advance(0.1), 1);
        assert!(t.accumulator() < 1e-6);
    }

    #[test]
    fn test_advance_two_ticks() {
        let mut t = TickTimer::new(0.1);
        assert_eq!(t.advance(0.25), 2);
        assert!((t.accumulator() - 0.05).abs() < 1e-5);
    }

    #[test]
    fn test_accumulation_across_calls() {
        let mut t = TickTimer::new(0.1);
        assert_eq!(t.advance(0.05), 0);
        assert_eq!(t.advance(0.05), 1);   // 0.05 + 0.05 = 0.1 → 1 tick
    }

    #[test]
    fn test_reset_clears_accumulator() {
        let mut t = TickTimer::new(0.1);
        t.advance(0.07);
        assert!(t.accumulator() > 0.0);
        t.reset();
        assert_eq!(t.accumulator(), 0.0);
    }

    #[test]
    fn test_set_duration() {
        let mut t = TickTimer::new(0.1);
        t.set_duration(0.05);
        assert_eq!(t.duration(), 0.05);
    }

    #[test]
    fn test_progress_reaches_one() {
        let mut t = TickTimer::new(0.1);
        t.advance(0.099);
        assert!(t.progress() > 0.9);
        assert!(t.progress() < 1.0);
    }

    #[test]
    fn test_progress_wraps_after_tick() {
        let mut t = TickTimer::new(0.1);
        t.advance(0.15);   // 1 tick consommé, 0.05 restant
        assert!((t.progress() - 0.5).abs() < 1e-5);
    }

    #[test]
    fn test_negative_dt_is_safe() {
        let mut t = TickTimer::new(0.1);
        assert_eq!(t.advance(-1.0), 0);
        assert_eq!(t.accumulator(), 0.0);
    }

    #[test]
    fn test_duration_clamped_to_minimum() {
        let t = TickTimer::new(0.0);
        assert!(t.duration() >= 1e-6);
    }
}