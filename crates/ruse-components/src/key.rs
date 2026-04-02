use ruse_runtime::{KeyCode, KeyEvent, Modifiers};

/// Help text associated with a key binding.
#[derive(Clone, Debug, Default)]
pub struct BindingHelp {
    pub key: String,
    pub desc: String,
}

/// A key binding that maps one or more key strings to an action.
#[derive(Clone, Debug)]
pub struct Binding {
    pub keys: Vec<String>,
    pub help: BindingHelp,
    pub enabled: bool,
}

impl Binding {
    pub fn new(keys: &[&str], help_key: &str, help_desc: &str) -> Self {
        Self {
            keys: keys.iter().map(|s| s.to_string()).collect(),
            help: BindingHelp {
                key: help_key.to_string(),
                desc: help_desc.to_string(),
            },
            enabled: true,
        }
    }

    /// Returns the help key string for backwards compatibility.
    pub fn help_key(&self) -> &str {
        &self.help.key
    }

    /// Returns the help description string for backwards compatibility.
    pub fn help_desc(&self) -> &str {
        &self.help.desc
    }

    pub fn enabled(&self) -> bool {
        self.enabled && !self.keys.is_empty()
    }

    pub fn set_enabled(&mut self, v: bool) {
        self.enabled = v;
    }

    /// Clear all keys and help text, effectively disabling this binding.
    pub fn unbind(&mut self) {
        self.keys.clear();
        self.help = BindingHelp::default();
    }

    /// Builder method to set help text.
    pub fn with_help(mut self, key: &str, desc: &str) -> Self {
        self.help = BindingHelp {
            key: key.to_string(),
            desc: desc.to_string(),
        };
        self
    }

    /// Check if a key event matches any of the binding's key strings.
    pub fn matches(&self, key: &KeyEvent) -> bool {
        if !self.enabled || self.keys.is_empty() {
            return false;
        }
        for binding_str in &self.keys {
            if key_string_matches(key, binding_str) {
                return true;
            }
        }
        false
    }
}

/// Check if a key event matches any of the provided bindings.
pub fn key_matches(key: &KeyEvent, bindings: &[&Binding]) -> bool {
    bindings.iter().any(|b| b.matches(key))
}

/// A trait for components that have key bindings.
pub trait KeyMap {
    fn short_help(&self) -> Vec<&Binding>;
    fn full_help(&self) -> Vec<Vec<&Binding>>;
}

