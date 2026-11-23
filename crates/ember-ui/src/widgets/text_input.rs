//! `TextInput`: a single-line text field (spec §5.5).

use crate::{Control, ControlBase, Measure, TextChanged, TextSubmitted, UiDrawContext};
use ember_core::math::Rect;
use ember_input::Key;
use glam::Vec2;

/// Padding inside the field, in pixels.
pub const TEXT_INPUT_PADDING: f32 = 6.0;
/// Default width when no `min_size` is set.
pub const TEXT_INPUT_WIDTH: f32 = 200.0;

/// Focus it (click, Tab, `request_focus`), then type. Left / Right /
/// Home / End move the caret; Backspace / Delete erase; Enter emits
/// [`TextSubmitted`]. Every edit emits [`TextChanged`]. Space types a
/// space (the field [captures text](Control::captures_text)).
///
/// The caret is a character index. No selection, clipboard or IME in
/// v0.1.
pub struct TextInput {
    name: String,
    base: ControlBase,
    text: String,
    caret: usize,
    /// Shown dimmed while the text is empty.
    pub placeholder: String,
    /// Maximum number of characters; `None` = unlimited.
    pub max_len: Option<usize>,
    /// `None` = `Theme::font_size`.
    pub size: Option<f32>,
}

impl TextInput {
    pub fn new() -> Self {
        Self {
            name: "TextInput".into(),
            base: ControlBase::new(),
            text: String::new(),
            caret: 0,
            placeholder: String::new(),
            max_len: None,
            size: None,
        }
    }

    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.set_text(text);
        self
    }

    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn with_max_len(mut self, max_len: usize) -> Self {
        self.max_len = Some(max_len);
        self
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// Replace the text and put the caret at the end. Emits nothing.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        if let Some(max) = self.max_len {
            self.text = self.text.chars().take(max).collect();
        }
        self.caret = self.len();
    }

    /// Caret position, in characters from the start.
    pub fn caret(&self) -> usize {
        self.caret
    }

    fn len(&self) -> usize {
        self.text.chars().count()
    }

    /// Byte offset of character index `i`.
    fn byte(&self, i: usize) -> usize {
        self.text
            .char_indices()
            .nth(i)
            .map_or(self.text.len(), |(b, _)| b)
    }

    fn changed(&mut self) {
        let text = self.text.clone();
        self.base.emit(move |input| TextChanged { input, text });
    }

    fn inner(&self) -> Rect {
        let r = self.rect();
        Rect::new(
            r.x + TEXT_INPUT_PADDING,
            r.y + TEXT_INPUT_PADDING,
            (r.w - 2.0 * TEXT_INPUT_PADDING).max(0.0),
            (r.h - 2.0 * TEXT_INPUT_PADDING).max(0.0),
        )
    }
}

impl Default for TextInput {
    fn default() -> Self {
        Self::new()
    }
}

crate::widgets::ui_object!(TextInput, "TextInput");

impl Control for TextInput {
    fn base(&self) -> &ControlBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ControlBase {
        &mut self.base
    }

    fn measure(&self, _available: Vec2, m: &Measure) -> Vec2 {
        let size = self.size.unwrap_or(m.theme().font_size);
        let line = m.text("Mg", size).y;
        Vec2::new(TEXT_INPUT_WIDTH, line + 2.0 * TEXT_INPUT_PADDING).max(self.min_size())
    }

    fn focusable(&self) -> bool {
        true
    }

    fn captures_text(&self) -> bool {
        true
    }

    fn on_text_input(&mut self, ch: char) {
        if self.max_len.is_some_and(|max| self.len() >= max) {
            return;
        }
        let at = self.byte(self.caret);
        self.text.insert(at, ch);
        self.caret += 1;
        self.changed();
    }

