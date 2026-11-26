// games/pagoda/src/controls.rs
//! Game-specific input mapping shared by the menu and the drop system.

use ember_input::{Input, Key};

/// The single Pagoda action: start, drop, or restart.
///
/// Space and Enter are keyboard aliases; the left mouse button covers
/// desktop clicks and the browser's touch-to-mouse mapping.
pub fn confirm_pressed(input: &Input) -> bool {
    input.is_key_pressed(Key::Space) || input.is_key_pressed(Key::Enter) || input.mouse_left_pressed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn space_confirms() {
        let input = Input {
            keys_pressed: vec![Key::Space],
            ..Default::default()
        };
        assert!(confirm_pressed(&input));
    }

    #[test]
    fn enter_confirms() {
        let input = Input {
            keys_pressed: vec![Key::Enter],
            ..Default::default()
        };
        assert!(confirm_pressed(&input));
    }

    #[test]
    fn left_click_confirms() {
        let input = Input {
            mouse_left_pressed: true,
            ..Default::default()
        };
        assert!(confirm_pressed(&input));
    }

    #[test]
    fn empty_input_does_not_confirm() {
        assert!(!confirm_pressed(&Input::default()));
    }
}
