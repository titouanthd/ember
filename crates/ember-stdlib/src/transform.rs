use glam::Vec2;

/// Un transformé 2D : position, rotation (en radians), échelle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub position: Vec2,
    pub rotation: f32,
    pub scale: Vec2,
}

impl Transform {
    /// Crée un nouveau transformé.
    pub fn new(position: Vec2, rotation: f32, scale: Vec2) -> Self {
        Self { position, rotation, scale }
    }

    /// Transformé identité (position (0,0), rotation 0, échelle (1,1)).
    pub fn identity() -> Self {
        Self {
            position: Vec2::ZERO,
            rotation: 0.0,
            scale: Vec2::ONE,
        }
    }

    /// Déplace la position.
    pub fn translate(&mut self, delta: Vec2) {
        self.position += delta;
    }

    /// Ajoute un angle à la rotation.
    pub fn rotate(&mut self, angle: f32) {
        self.rotation += angle;
    }

    /// Remplace l'échelle.
    pub fn set_scale(&mut self, scale: Vec2) {
        self.scale = scale;
    }

    /// Centre de la transform : `position + scale / 2.0`.
    ///
    /// Convention top-left (sauf Bullet Hell qui utilise position = centre,
    /// auquel cas cette méthode n'est pas utilisée).
    pub fn center(&self) -> Vec2 {
        self.position + self.scale / 2.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_transform() {
        let t = Transform::identity();
        assert_eq!(t.position, Vec2::ZERO);
        assert_eq!(t.rotation, 0.0);
        assert_eq!(t.scale, Vec2::ONE);
    }

    #[test]
    fn test_translate_modifies_position() {
        let mut t = Transform::identity();
        t.translate(Vec2::new(5.0, 3.0));
        assert_eq!(t.position, Vec2::new(5.0, 3.0));
    }

    #[test]
    fn test_rotate_modifies_rotation() {
        let mut t = Transform::identity();
        t.rotate(1.2);
        assert_eq!(t.rotation, 1.2);
    }

    #[test]
    fn test_set_scale_modifies_scale() {
        let mut t = Transform::identity();
        t.set_scale(Vec2::new(2.0, 0.5));
        assert_eq!(t.scale, Vec2::new(2.0, 0.5));
    }

    #[test]
    fn test_new_transform() {
        let t = Transform::new(Vec2::new(10.0, 20.0), 0.5, Vec2::new(2.0, 2.0));
        assert_eq!(t.position, Vec2::new(10.0, 20.0));
        assert_eq!(t.rotation, 0.5);
        assert_eq!(t.scale, Vec2::new(2.0, 2.0));
    }

    #[test]
    fn test_center_top_left_convention() {
        let t = Transform::new(Vec2::new(100.0, 200.0), 0.0, Vec2::new(20.0, 80.0));
        assert_eq!(t.center(), Vec2::new(110.0, 240.0));
    }

    #[test]
    fn test_center_identity() {
        let t = Transform::identity();
        assert_eq!(t.center(), Vec2::new(0.5, 0.5));
    }
}