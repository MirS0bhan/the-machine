//! Text buffer model for focused `field` nodes.

#[derive(Clone, Debug, Default)]
pub struct TextBuffer {
    pub text: String,
    pub caret: usize,
    pub sel_anchor: Option<usize>,
}

impl TextBuffer {
    pub fn from_props(text: &str, caret: Option<usize>) -> Self {
        let caret = caret.unwrap_or(text.len()).min(text.len());
        TextBuffer {
            text: text.to_string(),
            caret,
            sel_anchor: None,
        }
    }

    pub fn from_props_sel(text: &str, caret: Option<usize>, sel_anchor: Option<usize>) -> Self {
        let mut b = Self::from_props(text, caret);
        b.sel_anchor = sel_anchor.map(|s| s.min(b.text.len()));
        b
    }

    pub fn selection_range(&self) -> Option<(usize, usize)> {
        let a = self.sel_anchor?;
        if a == self.caret {
            return None;
        }
        Some((a.min(self.caret), a.max(self.caret)))
    }

    pub fn selected_text(&self) -> Option<&str> {
        self.selection_range().map(|(s, e)| &self.text[s..e])
    }

    pub fn delete_selection(&mut self) -> bool {
        if let Some((s, e)) = self.selection_range() {
            self.text.replace_range(s..e, "");
            self.caret = s;
            self.sel_anchor = None;
            return true;
        }
        false
    }

    pub fn select_all(&mut self) {
        self.sel_anchor = Some(0);
        self.caret = self.text.len();
    }

    pub fn insert(&mut self, ch: char) {
        if ch.is_control() && ch != '\n' {
            return;
        }
        self.delete_selection();
        self.text.insert(self.caret, ch);
        self.caret += ch.len_utf8();
        self.sel_anchor = None;
    }

    pub fn insert_str(&mut self, s: &str) {
        self.delete_selection();
        for ch in s.chars() {
            if ch.is_control() && ch != '\n' {
                continue;
            }
            self.text.insert(self.caret, ch);
            self.caret += ch.len_utf8();
        }
        self.sel_anchor = None;
    }

    pub fn backspace(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.caret == 0 {
            return;
        }
        let prev = self.text[..self.caret]
            .chars()
            .next_back()
            .map(|c| c.len_utf8())
            .unwrap_or(1);
        let start = self.caret - prev;
        self.text.replace_range(start..self.caret, "");
        self.caret = start;
    }

    pub fn delete_forward(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.caret >= self.text.len() {
            return;
        }
        let next = self.text[self.caret..]
            .chars()
            .next()
            .map(|c| c.len_utf8())
            .unwrap_or(1);
        self.text.replace_range(self.caret..self.caret + next, "");
    }

    pub fn move_left(&mut self) {
        self.sel_anchor = None;
        if self.caret == 0 {
            return;
        }
        let prev = self.text[..self.caret]
            .chars()
            .next_back()
            .map(|c| c.len_utf8())
            .unwrap_or(1);
        self.caret -= prev;
    }

    pub fn move_right(&mut self) {
        self.sel_anchor = None;
        if self.caret >= self.text.len() {
            return;
        }
        let next = self.text[self.caret..]
            .chars()
            .next()
            .map(|c| c.len_utf8())
            .unwrap_or(1);
        self.caret += next;
    }
}

/// Map a key name + modifiers to an edit action. Returns true if handled.
pub fn apply_key(buf: &mut TextBuffer, key: &str, text: Option<&str>, mods: &KeyMods) -> bool {
    if mods.ctrl || mods.alt || mods.meta {
        return false;
    }
    match key {
        "BackSpace" | "Backspace" => {
            buf.backspace();
            true
        }
        "Delete" => {
            buf.delete_forward();
            true
        }
        "ArrowLeft" | "Left" => {
            buf.move_left();
            true
        }
        "ArrowRight" | "Right" => {
            buf.move_right();
            true
        }
        "Home" => {
            buf.caret = 0;
            true
        }
        "End" => {
            buf.caret = buf.text.len();
            true
        }
        "Enter" | "Return" | "Tab" | "Escape" => false,
        _ => {
            if let Some(t) = text {
                if !t.is_empty() {
                    buf.insert_str(t);
                    return true;
                }
            }
            // Single printable key name (e.g. "a", "A").
            if key.len() == 1 {
                if let Some(ch) = key.chars().next() {
                    let ch = if mods.shift {
                        ch
                    } else {
                        ch.to_ascii_lowercase()
                    };
                    buf.insert(ch);
                    return true;
                }
            }
            false
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct KeyMods {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_backspace() {
        let mut b = TextBuffer::default();
        b.insert_str("Hi");
        assert_eq!(b.text, "Hi");
        assert_eq!(b.caret, 2);
        b.backspace();
        assert_eq!(b.text, "H");
        assert_eq!(b.caret, 1);
    }

    #[test]
    fn apply_key_types_letter() {
        let mut b = TextBuffer::default();
        assert!(apply_key(&mut b, "a", Some("a"), &KeyMods::default()));
        assert_eq!(b.text, "a");
    }

    #[test]
    fn select_all_and_delete() {
        let mut b = TextBuffer::from_props("hello", None);
        b.select_all();
        assert_eq!(b.selected_text(), Some("hello"));
        b.delete_selection();
        assert_eq!(b.text, "");
    }

    #[test]
    fn home_end_move_caret() {
        let mut b = TextBuffer::from_props("ab", None);
        apply_key(&mut b, "Home", None, &KeyMods::default());
        assert_eq!(b.caret, 0);
        apply_key(&mut b, "End", None, &KeyMods::default());
        assert_eq!(b.caret, 2);
    }
}
