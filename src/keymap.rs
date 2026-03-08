use crossterm::event::{KeyCode, KeyModifiers};

use crate::ui::Pane;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Quit,
    Save,
    ShowHelp,
    StartFilter,
    MoveUp,
    MoveDown,
    SwitchPane,
    FocusPlaylist,
    FocusLibrary,
    ToggleItem,
    Rename,
    Notes,
    ReorderUp,
    ReorderDown,
    LaunchFile,
    LoadPlaylist,
    MoveToLibrary,
    MoveToPlaylist,
    LaunchMpv,
    AddUrl,
    Delete,
    FuzzyNotes,
}

#[derive(Debug, Clone)]
pub struct FollowupKey {
    pub key: char,
    pub action: Action,
    pub description: &'static str,
}

#[derive(Debug, Clone)]
pub struct PrefixKeyBinding {
    pub prefix: char,
    pub description: &'static str,
    pub followups: Vec<FollowupKey>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCategory {
    Navigation,
    PaneSwitch,
    ItemActions,
    PlaylistActions,
    General,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyContext {
    Global,
    Playlist,
    Library,
}

#[derive(Debug, Clone)]
pub struct KeyBinding {
    pub key: KeyCode,
    pub modifiers: KeyModifiers,
    pub action: Action,
    pub description: &'static str,
    pub category: KeyCategory,
    pub context: KeyContext,
}

impl KeyBinding {
    const fn new(
        key: KeyCode,
        action: Action,
        description: &'static str,
        category: KeyCategory,
        context: KeyContext,
    ) -> Self {
        Self {
            key,
            modifiers: KeyModifiers::empty(),
            action,
            description,
            category,
            context,
        }
    }

    pub fn key_display(&self) -> String {
        match &self.key {
            KeyCode::Char(' ') => "Space".to_string(),
            KeyCode::Char(c) => c.to_string(),
            KeyCode::Tab => "Tab".to_string(),
            KeyCode::Enter => "Enter".to_string(),
            KeyCode::Backspace => "Bksp".to_string(),
            KeyCode::Esc => "Esc".to_string(),
            KeyCode::Up => "Up".to_string(),
            KeyCode::Down => "Down".to_string(),
            KeyCode::Left => "Left".to_string(),
            KeyCode::Right => "Right".to_string(),
            KeyCode::Home => "Home".to_string(),
            KeyCode::End => "End".to_string(),
            KeyCode::PageUp => "PgUp".to_string(),
            KeyCode::PageDown => "PgDn".to_string(),
            _ => "?".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Keymap {
    bindings: Vec<KeyBinding>,
    prefix_bindings: Vec<PrefixKeyBinding>,
}

impl Keymap {
    pub fn new() -> Self {
        Self {
            bindings: Self::default_bindings(),
            prefix_bindings: Self::default_prefix_bindings(),
        }
    }

    fn default_prefix_bindings() -> Vec<PrefixKeyBinding> {
        vec![
            PrefixKeyBinding {
                prefix: 'g',
                description: "general",
                followups: vec![
                    FollowupKey {
                        key: 'm',
                        action: Action::LaunchMpv,
                        description: "launch mpv",
                    },
                    FollowupKey {
                        key: 'f',
                        action: Action::FuzzyNotes,
                        description: "fuzzy search notes",
                    },
                ],
            },
            PrefixKeyBinding {
                prefix: 'a',
                description: "add",
                followups: vec![FollowupKey {
                    key: 'u',
                    action: Action::AddUrl,
                    description: "add url",
                }],
            },
        ]
    }

    pub fn get_prefix_binding(&self, prefix: char) -> Option<&PrefixKeyBinding> {
        self.prefix_bindings.iter().find(|p| p.prefix == prefix)
    }

    pub fn get_prefix_bindings(&self) -> &[PrefixKeyBinding] {
        &self.prefix_bindings
    }

    #[allow(clippy::too_many_lines)]
    fn default_bindings() -> Vec<KeyBinding> {
        vec![
            KeyBinding::new(
                KeyCode::Char('?'),
                Action::ShowHelp,
                "show help",
                KeyCategory::General,
                KeyContext::Global,
            ),
            KeyBinding::new(
                KeyCode::Char('/'),
                Action::StartFilter,
                "filter",
                KeyCategory::General,
                KeyContext::Global,
            ),
            KeyBinding::new(
                KeyCode::Char('q'),
                Action::Quit,
                "quit",
                KeyCategory::General,
                KeyContext::Global,
            ),
            KeyBinding::new(
                KeyCode::Char('s'),
                Action::Save,
                "save",
                KeyCategory::General,
                KeyContext::Global,
            ),
            KeyBinding::new(
                KeyCode::Char('j'),
                Action::MoveDown,
                "down",
                KeyCategory::Navigation,
                KeyContext::Global,
            ),
            KeyBinding::new(
                KeyCode::Char('k'),
                Action::MoveUp,
                "up",
                KeyCategory::Navigation,
                KeyContext::Global,
            ),
            KeyBinding::new(
                KeyCode::Tab,
                Action::SwitchPane,
                "switch pane",
                KeyCategory::PaneSwitch,
                KeyContext::Global,
            ),
            KeyBinding::new(
                KeyCode::Char('h'),
                Action::FocusPlaylist,
                "focus playlist",
                KeyCategory::PaneSwitch,
                KeyContext::Global,
            ),
            KeyBinding::new(
                KeyCode::Char('l'),
                Action::FocusLibrary,
                "focus library",
                KeyCategory::PaneSwitch,
                KeyContext::Global,
            ),
            KeyBinding::new(
                KeyCode::Char(' '),
                Action::ToggleItem,
                "add/remove",
                KeyCategory::ItemActions,
                KeyContext::Global,
            ),
            KeyBinding::new(
                KeyCode::Char('r'),
                Action::Rename,
                "rename",
                KeyCategory::ItemActions,
                KeyContext::Global,
            ),
            KeyBinding::new(
                KeyCode::Char('n'),
                Action::Notes,
                "notes",
                KeyCategory::ItemActions,
                KeyContext::Global,
            ),
            KeyBinding::new(
                KeyCode::Char('J'),
                Action::ReorderDown,
                "move down",
                KeyCategory::PlaylistActions,
                KeyContext::Playlist,
            ),
            KeyBinding::new(
                KeyCode::Char('K'),
                Action::ReorderUp,
                "move up",
                KeyCategory::PlaylistActions,
                KeyContext::Playlist,
            ),
            KeyBinding::new(
                KeyCode::Char('o'),
                Action::LaunchFile,
                "open file",
                KeyCategory::PlaylistActions,
                KeyContext::Playlist,
            ),
            KeyBinding::new(
                KeyCode::Char('p'),
                Action::LoadPlaylist,
                "load playlist",
                KeyCategory::PlaylistActions,
                KeyContext::Playlist,
            ),
            KeyBinding::new(
                KeyCode::Char('L'),
                Action::MoveToLibrary,
                "to library",
                KeyCategory::ItemActions,
                KeyContext::Playlist,
            ),
            KeyBinding::new(
                KeyCode::Char('H'),
                Action::MoveToPlaylist,
                "to playlist",
                KeyCategory::ItemActions,
                KeyContext::Library,
            ),
            KeyBinding::new(
                KeyCode::Char('x'),
                Action::Delete,
                "delete",
                KeyCategory::ItemActions,
                KeyContext::Library,
            ),
        ]
    }

    pub fn get_action(&self, key: KeyCode, modifiers: KeyModifiers, pane: Pane) -> Option<Action> {
        for binding in &self.bindings {
            let key_matches = binding.key == key;
            let modifiers_match =
                matches!(binding.key, KeyCode::Char(_)) || binding.modifiers == modifiers;

            if key_matches && modifiers_match {
                let context_matches = match binding.context {
                    KeyContext::Global => true,
                    KeyContext::Playlist => pane == Pane::Playlist,
                    KeyContext::Library => pane == Pane::Library,
                };
                if context_matches {
                    return Some(binding.action);
                }
            }
        }
        None
    }

    pub fn get_bindings_for_pane(&self, pane: Pane) -> Vec<&KeyBinding> {
        self.bindings
            .iter()
            .filter(|b| match b.context {
                KeyContext::Global => true,
                KeyContext::Playlist => pane == Pane::Playlist,
                KeyContext::Library => pane == Pane::Library,
            })
            .collect()
    }
}

impl Default for Keymap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::Pane;

    #[test]
    fn key_display_shows_char() {
        // Given a key binding with a character.
        let binding = KeyBinding::new(
            KeyCode::Char('a'),
            Action::Quit,
            "quit",
            KeyCategory::General,
            KeyContext::Global,
        );

        // When displaying the key.
        let display = binding.key_display();

        // Then the character is shown.
        assert_eq!(display, "a");
    }

    #[test]
    fn key_display_shows_space() {
        // Given a key binding with space.
        let binding = KeyBinding::new(
            KeyCode::Char(' '),
            Action::ToggleItem,
            "toggle",
            KeyCategory::ItemActions,
            KeyContext::Global,
        );

        // When displaying the key.
        let display = binding.key_display();

        // Then "Space" is shown.
        assert_eq!(display, "Space");
    }

    #[rstest::rstest]
    #[case(KeyCode::Tab, "Tab")]
    #[case(KeyCode::Enter, "Enter")]
    #[case(KeyCode::Backspace, "Bksp")]
    #[case(KeyCode::Esc, "Esc")]
    #[case(KeyCode::Up, "Up")]
    #[case(KeyCode::Down, "Down")]
    #[case(KeyCode::Left, "Left")]
    #[case(KeyCode::Right, "Right")]
    #[case(KeyCode::Home, "Home")]
    #[case(KeyCode::End, "End")]
    #[case(KeyCode::PageUp, "PgUp")]
    #[case(KeyCode::PageDown, "PgDn")]
    fn key_display_special_keys(#[case] key: KeyCode, #[case] expected: &str) {
        let binding = KeyBinding::new(
            key,
            Action::Quit,
            "test",
            KeyCategory::General,
            KeyContext::Global,
        );
        assert_eq!(binding.key_display(), expected);
    }

    #[test]
    fn get_action_returns_action_for_global_context() {
        // Given a keymap.
        let keymap = Keymap::new();

        // When getting action for a global key.
        let action = keymap.get_action(KeyCode::Char('q'), KeyModifiers::empty(), Pane::Playlist);

        // Then the action is returned.
        assert_eq!(action, Some(Action::Quit));
    }

    #[test]
    fn get_action_returns_action_in_library_pane() {
        // Given a keymap.
        let keymap = Keymap::new();

        // When getting action for a global key in library pane.
        let action = keymap.get_action(KeyCode::Char('q'), KeyModifiers::empty(), Pane::Library);

        // Then the action is returned.
        assert_eq!(action, Some(Action::Quit));
    }

    #[test]
    fn get_action_respects_playlist_context() {
        // Given a keymap.
        let keymap = Keymap::new();

        // When getting action for a playlist-only key in playlist pane.
        let action = keymap.get_action(KeyCode::Char('J'), KeyModifiers::empty(), Pane::Playlist);

        // Then the action is returned.
        assert_eq!(action, Some(Action::ReorderDown));
    }

    #[test]
    fn get_action_blocks_playlist_context_in_library() {
        // Given a keymap.
        let keymap = Keymap::new();

        // When getting action for a playlist-only key in library pane.
        let action = keymap.get_action(KeyCode::Char('J'), KeyModifiers::empty(), Pane::Library);

        // Then no action is returned.
        assert!(action.is_none());
    }

    #[test]
    fn get_action_respects_library_context() {
        // Given a keymap.
        let keymap = Keymap::new();

        // When getting action for a library-only key in library pane.
        let action = keymap.get_action(KeyCode::Char('H'), KeyModifiers::empty(), Pane::Library);

        // Then the action is returned.
        assert_eq!(action, Some(Action::MoveToPlaylist));
    }

    #[test]
    fn get_action_blocks_library_context_in_playlist() {
        // Given a keymap.
        let keymap = Keymap::new();

        // When getting action for a library-only key in playlist pane.
        let action = keymap.get_action(KeyCode::Char('H'), KeyModifiers::empty(), Pane::Playlist);

        // Then no action is returned.
        assert!(action.is_none());
    }

    #[test]
    fn get_action_returns_none_for_unbound_key() {
        // Given a keymap.
        let keymap = Keymap::new();

        // When getting action for an unbound key.
        let action = keymap.get_action(KeyCode::Char('x'), KeyModifiers::empty(), Pane::Playlist);

        // Then no action is returned.
        assert!(action.is_none());
    }

    #[test]
    fn get_bindings_for_pane_includes_global_bindings() {
        // Given a keymap.
        let keymap = Keymap::new();

        // When getting bindings for playlist pane.
        let bindings = keymap.get_bindings_for_pane(Pane::Playlist);

        // Then global bindings are included.
        assert!(bindings.iter().any(|b| b.action == Action::Quit));
    }

    #[test]
    fn get_bindings_for_playlist_pane_includes_playlist_bindings() {
        // Given a keymap.
        let keymap = Keymap::new();

        // When getting bindings for playlist pane.
        let bindings = keymap.get_bindings_for_pane(Pane::Playlist);

        // Then playlist-specific bindings are included.
        assert!(bindings.iter().any(|b| b.action == Action::ReorderUp));
    }

    #[test]
    fn get_bindings_for_library_pane_excludes_playlist_bindings() {
        // Given a keymap.
        let keymap = Keymap::new();

        // When getting bindings for library pane.
        let bindings = keymap.get_bindings_for_pane(Pane::Library);

        // Then playlist-specific bindings are excluded.
        assert!(!bindings.iter().any(|b| b.action == Action::ReorderUp));
    }

    #[test]
    fn get_bindings_for_library_pane_includes_library_bindings() {
        // Given a keymap.
        let keymap = Keymap::new();

        // When getting bindings for library pane.
        let bindings = keymap.get_bindings_for_pane(Pane::Library);

        // Then library-specific bindings are included.
        assert!(bindings.iter().any(|b| b.action == Action::MoveToPlaylist));
    }

    #[test]
    fn get_prefix_binding_returns_binding() {
        // Given a keymap.
        let keymap = Keymap::new();

        // When getting prefix binding for 'g'.
        let binding = keymap.get_prefix_binding('g');

        // Then the binding is returned.
        assert!(binding.is_some());
        assert_eq!(binding.unwrap().prefix, 'g');
    }

    #[test]
    fn get_prefix_binding_returns_none_for_unknown() {
        // Given a keymap.
        let keymap = Keymap::new();

        // When getting prefix binding for unknown prefix.
        let binding = keymap.get_prefix_binding('x');

        // Then none is returned.
        assert!(binding.is_none());
    }

    #[test]
    fn get_prefix_bindings_returns_all() {
        // Given a keymap.
        let keymap = Keymap::new();

        // When getting all prefix bindings.
        let bindings = keymap.get_prefix_bindings();

        // Then at least one binding exists.
        assert!(!bindings.is_empty());
    }

    #[test]
    fn default_creates_keymap() {
        // Given a default keymap.
        let keymap = Keymap::default();

        // Then it has bindings.
        let bindings = keymap.get_bindings_for_pane(Pane::Playlist);
        assert!(!bindings.is_empty());
    }
}