    fn on_key_press(&mut self, key: Key) {
        match key {
            Key::Left => self.caret = self.caret.saturating_sub(1),
            Key::Right => self.caret = (self.caret + 1).min(self.len()),
            Key::Home => self.caret = 0,
            Key::End => self.caret = self.len(),
            Key::Backspace if self.caret > 0 => {
                self.caret -= 1;
                let at = self.byte(self.caret);
                self.text.remove(at);
                self.changed();
            }
            Key::Delete if self.caret < self.len() => {
                let at = self.byte(self.caret);
                self.text.remove(at);
                self.changed();
            }
            _ => {}
        }
    }

    /// Enter (Space types instead, see `captures_text`).
    fn on_activate(&mut self) {
        let text = self.text.clone();
        self.base.emit(move |input| TextSubmitted { input, text });
    }

    fn draw_control(&self, ui: &mut UiDrawContext) {
        let rect = self.rect();
        let theme = ui.theme();
        let bg = theme.panel_bg;
        let (border_width, border) = if ui.is_focused() {
            (2.0, theme.focus_border)
        } else {
            (1.0, theme.panel_border)
        };
        let size = self.size.unwrap_or(theme.font_size);
        let (shown, color) = if self.text.is_empty() {
            (self.placeholder.as_str(), theme.text_dim)
        } else {
            (self.text.as_str(), theme.text)
        };
        let text_color = theme.text;
        ui.draw_background(rect, bg);
        ui.draw_border(rect, border_width, border);

        let inner = self.inner();
        let full = ui.render().measure_text(shown, size);
        // Scroll the text left so the caret stays inside the field.
        let caret_x = if self.text.is_empty() {
            0.0
        } else {
            ui.render()
                .measure_text(&self.text[..self.byte(self.caret)], size)
                .width
        };
        let shift = (caret_x - inner.w).max(0.0);
        let top = inner.y + (inner.h - full.height) * 0.5;
        ui.push_clip(inner);
        if !shown.is_empty() {
            let baseline = Vec2::new(inner.x - shift, top + full.offset_y);
            ui.draw_text(shown, baseline, size, color);
        }
        if ui.is_focused() {
            let x = inner.x + caret_x - shift;
            ui.draw_background(Rect::new(x, top, 1.0, full.height.max(size)), text_color);
        }
    }
}

crate::control_node!(TextInput);

#[cfg(test)]
mod tests {
    use super::*;

    fn typed(s: &str) -> TextInput {
        let mut t = TextInput::new();
        for ch in s.chars() {
            t.on_text_input(ch);
        }
        t
    }

    #[test]
    fn typing_inserts_at_the_caret_and_queues_changes() {
        let mut t = typed("hllo");
        assert_eq!(t.base().queued_signals(), 4);
        t.on_key_press(Key::Home);
        t.on_key_press(Key::Right);
        t.on_text_input('e');
        assert_eq!(t.text(), "hello");
        assert_eq!(t.caret(), 2);
    }

    #[test]
    fn backspace_and_delete_handle_multibyte_chars() {
        let mut t = typed("aé€b");
        t.on_key_press(Key::Left);
        t.on_key_press(Key::Backspace);
        assert_eq!(t.text(), "aéb");
        t.on_key_press(Key::Home);
        t.on_key_press(Key::Delete);
        assert_eq!(t.text(), "éb");
        t.on_key_press(Key::End);
        t.on_key_press(Key::Delete);
        assert_eq!(t.text(), "éb", "Delete at the end does nothing");
        assert_eq!(t.caret(), 2);
    }

    #[test]
    fn caret_stays_in_bounds() {
        let mut t = typed("ab");
        t.on_key_press(Key::Right);
        assert_eq!(t.caret(), 2);
        t.on_key_press(Key::Home);
        t.on_key_press(Key::Left);
        t.on_key_press(Key::Backspace);
        assert_eq!((t.caret(), t.text()), (0, "ab"));
    }

    #[test]
    fn max_len_stops_typing() {
        let mut t = TextInput::new().with_max_len(3);
        for ch in "abcdef".chars() {
            t.on_text_input(ch);
        }
        assert_eq!(t.text(), "abc");
        assert_eq!(
            TextInput::new().with_max_len(2).with_text("xyz").text(),
            "xy"
        );
    }
}
