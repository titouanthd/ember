//! Petits helpers vectoriels absents de glam.

use glam::Vec2;

/// Réflexion miroir de `v` autour de la normale `n`.
///
/// Formule standard : `v' = v − 2·(v·n)·n`. `n` **doit être unitaire** ;
/// si ce n'est pas le cas, normaliser d'abord (le résultat sinon n'a pas
/// de sens géométrique).
///
/// Équivalent à `Vec3::reflect` de glam, qui n'existe pas pour `Vec2`.
pub fn reflect(v: Vec2, n: Vec2) -> Vec2 {
    v - 2.0 * v.dot(n) * n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reflect_horizontal_normal() {
        let v = Vec2::new(-100.0, 50.0);
        let n = Vec2::new(1.0, 0.0);
        assert_eq!(reflect(v, n), Vec2::new(100.0, 50.0));
    }

    #[test]
    fn reflect_vertical_normal() {
        let v = Vec2::new(100.0, -50.0);
        let n = Vec2::new(0.0, 1.0);
        assert_eq!(reflect(v, n), Vec2::new(100.0, 50.0));
    }

    #[test]
    fn reflect_preserves_tangential() {
        // Composante tangentielle (ici y) inchangée, composante normale flip.
        let v = Vec2::new(-100.0, 50.0);
        let n = Vec2::new(1.0, 0.0);
        let r = reflect(v, n);
        assert_eq!(r.y, v.y);
        assert_eq!(r.x, -v.x);
    }

    #[test]
    fn reflect_is_involution() {
        // Réfléchir deux fois = identité.
        let v = Vec2::new(3.0, -4.0);
        let n = Vec2::new(1.0, 1.0).normalize();
        let r = reflect(reflect(v, n), n);
        assert!((r - v).length() < 1e-5);
    }

    #[test]
    fn reflect_parallel_to_normal_flips() {
        let v = Vec2::new(5.0, 0.0);
        let n = Vec2::new(-1.0, 0.0);
        assert_eq!(reflect(v, n), Vec2::new(-5.0, 0.0));
    }

    #[test]
    fn reflect_perpendicular_to_normal_is_noop() {
        let v = Vec2::new(0.0, 5.0);
        let n = Vec2::new(1.0, 0.0);
        assert_eq!(reflect(v, n), v);
    }
}