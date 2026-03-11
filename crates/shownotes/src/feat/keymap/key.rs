use crossterm::event::KeyCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    Char(char),
    Tab,
    Enter,
    Backspace,
    Esc,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Leader,
}

impl Key {
    pub fn from_keycode(code: KeyCode) -> Option<Self> {
        match code {
            KeyCode::Char(c) => Some(Key::Char(c)),
            KeyCode::Tab => Some(Key::Tab),
            KeyCode::Enter => Some(Key::Enter),
            KeyCode::Backspace => Some(Key::Backspace),
            KeyCode::Esc => Some(Key::Esc),
            KeyCode::Up => Some(Key::Up),
            KeyCode::Down => Some(Key::Down),
            KeyCode::Left => Some(Key::Left),
            KeyCode::Right => Some(Key::Right),
            KeyCode::Home => Some(Key::Home),
            KeyCode::End => Some(Key::End),
            KeyCode::PageUp => Some(Key::PageUp),
            KeyCode::PageDown => Some(Key::PageDown),
            _ => None,
        }
    }

    pub fn display(&self) -> String {
        match self {
            Key::Char(' ') => "Space".to_string(),
            Key::Char(c) => c.to_string(),
            Key::Tab => "Tab".to_string(),
            Key::Enter => "Enter".to_string(),
            Key::Backspace => "Bksp".to_string(),
            Key::Esc => "Esc".to_string(),
            Key::Up => "Up".to_string(),
            Key::Down => "Down".to_string(),
            Key::Left => "Left".to_string(),
            Key::Right => "Right".to_string(),
            Key::Home => "Home".to_string(),
            Key::End => "End".to_string(),
            Key::PageUp => "PgUp".to_string(),
            Key::PageDown => "PgDn".to_string(),
            Key::Leader => "Leader".to_string(),
        }
    }
}

pub fn parse_key_sequence(s: &str) -> Vec<Key> {
    let mut keys = Vec::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '<' {
            let mut tag = String::new();
            while let Some(&next) = chars.peek() {
                if next == '>' {
                    chars.next();
                    break;
                }
                tag.push(next);
                chars.next();
            }
            let key = match tag.as_str() {
                "Tab" => Key::Tab,
                "Enter" => Key::Enter,
                "Bksp" => Key::Backspace,
                "Esc" => Key::Esc,
                "Space" => Key::Char(' '),
                "Up" => Key::Up,
                "Down" => Key::Down,
                "Left" => Key::Left,
                "Right" => Key::Right,
                "Home" => Key::Home,
                "End" => Key::End,
                "PgUp" => Key::PageUp,
                "PgDn" => Key::PageDown,
                "leader" => Key::Leader,
                _ => Key::Char('<'),
            };
            keys.push(key);
        } else {
            keys.push(Key::Char(c));
        }
    }

    keys
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_display_shows_char() {
        let key = Key::Char('a');
        assert_eq!(key.display(), "a");
    }

    #[test]
    fn key_display_shows_space() {
        let key = Key::Char(' ');
        assert_eq!(key.display(), "Space");
    }

    #[rstest::rstest]
    #[case(Key::Tab, "Tab")]
    #[case(Key::Enter, "Enter")]
    #[case(Key::Backspace, "Bksp")]
    #[case(Key::Esc, "Esc")]
    #[case(Key::Up, "Up")]
    #[case(Key::Down, "Down")]
    #[case(Key::Left, "Left")]
    #[case(Key::Right, "Right")]
    #[case(Key::Home, "Home")]
    #[case(Key::End, "End")]
    #[case(Key::PageUp, "PgUp")]
    #[case(Key::PageDown, "PgDn")]
    fn key_display_special_keys(#[case] key: Key, #[case] expected: &str) {
        assert_eq!(key.display(), expected);
    }

    #[test]
    fn parse_simple_chars() {
        let keys = parse_key_sequence("abc");
        assert_eq!(keys, vec![Key::Char('a'), Key::Char('b'), Key::Char('c')]);
    }

    #[test]
    fn parse_special_keys() {
        let keys = parse_key_sequence("<Tab><Enter>");
        assert_eq!(keys, vec![Key::Tab, Key::Enter]);
    }

    #[test]
    fn parse_mixed() {
        let keys = parse_key_sequence("g<Space>m");
        assert_eq!(keys, vec![Key::Char('g'), Key::Char(' '), Key::Char('m')]);
    }

    #[test]
    fn parse_leader_key() {
        // Given a key sequence with leader.
        let keys = parse_key_sequence("<leader>ua");

        // Then leader is parsed as Key::Leader.
        assert_eq!(keys, vec![Key::Leader, Key::Char('u'), Key::Char('a')]);
    }

    #[test]
    fn leader_display() {
        // Given a leader key.
        let key = Key::Leader;

        // Then display shows "Leader".
        assert_eq!(key.display(), "Leader");
    }
}
