//! Mouse buttons.

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buttons_are_distinct() {
        assert_ne!(MouseButton::Left, MouseButton::Right);
        assert_ne!(MouseButton::Left, MouseButton::Middle);
    }

    #[test]
    fn buttons_are_copy() {
        let b = MouseButton::Left;
        let c = b;
        assert_eq!(b, c);
    }
}
