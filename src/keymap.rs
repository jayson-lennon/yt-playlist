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
    FocusDirectory,
    ToggleItem,
    Rename,
    Notes,
    ReorderUp,
    ReorderDown,
    PlayInMpv,
    MoveToDirectory,
    MoveToPlaylist,
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
    Directory,
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
}

impl Keymap {
    pub fn new() -> Self {
        Self {
            bindings: Self::default_bindings(),
        }
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
                Action::FocusDirectory,
                "focus directory",
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
                KeyCode::Enter,
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
                Action::PlayInMpv,
                "play in mpv",
                KeyCategory::PlaylistActions,
                KeyContext::Playlist,
            ),
            KeyBinding::new(
                KeyCode::Char('L'),
                Action::MoveToDirectory,
                "to directory",
                KeyCategory::PaneSwitch,
                KeyContext::Playlist,
            ),
            KeyBinding::new(
                KeyCode::Char('H'),
                Action::MoveToPlaylist,
                "to playlist",
                KeyCategory::PaneSwitch,
                KeyContext::Directory,
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
                    KeyContext::Directory => pane == Pane::Directory,
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
                KeyContext::Directory => pane == Pane::Directory,
            })
            .collect()
    }
}

impl Default for Keymap {
    fn default() -> Self {
        Self::new()
    }
}
