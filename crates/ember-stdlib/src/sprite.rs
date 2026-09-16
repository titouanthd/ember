use macroquad::prelude::Color;
/// Sera utilisé pour gérer des textures et des sprites dans le futur, mais pour l'instant il ne contient que la couleur et la visibilité.
/// Composant de rendu : définit la couleur et la visibilité d'une entité.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sprite {
    pub color: Color,
    pub visible: bool,
}

impl Sprite {
    /// Crée un nouveau sprite avec une couleur donnée.
    pub fn new(color: Color) -> Self {
        Self {
            color,
            visible: true,
        }
    }

    /// Crée un sprite blanc par défaut.
    pub fn white() -> Self {
        Self::new(Color::new(1.0, 1.0, 1.0, 1.0))
    }

    /// Masque le sprite.
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Affiche le sprite.
    pub fn show(&mut self) {
        self.visible = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sprite_new() {
        let color = Color::new(0.5, 0.8, 0.2, 1.0);
        let sprite = Sprite::new(color);
        assert_eq!(sprite.color, color);
        assert!(sprite.visible);
    }

    #[test]
    fn test_sprite_white() {
        let sprite = Sprite::white();
        assert_eq!(sprite.color, Color::new(1.0, 1.0, 1.0, 1.0));
        assert!(sprite.visible);
    }

    #[test]
    fn test_sprite_hide_show() {
        let mut sprite = Sprite::white();
        sprite.hide();
        assert!(!sprite.visible);
        sprite.show();
        assert!(sprite.visible);
    }
}
