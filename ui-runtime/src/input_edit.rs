//! Text buffer model for focused `field` nodes.
//!
//! Carries a selection anchor and a bounded undo/redo history so the boot shell
//! offers the editing subset users expect from a desktop text field: caret and
//! word motion, Shift-extended selection, select-all, cut/copy/paste over a
//! range, and Ctrl+Z / Ctrl+Shift+Z.

/// Undo depth kept per field. Bounded so a long-lived session cannot grow without limit.
pub const UNDO_DEPTH: usize = 64;

#[derive(Clone, Debug, Default, PartialEq)]
struct Snapshot {
    text: String,
    caret: usize,
}

#[derive(Clone, Debug, Default)]
pub struct TextBuffer {
    pub text: String,
    pub caret: usize,
    /// Selection anchor: `Some(i)` means text between `i` and `caret` is selected.
    pub anchor: Option<usize>,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
}

impl TextBuffer {
    pub fn from_props(text: &str, caret: Option<usize>) -> Self {
        let caret = clamp_boundary(text, caret.unwrap_or(text.len()));
        TextBuffer {
            text: text.to_string(),
            caret,
            anchor: None,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    /// Restore a buffer including its selection (used when props carry a range).
    pub fn from_props_with_selection(
        text: &str,
        caret: Option<usize>,
        anchor: Option<usize>,
    ) -> Self {
        let mut buf = Self::from_props(text, caret);
        buf.anchor = anchor
            .map(|a| clamp_boundary(text, a))
            .filter(|a| *a != buf.caret);
        buf
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            text: self.text.clone(),
            caret: self.caret,
        }
    }

    /// Record the pre-edit state so the change can be undone.
    fn checkpoint(&mut self) {
        let snap = self.snapshot();
        if self.undo.last() == Some(&snap) {
            return;
        }
        self.undo.push(snap);
        if self.undo.len() > UNDO_DEPTH {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    pub fn selection(&self) -> Option<(usize, usize)> {
        let anchor = self.anchor?;
        if anchor == self.caret {
            return None;
        }
        Some((anchor.min(self.caret), anchor.max(self.caret)))
    }

    pub fn selected_text(&self) -> String {
        match self.selection() {
            Some((start, end)) => self.text[start..end].to_string(),
            None => String::new(),
        }
    }

    pub fn select_all(&mut self) {
        if self.text.is_empty() {
            self.anchor = None;
            return;
        }
        self.anchor = Some(0);
        self.caret = self.text.len();
    }

    /// Delete the selection if there is one. Returns true when text changed.
    pub fn delete_selection(&mut self) -> bool {
        let Some((start, end)) = self.selection() else {
            return false;
        };
        self.checkpoint();
        self.text.replace_range(start..end, "");
        self.caret = start;
        self.anchor = None;
        true
    }

    pub fn insert(&mut self, ch: char) {
        if ch.is_control() && ch != '\n' {
            return;
        }
        if self.selection().is_some() {
            self.delete_selection();
        } else {
            self.checkpoint();
        }
        self.text.insert(self.caret, ch);
        self.caret += ch.len_utf8();
    }

    pub fn insert_str(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        if self.selection().is_some() {
            self.delete_selection();
        } else {
            self.checkpoint();
        }
        let filtered: String = s
            .chars()
            .filter(|c| !c.is_control() || *c == '\n')
            .collect();
        self.text.insert_str(self.caret, &filtered);
        self.caret += filtered.len();
    }

    pub fn backspace(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.caret == 0 {
            return;
        }
        self.checkpoint();
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
        self.checkpoint();
        let next = self.text[self.caret..]
            .chars()
            .next()
            .map(|c| c.len_utf8())
            .unwrap_or(1);
        self.text.replace_range(self.caret..self.caret + next, "");
    }

    /// Move the caret, extending or dropping the selection per `extend`.
    fn move_caret(&mut self, to: usize, extend: bool) {
        if extend {
            if self.anchor.is_none() {
                self.anchor = Some(self.caret);
            }
        } else {
            self.anchor = None;
        }
        self.caret = clamp_boundary(&self.text, to);
    }

    pub fn move_left_ext(&mut self, extend: bool) {
        let prev = self.text[..self.caret]
            .chars()
            .next_back()
            .map(|c| c.len_utf8())
            .unwrap_or(0);
        let to = self.caret.saturating_sub(prev);
        self.move_caret(to, extend);
    }

    pub fn move_right_ext(&mut self, extend: bool) {
        let next = self.text[self.caret..]
            .chars()
            .next()
            .map(|c| c.len_utf8())
            .unwrap_or(0);
        let to = (self.caret + next).min(self.text.len());
        self.move_caret(to, extend);
    }

    pub fn word_left(&self) -> usize {
        let head = &self.text[..self.caret];
        let trimmed = head.trim_end_matches(|c: char| c.is_whitespace());
        match trimmed.rfind(|c: char| c.is_whitespace()) {
            Some(pos) => {
                pos + head[pos..]
                    .chars()
                    .next()
                    .map(|c| c.len_utf8())
                    .unwrap_or(1)
            }
            None => 0,
        }
    }

    pub fn word_right(&self) -> usize {
        let tail = &self.text[self.caret..];
        let skipped: usize = tail
            .chars()
            .take_while(|c| c.is_whitespace())
            .map(|c| c.len_utf8())
            .sum();
        let rest = &tail[skipped..];
        match rest.find(|c: char| c.is_whitespace()) {
            Some(pos) => self.caret + skipped + pos,
            None => self.text.len(),
        }
    }

    /// Start of the visual line containing the caret (single-line fields → 0).
    pub fn line_start(&self) -> usize {
        self.text[..self.caret]
            .rfind('\n')
            .map(|p| p + 1)
            .unwrap_or(0)
    }

    pub fn line_end(&self) -> usize {
        self.text[self.caret..]
            .find('\n')
            .map(|p| self.caret + p)
            .unwrap_or(self.text.len())
    }

    pub fn undo(&mut self) -> bool {
        let Some(prev) = self.undo.pop() else {
            return false;
        };
        self.redo.push(self.snapshot());
        self.text = prev.text;
        self.caret = clamp_boundary(&self.text, prev.caret);
        self.anchor = None;
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo.pop() else {
            return false;
        };
        self.undo.push(self.snapshot());
        self.text = next.text;
        self.caret = clamp_boundary(&self.text, next.caret);
        self.anchor = None;
        true
    }

    /// Seed undo history from persisted props so Ctrl+Z survives a re-read.
    pub fn seed_history(&mut self, history: &[String]) {
        self.undo = history.iter().map(snapshot_of).collect();
    }

    /// Seed redo history so Ctrl+Shift+Z works after the buffer is re-read from
    /// props (each key event rebuilds the buffer from the node).
    pub fn seed_redo(&mut self, history: &[String]) {
        self.redo = history.iter().map(snapshot_of).collect();
    }

    pub fn history(&self) -> Vec<String> {
        self.undo.iter().map(|s| s.text.clone()).collect()
    }

    pub fn redo_history(&self) -> Vec<String> {
        self.redo.iter().map(|s| s.text.clone()).collect()
    }
}

fn snapshot_of(text: impl AsRef<str>) -> Snapshot {
    let text = text.as_ref();
    Snapshot {
        caret: text.len(),
        text: text.to_string(),
    }
}

fn clamp_boundary(text: &str, idx: usize) -> usize {
    let mut idx = idx.min(text.len());
    while idx > 0 && !text.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

/// Outcome of a key press against a focused field.
#[derive(Clone, Debug, PartialEq)]
pub enum EditOutcome {
    /// Buffer text and/or caret changed.
    Changed,
    /// Caret or selection moved but text is identical.
    Moved,
    /// Key belongs to something other than the field.
    Unhandled,
}

/// Map a key name + modifiers to an edit action.
pub fn apply_key_ext(
    buf: &mut TextBuffer,
    key: &str,
    text: Option<&str>,
    mods: &KeyMods,
) -> EditOutcome {
    let word = mods.ctrl || mods.alt;
    match key {
        "BackSpace" | "Backspace" => {
            if word && buf.selection().is_none() {
                let to = buf.word_left();
                buf.anchor = Some(buf.caret);
                buf.caret = to;
                buf.delete_selection();
            } else {
                buf.backspace();
            }
            EditOutcome::Changed
        }
        "Delete" => {
            if word && buf.selection().is_none() {
                let to = buf.word_right();
                buf.anchor = Some(buf.caret);
                buf.caret = to;
                buf.delete_selection();
            } else {
                buf.delete_forward();
            }
            EditOutcome::Changed
        }
        "ArrowLeft" | "Left" => {
            if word {
                let to = buf.word_left();
                buf.move_caret(to, mods.shift);
            } else {
                buf.move_left_ext(mods.shift);
            }
            EditOutcome::Moved
        }
        "ArrowRight" | "Right" => {
            if word {
                let to = buf.word_right();
                buf.move_caret(to, mods.shift);
            } else {
                buf.move_right_ext(mods.shift);
            }
            EditOutcome::Moved
        }
        // Single-line fields treat vertical motion as line-edge motion, which is
        // what a caret in a one-line field can honestly do.
        "ArrowUp" | "Up" | "PageUp" => {
            let to = if key == "PageUp" { 0 } else { buf.line_start() };
            buf.move_caret(to, mods.shift);
            EditOutcome::Moved
        }
        "ArrowDown" | "Down" | "PageDown" => {
            let to = if key == "PageDown" {
                buf.text.len()
            } else {
                buf.line_end()
            };
            buf.move_caret(to, mods.shift);
            EditOutcome::Moved
        }
        "Home" => {
            let to = if mods.ctrl { 0 } else { buf.line_start() };
            buf.move_caret(to, mods.shift);
            EditOutcome::Moved
        }
        "End" => {
            let to = if mods.ctrl {
                buf.text.len()
            } else {
                buf.line_end()
            };
            buf.move_caret(to, mods.shift);
            EditOutcome::Moved
        }
        "Enter" | "Return" | "Tab" | "Escape" => EditOutcome::Unhandled,
        _ => {
            if mods.ctrl || mods.meta || mods.alt {
                return EditOutcome::Unhandled;
            }
            if let Some(t) = text {
                if !t.is_empty() {
                    buf.insert_str(t);
                    return EditOutcome::Changed;
                }
            }
            // Single printable key name (e.g. "a", "A").
            if key.chars().count() == 1 {
                if let Some(ch) = key.chars().next() {
                    let ch = if mods.shift {
                        ch.to_uppercase().next().unwrap_or(ch)
                    } else if ch.is_ascii_uppercase() {
                        ch.to_ascii_lowercase()
                    } else {
                        ch
                    };
                    buf.insert(ch);
                    return EditOutcome::Changed;
                }
            }
            EditOutcome::Unhandled
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
impl KeyMods {
    pub fn shift() -> Self {
        KeyMods {
            shift: true,
            ..Default::default()
        }
    }

    pub fn ctrl() -> Self {
        KeyMods {
            ctrl: true,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buf(text: &str) -> TextBuffer {
        TextBuffer::from_props(text, Some(text.len()))
    }

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
        assert_eq!(
            apply_key_ext(&mut b, "a", Some("a"), &KeyMods::default()),
            EditOutcome::Changed
        );
        assert_eq!(b.text, "a");
    }

    #[test]
    fn chord_keys_are_left_to_the_shortcut_table() {
        let mut b = buf("hi");
        let mods = KeyMods {
            ctrl: true,
            ..Default::default()
        };
        assert_eq!(
            apply_key_ext(&mut b, "c", None, &mods),
            EditOutcome::Unhandled
        );
        assert_eq!(b.text, "hi");
    }

    #[test]
    fn shift_arrow_extends_selection() {
        let mut b = buf("hello");
        apply_key_ext(&mut b, "ArrowLeft", None, &KeyMods::shift());
        apply_key_ext(&mut b, "ArrowLeft", None, &KeyMods::shift());
        assert_eq!(b.selection(), Some((3, 5)));
        assert_eq!(b.selected_text(), "lo");
    }

    #[test]
    fn plain_arrow_collapses_selection() {
        let mut b = buf("hello");
        b.select_all();
        apply_key_ext(&mut b, "ArrowLeft", None, &KeyMods::default());
        assert!(b.selection().is_none());
    }

    #[test]
    fn typing_replaces_selection() {
        let mut b = buf("hello");
        b.select_all();
        b.insert_str("bye");
        assert_eq!(b.text, "bye");
        assert!(b.selection().is_none());
    }

    #[test]
    fn select_all_then_backspace_clears_text() {
        let mut b = buf("hello");
        b.select_all();
        b.backspace();
        assert_eq!(b.text, "");
    }

    #[test]
    fn word_motion_skips_words() {
        let mut b = buf("one two three");
        b.caret = b.word_left();
        assert_eq!(b.caret, 8);
        b.caret = 0;
        b.caret = b.word_right();
        assert_eq!(b.caret, 3);
    }

    #[test]
    fn ctrl_backspace_deletes_word() {
        let mut b = buf("one two");
        apply_key_ext(&mut b, "Backspace", None, &KeyMods::ctrl());
        assert_eq!(b.text, "one ");
    }

    #[test]
    fn home_end_and_page_keys_move_caret() {
        let mut b = buf("abcdef");
        apply_key_ext(&mut b, "Home", None, &KeyMods::default());
        assert_eq!(b.caret, 0);
        apply_key_ext(&mut b, "End", None, &KeyMods::default());
        assert_eq!(b.caret, 6);
        apply_key_ext(&mut b, "PageUp", None, &KeyMods::default());
        assert_eq!(b.caret, 0);
        apply_key_ext(&mut b, "PageDown", None, &KeyMods::default());
        assert_eq!(b.caret, 6);
    }

    #[test]
    fn arrow_up_down_hit_line_edges() {
        let mut b = buf("abc");
        apply_key_ext(&mut b, "ArrowUp", None, &KeyMods::default());
        assert_eq!(b.caret, 0);
        apply_key_ext(&mut b, "ArrowDown", None, &KeyMods::default());
        assert_eq!(b.caret, 3);
    }

    #[test]
    fn undo_and_redo_restore_text() {
        let mut b = TextBuffer::default();
        b.insert_str("hello");
        b.insert_str(" world");
        assert!(b.undo());
        assert_eq!(b.text, "hello");
        assert!(b.undo());
        assert_eq!(b.text, "");
        assert!(b.redo());
        assert_eq!(b.text, "hello");
        assert!(b.redo());
        assert_eq!(b.text, "hello world");
        assert!(!b.redo());
    }

    #[test]
    fn undo_depth_is_bounded() {
        let mut b = TextBuffer::default();
        for _ in 0..(UNDO_DEPTH + 20) {
            b.insert('x');
        }
        assert!(b.history().len() <= UNDO_DEPTH);
    }

    #[test]
    fn history_roundtrips_through_props() {
        let mut b = TextBuffer::default();
        b.insert_str("a");
        b.insert_str("b");
        let history = b.history();
        let mut restored = TextBuffer::from_props("ab", Some(2));
        restored.seed_history(&history);
        assert!(restored.undo());
        assert_eq!(restored.text, "a");
    }

    #[test]
    fn selection_survives_prop_roundtrip() {
        let b = TextBuffer::from_props_with_selection("hello", Some(5), Some(1));
        assert_eq!(b.selection(), Some((1, 5)));
    }

    #[test]
    fn multibyte_caret_stays_on_boundaries() {
        let mut b = TextBuffer::from_props("héllo", Some(2));
        b.move_left_ext(false);
        assert!(b.text.is_char_boundary(b.caret));
        b.insert('x');
        assert!(b.text.starts_with('x'));
    }

    #[test]
    fn control_chars_are_filtered_on_paste() {
        let mut b = TextBuffer::default();
        b.insert_str("a\u{7}b");
        assert_eq!(b.text, "ab");
    }

    #[test]
    fn shift_letter_uppercases() {
        let mut b = TextBuffer::default();
        let mods = KeyMods::shift();
        apply_key_ext(&mut b, "a", None, &mods);
        assert_eq!(b.text, "A");
    }
}