/// Parse a key string like "ctrl+c", "shift+tab", "enter", "q", "up" etc.
/// and compare against a KeyEvent.
fn key_string_matches(key: &KeyEvent, s: &str) -> bool {
    let s = s.to_lowercase();
    let parts: Vec<&str> = s.split('+').collect();

    let mut expected_mods = Modifiers::empty();
    let mut key_part = "";

    for part in &parts {
        match *part {
            "ctrl" | "control" => expected_mods |= Modifiers::CTRL,
            "alt" | "option" => expected_mods |= Modifiers::ALT,
            "shift" => expected_mods |= Modifiers::SHIFT,
            "super" | "cmd" | "meta" => expected_mods |= Modifiers::SUPER,
            _ => key_part = part,
        }
    }

    if key.modifiers != expected_mods {
        return false;
    }

    match key_part {
        "enter" | "return" => key.code == KeyCode::Enter,
        "tab" => key.code == KeyCode::Tab,
        "backtab" => key.code == KeyCode::BackTab,
        "backspace" | "bs" => key.code == KeyCode::Backspace,
        "delete" | "del" => key.code == KeyCode::Delete,
        "escape" | "esc" => key.code == KeyCode::Escape,
        "up" => key.code == KeyCode::Up,
        "down" => key.code == KeyCode::Down,
        "left" => key.code == KeyCode::Left,
        "right" => key.code == KeyCode::Right,
        "home" => key.code == KeyCode::Home,
        "end" => key.code == KeyCode::End,
        "pageup" | "pgup" => key.code == KeyCode::PageUp,
        "pagedown" | "pgdown" | "pgdn" => key.code == KeyCode::PageDown,
        "insert" | "ins" => key.code == KeyCode::Insert,
        "space" | " " => key.code == KeyCode::Char(' '),
        s if s.starts_with('f') && s.len() > 1 => {
            if let Ok(n) = s[1..].parse::<u8>() {
                key.code == KeyCode::F(n)
            } else {
                false
            }
        }
        s if s.len() == 1 => {
            let ch = s.chars().next().expect("length checked above");
            key.code == KeyCode::Char(ch)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_key(code: KeyCode, modifiers: Modifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            is_repeat: false,
        }
    }

    #[test]
    fn test_simple_char_match() {
        let b = Binding::new(&["q"], "q", "quit");
        let key = make_key(KeyCode::Char('q'), Modifiers::empty());
        assert!(b.matches(&key));
    }

    #[test]
    fn test_simple_char_no_match() {
        let b = Binding::new(&["q"], "q", "quit");
        let key = make_key(KeyCode::Char('x'), Modifiers::empty());
        assert!(!b.matches(&key));
    }

    #[test]
    fn test_ctrl_c() {
        let b = Binding::new(&["ctrl+c"], "ctrl+c", "interrupt");
        let key = make_key(KeyCode::Char('c'), Modifiers::CTRL);
        assert!(b.matches(&key));
    }

    #[test]
    fn test_ctrl_c_no_ctrl() {
        let b = Binding::new(&["ctrl+c"], "ctrl+c", "interrupt");
        let key = make_key(KeyCode::Char('c'), Modifiers::empty());
        assert!(!b.matches(&key));
    }

    #[test]
    fn test_enter() {
        let b = Binding::new(&["enter"], "enter", "confirm");
        let key = make_key(KeyCode::Enter, Modifiers::empty());
        assert!(b.matches(&key));
    }

    #[test]
    fn test_up_arrow() {
        let b = Binding::new(&["up"], "up", "move up");
        let key = make_key(KeyCode::Up, Modifiers::empty());
        assert!(b.matches(&key));
    }

    #[test]
    fn test_multiple_keys() {
        let b = Binding::new(&["q", "ctrl+c"], "q/ctrl+c", "quit");
        let key_q = make_key(KeyCode::Char('q'), Modifiers::empty());
        let key_ctrl_c = make_key(KeyCode::Char('c'), Modifiers::CTRL);
        assert!(b.matches(&key_q));
        assert!(b.matches(&key_ctrl_c));
    }

    #[test]
    fn test_disabled_binding() {
        let mut b = Binding::new(&["q"], "q", "quit");
        b.set_enabled(false);
        let key = make_key(KeyCode::Char('q'), Modifiers::empty());
        assert!(!b.matches(&key));
    }

    #[test]
    fn test_key_matches_function() {
        let b1 = Binding::new(&["q"], "q", "quit");
        let b2 = Binding::new(&["ctrl+c"], "ctrl+c", "interrupt");
        let key = make_key(KeyCode::Char('q'), Modifiers::empty());
        assert!(key_matches(&key, &[&b1, &b2]));
    }

    #[test]
    fn test_f_keys() {
        let b = Binding::new(&["f1"], "f1", "help");
        let key = make_key(KeyCode::F(1), Modifiers::empty());
        assert!(b.matches(&key));
    }

    #[test]
    fn test_space() {
        let b = Binding::new(&["space"], "space", "select");
        let key = make_key(KeyCode::Char(' '), Modifiers::empty());
        assert!(b.matches(&key));
    }

    #[test]
    fn test_tab() {
        let b = Binding::new(&["tab"], "tab", "next");
        let key = make_key(KeyCode::Tab, Modifiers::empty());
        assert!(b.matches(&key));
    }

    #[test]
    fn test_shift_tab() {
        let b = Binding::new(&["shift+tab"], "shift+tab", "prev");
        let key = make_key(KeyCode::Tab, Modifiers::SHIFT);
        assert!(b.matches(&key));
    }

    #[test]
    fn test_escape() {
        let b = Binding::new(&["esc"], "esc", "cancel");
        let key = make_key(KeyCode::Escape, Modifiers::empty());
        assert!(b.matches(&key));
    }

    #[test]
    fn test_pageup() {
        let b = Binding::new(&["pgup"], "pgup", "page up");
        let key = make_key(KeyCode::PageUp, Modifiers::empty());
        assert!(b.matches(&key));
    }

    #[test]
    fn test_case_insensitive() {
        let b = Binding::new(&["Ctrl+C"], "ctrl+c", "interrupt");
        let key = make_key(KeyCode::Char('c'), Modifiers::CTRL);
        assert!(b.matches(&key));
    }

    #[test]
    fn test_unbind() {
        let mut b = Binding::new(&["q"], "q", "quit");
        b.unbind();
        let key = make_key(KeyCode::Char('q'), Modifiers::empty());
        assert!(!b.matches(&key));
        assert!(!b.enabled());
        assert!(b.help.key.is_empty());
        assert!(b.help.desc.is_empty());
    }

    #[test]
    fn test_enabled_empty_keys() {
        let b = Binding::new(&[], "", "nothing");
        assert!(!b.enabled());
    }

    #[test]
    fn test_with_help() {
        let b = Binding::new(&["q"], "", "").with_help("q", "quit");
        assert_eq!(b.help_key(), "q");
        assert_eq!(b.help_desc(), "quit");
    }
}
