//! Gamepad buttons. v0.1 models face buttons + shoulders + start/select
//! + d-pad. Analog sticks and triggers are v0.2 — no game uses them yet.

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum GamepadButton {
    /// The "confirm" button — South (Xbox A, PS Cross, Switch B).
    /// Part of `primary_action()` per ADR-010.
    South,
    /// The "cancel" button — East (Xbox B, PS Circle, Switch A).
    /// Part of `secondary_action()` per ADR-010.
    East,
    North,
    West,
    Start,
    Select,
    LeftBumper,
    RightBumper,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn south_and_east_are_distinct() {
        assert_ne!(GamepadButton::South, GamepadButton::East);
    }

    #[test]
    fn dpad_directions_are_distinct() {
        let all = [
            GamepadButton::DPadUp,
            GamepadButton::DPadDown,
            GamepadButton::DPadLeft,
            GamepadButton::DPadRight,
        ];
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }
}
