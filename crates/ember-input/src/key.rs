//! Backend-agnostic keyboard key enum.
//!
//! Replaces `macroquad::KeyCode` at the engine boundary. The backend
//! (`ember-render-macroquad`) maps its native key codes to `Key`.
//!
//! `#[non_exhaustive]`: adding a variant later must not break a game's
//! `match` arms. Games match by equality (`is_key_pressed(Key::Space)`)
//! rather than exhaustive `match`, so this costs nothing.

/// A keyboard key.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    // Navigation / confirmation
    Space,
    Enter,
    Escape,
    Tab,
    Backspace,

    // Arrows (many games use these instead of WASD)
    Up,
    Down,
    Left,
    Right,

    // Letters
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,

    // Digits
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,

    // Function keys (debug toggles)
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,

    // Backend-specific key, for keys this enum does not model.
    // Backends pass their own code through; games rarely use this.
    Unknown(u32),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_copy_and_comparable() {
        let a = Key::Space;
        let b = a; // Copy
        assert_eq!(a, b);
    }

    #[test]
    fn unknown_carries_a_code() {
        assert_ne!(Key::Unknown(1), Key::Unknown(2));
        assert_eq!(Key::Unknown(42), Key::Unknown(42));
    }

    #[test]
    fn distinct_keys_differ() {
        assert_ne!(Key::Space, Key::Enter);
        assert_ne!(Key::A, Key::B);
        assert_ne!(Key::Num0, Key::Num1);
    }
}
