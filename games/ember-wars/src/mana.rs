//! Pool de Mana — ressource de spawn, régénère passivement.

#[derive(Debug, Clone)]
pub struct ManaPool {
    pub current: f32,
    pub max: f32,
    pub regen: f32,
}

impl ManaPool {
    pub fn new(start: f32, max: f32, regen: f32) -> Self {
        Self {
            current: start.min(max),
            max,
            regen,
        }
    }

    /// Fait avancer la régénération. Clampe à `max`.
    pub fn tick(&mut self, dt: f32) {
        if dt > 0.0 {
            self.current = (self.current + self.regen * dt).min(self.max);
        }
    }

    /// Tente de dépenser `amount`. Retourne `true` si la dépense a eu lieu.
    pub fn try_spend(&mut self, amount: f32) -> bool {
        if self.current >= amount {
            self.current -= amount;
            true
        } else {
            false
        }
    }

    /// Ajoute `amount`, clampé au max.
    pub fn gain(&mut self, amount: f32) {
        self.current = (self.current + amount).min(self.max);
    }

    /// Fraction [0, 1] pour l'affichage.
    pub fn fraction(&self) -> f32 {
        if self.max <= 0.0 {
            0.0
        } else {
            (self.current / self.max).clamp(0.0, 1.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_regenerates_and_clamps() {
        let mut pool = ManaPool::new(0.0, 100.0, 10.0);
        pool.tick(1.0);
        assert!((pool.current - 10.0).abs() < 1e-6);
        pool.tick(100.0);
        assert!((pool.current - 100.0).abs() < 1e-6);
    }

    #[test]
    fn try_spend_respects_balance() {
        let mut pool = ManaPool::new(30.0, 100.0, 0.0);
        assert!(pool.try_spend(20.0));
        assert!((pool.current - 10.0).abs() < 1e-6);
        assert!(!pool.try_spend(20.0));
        assert!((pool.current - 10.0).abs() < 1e-6);
    }

    #[test]
    fn gain_clamps_to_max() {
        let mut pool = ManaPool::new(95.0, 100.0, 0.0);
        pool.gain(50.0);
        assert!((pool.current - 100.0).abs() < 1e-6);
    }

    #[test]
    fn fraction_is_bounded() {
        let mut pool = ManaPool::new(50.0, 100.0, 0.0);
        assert!((pool.fraction() - 0.5).abs() < 1e-6);
        pool.gain(1000.0);
        assert!((pool.fraction() - 1.0).abs() < 1e-6);
    }
}