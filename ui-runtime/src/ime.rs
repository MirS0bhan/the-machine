//! Simple compose-key / dead-key IME (P2). Full OS IME buses remain deferred.

use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct ImeState {
    /// Pending dead key, if any (e.g. "'", "`", "^", "\"").
    pub pending: Option<String>,
}

#[derive(Clone, Debug)]
pub enum ImeOutput {
    /// Commit this text into the field.
    Commit(String),
    /// Key was absorbed into compose state (no text yet).
    Pending,
    /// Not an IME sequence — fall through to normal typing.
    Pass,
}

impl ImeState {
    pub fn reset(&mut self) {
        self.pending = None;
    }

    /// Feed a key name + optional text. Returns how the editor should proceed.
    pub fn feed(&mut self, key: &str, text: Option<&str>) -> ImeOutput {
        // Explicit compose cancel.
        if matches!(key, "Escape" | "Esc") {
            if self.pending.is_some() {
                self.pending = None;
                return ImeOutput::Pending;
            }
            return ImeOutput::Pass;
        }

        let ch = text.and_then(|t| t.chars().next()).or_else(|| {
            if key.len() == 1 {
                key.chars().next()
            } else {
                None
            }
        });

        if let Some(dead) = self.pending.clone() {
            // Multi_key: first char selects compose dead, second char completes.
            if dead == "multi" {
                if let Some(c) = ch {
                    self.pending = Some(format!("multi:{c}"));
                    return ImeOutput::Pending;
                }
                return ImeOutput::Pass;
            }
            if let Some(rest) = dead.strip_prefix("multi:") {
                self.pending = None;
                if let Some(c) = ch {
                    if let Some(composed) = compose(rest, c) {
                        return ImeOutput::Commit(composed);
                    }
                    return ImeOutput::Commit(format!("{rest}{c}"));
                }
                return ImeOutput::Pass;
            }

            self.pending = None;
            if let Some(c) = ch {
                if let Some(composed) = compose(&dead, c) {
                    return ImeOutput::Commit(composed);
                }
                // Failed compose: emit dead + char.
                return ImeOutput::Commit(format!("{dead}{c}"));
            }
            return ImeOutput::Pass;
        }

        // Start dead-key sequence.
        if matches!(key, "DeadAcute" | "dead_acute") || text == Some("´") {
            self.pending = Some("'".into());
            return ImeOutput::Pending;
        }
        if matches!(key, "DeadGrave" | "dead_grave")
            && (key.starts_with("Dead") || key.starts_with("dead"))
        {
            self.pending = Some("`".into());
            return ImeOutput::Pending;
        }
        if matches!(key, "DeadCircumflex" | "dead_circumflex")
            && (key.starts_with("Dead") || key.starts_with("dead"))
        {
            self.pending = Some("^".into());
            return ImeOutput::Pending;
        }
        if matches!(key, "DeadDiaeresis" | "dead_diaeresis") {
            self.pending = Some("\"".into());
            return ImeOutput::Pending;
        }
        if matches!(key, "DeadTilde" | "dead_tilde") {
            self.pending = Some("~".into());
            return ImeOutput::Pending;
        }

        // Compose key then two characters.
        if key == "Multi_key" {
            self.pending = Some("multi".into());
            return ImeOutput::Pending;
        }

        ImeOutput::Pass
    }
}

fn compose(dead: &str, ch: char) -> Option<String> {
    let table = compose_table();
    table.get(&(dead.to_string(), ch)).cloned()
}

fn compose_table() -> HashMap<(String, char), String> {
    let mut m = HashMap::new();
    let pairs = [
        ("'", 'a', "á"),
        ("'", 'e', "é"),
        ("'", 'i', "í"),
        ("'", 'o', "ó"),
        ("'", 'u', "ú"),
        ("'", 'A', "Á"),
        ("'", 'E', "É"),
        ("'", 'I', "Í"),
        ("'", 'O', "Ó"),
        ("'", 'U', "Ú"),
        ("`", 'a', "à"),
        ("`", 'e', "è"),
        ("`", 'i', "ì"),
        ("`", 'o', "ò"),
        ("`", 'u', "ù"),
        ("^", 'a', "â"),
        ("^", 'e', "ê"),
        ("^", 'i', "î"),
        ("^", 'o', "ô"),
        ("^", 'u', "û"),
        ("\"", 'a', "ä"),
        ("\"", 'e', "ë"),
        ("\"", 'i', "ï"),
        ("\"", 'o', "ö"),
        ("\"", 'u', "ü"),
        ("\"", 'A', "Ä"),
        ("\"", 'O', "Ö"),
        ("\"", 'U', "Ü"),
        ("~", 'n', "ñ"),
        ("~", 'N', "Ñ"),
        (",", 'c', "ç"),
        (",", 'C', "Ç"),
    ];
    for (d, c, out) in pairs {
        m.insert((d.into(), c), out.into());
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acute_e() {
        let mut ime = ImeState::default();
        assert!(matches!(
            ime.feed("DeadAcute", Some("´")),
            ImeOutput::Pending
        ));
        match ime.feed("e", Some("e")) {
            ImeOutput::Commit(s) => assert_eq!(s, "é"),
            other => panic!("expected commit, got {other:?}"),
        }
    }

    #[test]
    fn multi_key_apostrophe_e() {
        let mut ime = ImeState::default();
        assert!(matches!(ime.feed("Multi_key", None), ImeOutput::Pending));
        assert!(matches!(
            ime.feed("apostrophe", Some("'")),
            ImeOutput::Pending
        ));
        match ime.feed("e", Some("e")) {
            ImeOutput::Commit(s) => assert_eq!(s, "é"),
            other => panic!("expected commit, got {other:?}"),
        }
    }
}
