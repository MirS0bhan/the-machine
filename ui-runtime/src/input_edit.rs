//! Text buffer model for focused `field` nodes.

#[derive(Clone, Debug, Default)]
pub struct TextBuffer {
    pub text: String,
    pub caret: usize,
}

impl TextBuffer {
    pub fn from_props(text: &str, caret: Option<usize>) -> Self {
        let caret = caret.unwrap_or(text.len()).min(text.len());
        TextBuffer {
            text: text.to_string(),
            caret,
        }
    }

    pub fn insert(&mut self, ch: char) {
        if ch.is_control() && ch != '\n' {
            return;
        }
        self.text.insert(self.caret, ch);
        self.caret += ch.len_utf8();
    }

    pub fn insert_str(&mut self, s: &str) {
        for ch in s.chars() {
            self.insert(ch);
        }
    }

    pub fn backspace(&mut self) {
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
}
