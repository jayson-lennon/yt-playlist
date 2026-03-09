use std::fmt;

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
    EditSources,
    GenerateShowNotes,
}

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
        }
    }
}

fn parse_key_sequence(s: &str) -> Vec<Key> {
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
                tag.push(chars.next().unwrap());
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
                _ => Key::Char('<'),
            };
            keys.push(key);
        } else {
            keys.push(Key::Char(c));
        }
    }

    keys
}

#[derive(Debug, Clone)]
pub enum KeyNode {
    Leaf {
        action: Action,
        description: &'static str,
        category: KeyCategory,
        context: KeyContext,
    },
    Branch {
        description: &'static str,
        children: Vec<KeyChild>,
    },
}

impl KeyNode {
    pub fn description(&self) -> &str {
        match self {
            KeyNode::Leaf { description, .. } | KeyNode::Branch { description, .. } => description,
        }
    }

    pub fn is_branch(&self) -> bool {
        matches!(self, KeyNode::Branch { .. })
    }

    pub fn category(&self) -> Option<KeyCategory> {
        match self {
            KeyNode::Leaf { category, .. } => Some(*category),
            KeyNode::Branch { .. } => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct KeyChild {
    pub key: Key,
    pub node: KeyNode,
}

impl KeyChild {
    pub fn new(key: Key, node: KeyNode) -> Self {
        Self { key, node }
    }

    pub fn leaf(
        key: Key,
        action: Action,
        description: &'static str,
        category: KeyCategory,
        context: KeyContext,
    ) -> Self {
        Self {
            key,
            node: KeyNode::Leaf {
                action,
                description,
                category,
                context,
            },
        }
    }

    pub fn branch(key: Key, description: &'static str, children: Vec<KeyChild>) -> Self {
        Self {
            key,
            node: KeyNode::Branch {
                description,
                children,
            },
        }
    }
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
pub struct MissingDescription {
    pub path: Vec<Key>,
}

impl fmt::Display for MissingDescription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let path_str: String = self.path.iter().map(Key::display).collect();
        write!(f, "Key sequence '{path_str}' is missing a description")
    }
}

#[derive(Debug, Clone)]
pub struct FinalizeError {
    pub missing_descriptions: Vec<MissingDescription>,
}

impl fmt::Display for FinalizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Keymap finalization failed. Missing descriptions:")?;
        for missing in &self.missing_descriptions {
            writeln!(f, "  - {missing}")?;
        }
        Ok(())
    }
}

impl std::error::Error for FinalizeError {}

pub struct GroupBuilder<'a> {
    keymap: &'a mut Keymap,
    prefix: Vec<Key>,
}

impl<'a> GroupBuilder<'a> {
    fn new(keymap: &'a mut Keymap, prefix: Vec<Key>) -> Self {
        Self { keymap, prefix }
    }

    pub fn bind(
        &mut self,
        sequence: &str,
        action: Action,
        description: &'static str,
        category: KeyCategory,
        context: KeyContext,
    ) -> &mut Self {
        let keys = parse_key_sequence(sequence);
        if keys.is_empty() {
            return self;
        }

        let full_sequence: Vec<Key> = self.prefix.iter().copied().chain(keys).collect();
        self.keymap
            .insert_into_tree(&full_sequence, action, description, category, context);
        self
    }

    pub fn describe<F>(&mut self, prefix: &str, description: &'static str, bindings: F) -> &mut Self
    where
        F: FnOnce(&mut GroupBuilder),
    {
        let keys = parse_key_sequence(prefix);
        if keys.is_empty() {
            return self;
        }

        let full_prefix: Vec<Key> = self.prefix.iter().copied().chain(keys).collect();
        self.keymap
            .ensure_branch_with_description(&full_prefix, description);

        let mut builder = GroupBuilder::new(self.keymap, full_prefix);
        bindings(&mut builder);
        self
    }
}

#[derive(Debug, Clone)]
pub struct Keymap {
    bindings: Vec<KeyChild>,
}

impl Keymap {
    #[allow(clippy::too_many_lines, clippy::missing_panics_doc)]
    pub fn new() -> Self {
        let mut keymap = Self {
            bindings: Vec::new(),
        };

        keymap
            .bind(
                "?",
                Action::ShowHelp,
                "show help",
                KeyCategory::General,
                KeyContext::Global,
            )
            .bind(
                "/",
                Action::StartFilter,
                "filter",
                KeyCategory::General,
                KeyContext::Global,
            )
            .bind(
                "q",
                Action::Quit,
                "quit",
                KeyCategory::General,
                KeyContext::Global,
            )
            .bind(
                "s",
                Action::Save,
                "save",
                KeyCategory::General,
                KeyContext::Global,
            )
            .bind(
                "j",
                Action::MoveDown,
                "down",
                KeyCategory::Navigation,
                KeyContext::Global,
            )
            .bind(
                "k",
                Action::MoveUp,
                "up",
                KeyCategory::Navigation,
                KeyContext::Global,
            )
            .bind(
                "<Tab>",
                Action::SwitchPane,
                "switch pane",
                KeyCategory::PaneSwitch,
                KeyContext::Global,
            )
            .bind(
                "h",
                Action::FocusPlaylist,
                "focus playlist",
                KeyCategory::PaneSwitch,
                KeyContext::Global,
            )
            .bind(
                "l",
                Action::FocusLibrary,
                "focus library",
                KeyCategory::PaneSwitch,
                KeyContext::Global,
            )
            .bind(
                "<Space>",
                Action::ToggleItem,
                "add/remove",
                KeyCategory::ItemActions,
                KeyContext::Global,
            )
            .bind(
                "r",
                Action::Rename,
                "rename",
                KeyCategory::ItemActions,
                KeyContext::Global,
            )
            .bind(
                "e",
                Action::EditSources,
                "edit sources",
                KeyCategory::ItemActions,
                KeyContext::Global,
            )
            .bind(
                "n",
                Action::Notes,
                "notes",
                KeyCategory::ItemActions,
                KeyContext::Global,
            )
            .bind(
                "J",
                Action::ReorderDown,
                "move down",
                KeyCategory::PlaylistActions,
                KeyContext::Playlist,
            )
            .bind(
                "K",
                Action::ReorderUp,
                "move up",
                KeyCategory::PlaylistActions,
                KeyContext::Playlist,
            )
            .bind(
                "o",
                Action::LaunchFile,
                "open file",
                KeyCategory::PlaylistActions,
                KeyContext::Playlist,
            )
            .bind(
                "p",
                Action::LoadPlaylist,
                "load playlist",
                KeyCategory::PlaylistActions,
                KeyContext::Playlist,
            )
            .bind(
                "L",
                Action::MoveToLibrary,
                "to library",
                KeyCategory::ItemActions,
                KeyContext::Playlist,
            )
            .bind(
                "H",
                Action::MoveToPlaylist,
                "to playlist",
                KeyCategory::ItemActions,
                KeyContext::Library,
            )
            .bind(
                "x",
                Action::Delete,
                "delete",
                KeyCategory::ItemActions,
                KeyContext::Library,
            )
            .describe("g", "general", |g| {
                g.bind(
                    "m",
                    Action::LaunchMpv,
                    "launch mpv",
                    KeyCategory::General,
                    KeyContext::Global,
                )
                .bind(
                    "f",
                    Action::FuzzyNotes,
                    "fuzzy search notes",
                    KeyCategory::General,
                    KeyContext::Global,
                )
                .bind(
                    "n",
                    Action::GenerateShowNotes,
                    "generate show notes",
                    KeyCategory::General,
                    KeyContext::Global,
                );
            })
            .describe("a", "add", |a| {
                a.bind(
                    "u",
                    Action::AddUrl,
                    "add url",
                    KeyCategory::General,
                    KeyContext::Global,
                );
            });

        keymap.finalize().expect("keymap has missing descriptions");
        keymap
    }

    pub fn empty() -> Self {
        Self {
            bindings: Vec::new(),
        }
    }

    pub fn describe<F>(&mut self, prefix: &str, description: &'static str, bindings: F) -> &mut Self
    where
        F: FnOnce(&mut GroupBuilder),
    {
        let prefix_keys = parse_key_sequence(prefix);
        if prefix_keys.is_empty() {
            return self;
        }

        self.ensure_branch_with_description(&prefix_keys, description);

        let mut builder = GroupBuilder::new(self, prefix_keys);
        bindings(&mut builder);
        self
    }

    fn ensure_branch_with_description(&mut self, keys: &[Key], description: &'static str) {
        if keys.is_empty() {
            return;
        }

        let first_key = keys[0];

        if let Some(child) = self.bindings.iter_mut().find(|c| c.key == first_key) {
            Self::ensure_branch_in_child(child, keys, description);
        } else {
            let new_child = Self::build_branch_tree(keys, description);
            self.bindings.push(new_child);
        }
    }

    fn build_branch_tree(keys: &[Key], description: &'static str) -> KeyChild {
        if keys.len() == 1 {
            KeyChild::branch(keys[0], description, Vec::new())
        } else {
            let first = keys[0];
            let rest = &keys[1..];
            let child = Self::build_branch_tree(rest, description);
            KeyChild::branch(first, description, vec![child])
        }
    }

    fn ensure_branch_in_child(child: &mut KeyChild, keys: &[Key], description: &'static str) {
        if keys.len() == 1 {
            match &mut child.node {
                KeyNode::Branch {
                    description: desc, ..
                } => {
                    if *desc == "..." {
                        *desc = description;
                    }
                }
                KeyNode::Leaf { .. } => {
                    child.node = KeyNode::Branch {
                        description,
                        children: Vec::new(),
                    };
                }
            }
            return;
        }

        let next_key = keys[1];
        let rest = &keys[2..];

        match &mut child.node {
            KeyNode::Leaf { .. } => {
                let new_child = if rest.is_empty() {
                    KeyChild::branch(next_key, description, Vec::new())
                } else {
                    Self::build_branch_tree(&keys[1..], description)
                };
                child.node = KeyNode::Branch {
                    description,
                    children: vec![new_child],
                };
            }
            KeyNode::Branch {
                description: desc,
                children,
            } => {
                if *desc == "..." {
                    *desc = description;
                }
                if let Some(next_child) = children.iter_mut().find(|c| c.key == next_key) {
                    Self::ensure_branch_in_child(next_child, &keys[1..], description);
                } else {
                    let new_child = if rest.is_empty() {
                        KeyChild::branch(next_key, description, Vec::new())
                    } else {
                        Self::build_branch_tree(&keys[1..], description)
                    };
                    children.push(new_child);
                }
            }
        }
    }

    pub fn bind(
        &mut self,
        sequence: &str,
        action: Action,
        description: &'static str,
        category: KeyCategory,
        context: KeyContext,
    ) -> &mut Self {
        let keys = parse_key_sequence(sequence);
        if keys.is_empty() {
            return self;
        }
        self.insert_into_tree(&keys, action, description, category, context);
        self
    }

    fn insert_into_tree(
        &mut self,
        keys: &[Key],
        action: Action,
        description: &'static str,
        category: KeyCategory,
        context: KeyContext,
    ) {
        if keys.is_empty() {
            return;
        }

        let first_key = keys[0];

        if let Some(child) = self.bindings.iter_mut().find(|c| c.key == first_key) {
            Self::insert_into_child(child, keys, action, description, category, context);
        } else {
            let new_child = Self::build_tree(keys, action, description, category, context);
            self.bindings.push(new_child);
        }
    }

    fn build_tree(
        keys: &[Key],
        action: Action,
        description: &'static str,
        category: KeyCategory,
        context: KeyContext,
    ) -> KeyChild {
        if keys.len() == 1 {
            KeyChild::leaf(keys[0], action, description, category, context)
        } else {
            let first = keys[0];
            let rest = &keys[1..];
            let child = Self::build_tree(rest, action, description, category, context);
            KeyChild::branch(first, "...", vec![child])
        }
    }

    fn insert_into_child(
        child: &mut KeyChild,
        keys: &[Key],
        action: Action,
        description: &'static str,
        category: KeyCategory,
        context: KeyContext,
    ) {
        if keys.len() == 1 {
            child.node = KeyNode::Leaf {
                action,
                description,
                category,
                context,
            };
            return;
        }

        let next_key = keys[1];

        match &mut child.node {
            KeyNode::Leaf { .. } => {
                let new_child =
                    Self::build_tree(&keys[1..], action, description, category, context);
                child.node = KeyNode::Branch {
                    description: "...",
                    children: vec![new_child],
                };
            }
            KeyNode::Branch { children, .. } => {
                if let Some(next_child) = children.iter_mut().find(|c| c.key == next_key) {
                    Self::insert_into_child(
                        next_child,
                        &keys[1..],
                        action,
                        description,
                        category,
                        context,
                    );
                } else {
                    let new_child =
                        Self::build_tree(&keys[1..], action, description, category, context);
                    children.push(new_child);
                }
            }
        }
    }

    /// Validates the keymap and returns an error if any branches are missing descriptions.
    ///
    /// # Errors
    ///
    /// Returns `FinalizeError` if any key sequences have placeholder or empty descriptions.
    pub fn finalize(&self) -> Result<(), FinalizeError> {
        let mut missing_descriptions = Vec::new();
        Self::collect_missing_descriptions(&self.bindings, Vec::new(), &mut missing_descriptions);

        if missing_descriptions.is_empty() {
            Ok(())
        } else {
            Err(FinalizeError {
                missing_descriptions,
            })
        }
    }

    fn collect_missing_descriptions(
        children: &[KeyChild],
        path: Vec<Key>,
        missing: &mut Vec<MissingDescription>,
    ) {
        for child in children {
            let mut child_path = path.clone();
            child_path.push(child.key);

            let description = child.node.description();
            if description == "..." || description.is_empty() {
                missing.push(MissingDescription {
                    path: child_path.clone(),
                });
            }

            if let KeyNode::Branch { children, .. } = &child.node {
                Self::collect_missing_descriptions(children, child_path, missing);
            }
        }
    }

    pub fn get_node_at_path(&self, keys: &[Key]) -> Option<&KeyNode> {
        if keys.is_empty() {
            return None;
        }

        let first_child = self.bindings.iter().find(|c| c.key == keys[0])?;
        Self::traverse_to_node(first_child, &keys[1..])
    }

    fn traverse_to_node<'a>(child: &'a KeyChild, remaining_keys: &[Key]) -> Option<&'a KeyNode> {
        if remaining_keys.is_empty() {
            return Some(&child.node);
        }

        match &child.node {
            KeyNode::Leaf { .. } => None,
            KeyNode::Branch { children, .. } => {
                let next_child = children.iter().find(|c| c.key == remaining_keys[0])?;
                Self::traverse_to_node(next_child, &remaining_keys[1..])
            }
        }
    }

    pub fn get_children_at_path(&self, keys: &[Key]) -> Option<&[KeyChild]> {
        let node = self.get_node_at_path(keys)?;
        match node {
            KeyNode::Branch { children, .. } => Some(children),
            KeyNode::Leaf { .. } => None,
        }
    }

    pub fn is_prefix_key(&self, key: Key) -> bool {
        self.bindings
            .iter()
            .any(|c| c.key == key && c.node.is_branch())
    }

    pub fn get_bindings(&self) -> &[KeyChild] {
        &self.bindings
    }

    pub fn get_action(&self, key: KeyCode, _modifiers: KeyModifiers, pane: Pane) -> Option<Action> {
        let key = Key::from_keycode(key)?;

        let child = self.bindings.iter().find(|c| c.key == key)?;

        match &child.node {
            KeyNode::Leaf {
                action, context, ..
            } => {
                let context_matches = match context {
                    KeyContext::Global => true,
                    KeyContext::Playlist => pane == Pane::Playlist,
                    KeyContext::Library => pane == Pane::Library,
                };
                if context_matches { Some(*action) } else { None }
            }
            KeyNode::Branch { .. } => None,
        }
    }

    pub fn get_bindings_for_pane(&self, pane: Pane) -> Vec<LeafBinding> {
        self.bindings
            .iter()
            .filter_map(|child| {
                if let KeyNode::Leaf {
                    action,
                    description,
                    category,
                    context,
                } = &child.node
                {
                    let context_matches = match context {
                        KeyContext::Global => true,
                        KeyContext::Playlist => pane == Pane::Playlist,
                        KeyContext::Library => pane == Pane::Library,
                    };
                    if context_matches {
                        Some(LeafBinding {
                            key: child.key,
                            action: *action,
                            description,
                            category: *category,
                            context: *context,
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct LeafBinding {
    pub key: Key,
    pub action: Action,
    pub description: &'static str,
    pub category: KeyCategory,
    pub context: KeyContext,
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
        // Given a character key
        let key = Key::Char('a');

        // When displaying the key
        let display = key.display();

        // Then it shows the character
        assert_eq!(display, "a");
    }

    #[test]
    fn key_display_shows_space() {
        // Given a space key
        let key = Key::Char(' ');

        // When displaying the key
        let display = key.display();

        // Then it shows "Space"
        assert_eq!(display, "Space");
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
        // Given a special key
        // When displaying the key
        // Then it shows the expected string
        assert_eq!(key.display(), expected);
    }

    #[test]
    fn parse_simple_chars() {
        // Given a string of simple characters
        let input = "abc";

        // When parsing the key sequence
        let keys = parse_key_sequence(input);

        // Then each character becomes a key
        assert_eq!(keys, vec![Key::Char('a'), Key::Char('b'), Key::Char('c')]);
    }

    #[test]
    fn parse_special_keys() {
        // Given a string with special key tags
        let input = "<Tab><Enter>";

        // When parsing the key sequence
        let keys = parse_key_sequence(input);

        // Then special keys are recognized
        assert_eq!(keys, vec![Key::Tab, Key::Enter]);
    }

    #[test]
    fn parse_mixed() {
        // Given a string with mixed characters and special keys
        let input = "g<Space>m";

        // When parsing the key sequence
        let keys = parse_key_sequence(input);

        // Then both are parsed correctly
        assert_eq!(keys, vec![Key::Char('g'), Key::Char(' '), Key::Char('m')]);
    }

    #[rstest::rstest]
    #[case(Pane::Playlist)]
    #[case(Pane::Library)]
    fn get_action_returns_global_action_in_any_pane(#[case] pane: Pane) {
        // Given the default keymap
        let keymap = Keymap::new();

        // When getting a global action
        let action = keymap.get_action(KeyCode::Char('q'), KeyModifiers::empty(), pane);

        // Then the action is returned
        assert_eq!(action, Some(Action::Quit));
    }

    #[test]
    fn get_action_respects_playlist_context() {
        // Given the default keymap
        let keymap = Keymap::new();

        // When getting a playlist-only action in playlist pane
        let action = keymap.get_action(KeyCode::Char('J'), KeyModifiers::empty(), Pane::Playlist);

        // Then the action is returned
        assert_eq!(action, Some(Action::ReorderDown));
    }

    #[test]
    fn get_action_blocks_playlist_context_in_library() {
        // Given the default keymap
        let keymap = Keymap::new();

        // When getting a playlist-only action in library pane
        let action = keymap.get_action(KeyCode::Char('J'), KeyModifiers::empty(), Pane::Library);

        // Then no action is returned
        assert!(action.is_none());
    }

    #[test]
    fn get_action_respects_library_context() {
        // Given the default keymap
        let keymap = Keymap::new();

        // When getting a library-only action in library pane
        let action = keymap.get_action(KeyCode::Char('H'), KeyModifiers::empty(), Pane::Library);

        // Then the action is returned
        assert_eq!(action, Some(Action::MoveToPlaylist));
    }

    #[test]
    fn get_action_blocks_library_context_in_playlist() {
        // Given the default keymap
        let keymap = Keymap::new();

        // When getting a library-only action in playlist pane
        let action = keymap.get_action(KeyCode::Char('H'), KeyModifiers::empty(), Pane::Playlist);

        // Then no action is returned
        assert!(action.is_none());
    }

    #[test]
    fn get_action_returns_none_for_unbound_key() {
        // Given the default keymap
        let keymap = Keymap::new();

        // When getting an unbound key
        let action = keymap.get_action(KeyCode::Char('z'), KeyModifiers::empty(), Pane::Playlist);

        // Then no action is returned
        assert!(action.is_none());
    }

    #[test]
    fn get_bindings_for_pane_includes_global_bindings() {
        // Given the default keymap
        let keymap = Keymap::new();

        // When getting bindings for playlist pane
        let bindings = keymap.get_bindings_for_pane(Pane::Playlist);

        // Then global bindings are included
        assert!(bindings.iter().any(|b| b.action == Action::Quit));
    }

    #[test]
    fn get_bindings_for_playlist_pane_includes_playlist_bindings() {
        // Given the default keymap
        let keymap = Keymap::new();

        // When getting bindings for playlist pane
        let bindings = keymap.get_bindings_for_pane(Pane::Playlist);

        // Then playlist bindings are included
        assert!(bindings.iter().any(|b| b.action == Action::ReorderUp));
    }

    #[test]
    fn get_bindings_for_library_pane_excludes_playlist_bindings() {
        // Given the default keymap
        let keymap = Keymap::new();

        let bindings = keymap.get_bindings_for_pane(Pane::Library);

        assert!(!bindings.iter().any(|b| b.action == Action::ReorderUp));
    }

    #[test]
    fn get_bindings_for_library_pane_includes_library_bindings() {
        // Given the default keymap
        let keymap = Keymap::new();

        // When getting bindings for library pane
        let bindings = keymap.get_bindings_for_pane(Pane::Library);

        // Then library bindings are included
        assert!(bindings.iter().any(|b| b.action == Action::MoveToPlaylist));
    }

    #[test]
    fn default_creates_keymap() {
        // When creating a default keymap
        let keymap = Keymap::default();

        // Then it has bindings
        let bindings = keymap.get_bindings_for_pane(Pane::Playlist);
        assert!(!bindings.is_empty());
    }

    #[test]
    fn bind_creates_leaf_node_at_path() {
        // Given an empty keymap
        let mut keymap = Keymap::empty();

        // When binding a single key
        keymap.bind(
            "x",
            Action::Quit,
            "quit",
            KeyCategory::General,
            KeyContext::Global,
        );

        // Then a node exists at that path
        let node = keymap.get_node_at_path(&[Key::Char('x')]);
        assert!(node.is_some());
    }

    #[test]
    fn bind_leaf_has_correct_action() {
        // Given a keymap with a binding
        let mut keymap = Keymap::empty();
        keymap.bind(
            "x",
            Action::Quit,
            "quit",
            KeyCategory::General,
            KeyContext::Global,
        );

        // When getting the node
        let node = keymap.get_node_at_path(&[Key::Char('x')]).unwrap();

        // Then it has the correct action
        assert!(matches!(
            node,
            KeyNode::Leaf {
                action: Action::Quit,
                ..
            }
        ));
    }

    #[test]
    fn bind_leaf_has_correct_description() {
        // Given a keymap with a binding
        let mut keymap = Keymap::empty();
        keymap.bind(
            "x",
            Action::Quit,
            "quit",
            KeyCategory::General,
            KeyContext::Global,
        );

        // When getting the node
        let node = keymap.get_node_at_path(&[Key::Char('x')]).unwrap();

        // Then it has the correct description
        assert_eq!(node.description(), "quit");
    }

    #[test]
    fn bind_leaf_has_correct_category() {
        // Given a keymap with a binding
        let mut keymap = Keymap::empty();
        keymap.bind(
            "x",
            Action::Quit,
            "quit",
            KeyCategory::General,
            KeyContext::Global,
        );

        // When getting the node
        let node = keymap.get_node_at_path(&[Key::Char('x')]).unwrap();

        // Then it has the correct category
        assert_eq!(node.category(), Some(KeyCategory::General));
    }

    #[rstest::rstest]
    #[case(KeyContext::Global)]
    #[case(KeyContext::Playlist)]
    #[case(KeyContext::Library)]
    fn bind_leaf_has_correct_context(#[case] context: KeyContext) {
        // Given an empty keymap
        let mut keymap = Keymap::empty();

        // When binding with a specific context
        keymap.bind("x", Action::Quit, "quit", KeyCategory::General, context);

        // Then the leaf has that context
        let node = keymap.get_node_at_path(&[Key::Char('x')]).unwrap();
        assert!(matches!(
            node,
            KeyNode::Leaf {
                context: c,
                ..
            } if *c == context
        ));
    }

    #[test]
    fn bind_creates_branch_for_multi_key_sequence() {
        // Given an empty keymap
        let mut keymap = Keymap::empty();

        // When binding a multi-key sequence
        keymap.bind(
            "gm",
            Action::LaunchMpv,
            "launch mpv",
            KeyCategory::General,
            KeyContext::Global,
        );

        // Then the first key is a prefix key
        assert!(keymap.is_prefix_key(Key::Char('g')));

        // And a branch node exists at the first key
        let node = keymap.get_node_at_path(&[Key::Char('g')]).unwrap();
        assert!(node.is_branch());
    }

    #[test]
    fn finalize_fails_with_placeholder_description() {
        // Given a keymap with an undescribed branch
        let mut keymap = Keymap::empty();
        keymap.bind(
            "gm",
            Action::LaunchMpv,
            "launch mpv",
            KeyCategory::General,
            KeyContext::Global,
        );

        // When finalizing
        let result = keymap.finalize();

        // Then it fails with missing description
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.missing_descriptions.len(), 1);
        assert_eq!(err.missing_descriptions[0].path, vec![Key::Char('g')]);
    }

    #[test]
    fn describe_sets_branch_description() {
        // Given an empty keymap
        let mut keymap = Keymap::empty();

        // When describing a prefix with bindings
        keymap.describe("g", "general", |g| {
            g.bind(
                "m",
                Action::LaunchMpv,
                "launch mpv",
                KeyCategory::General,
                KeyContext::Global,
            );
        });

        // Then the branch has the description
        let node = keymap.get_node_at_path(&[Key::Char('g')]).unwrap();
        assert_eq!(node.description(), "general");
    }

    #[test]
    fn finalize_succeeds_when_branch_is_described() {
        // Given a keymap with a described branch
        let mut keymap = Keymap::empty();
        keymap.describe("g", "general", |g| {
            g.bind(
                "m",
                Action::LaunchMpv,
                "launch mpv",
                KeyCategory::General,
                KeyContext::Global,
            );
        });

        // When finalizing
        let result = keymap.finalize();

        // Then it succeeds
        assert!(result.is_ok());
    }

    #[test]
    fn describe_creates_branch_with_description() {
        // Given an empty keymap
        let mut keymap = Keymap::empty();

        // When describing a prefix with multiple bindings
        keymap.describe("g", "general", |g| {
            g.bind(
                "m",
                Action::LaunchMpv,
                "launch mpv",
                KeyCategory::General,
                KeyContext::Global,
            )
            .bind(
                "f",
                Action::FuzzyNotes,
                "fuzzy notes",
                KeyCategory::General,
                KeyContext::Global,
            );
        });

        // Then the branch has the description
        let node = keymap.get_node_at_path(&[Key::Char('g')]).unwrap();
        assert_eq!(node.description(), "general");
    }

    #[rstest::rstest]
    #[case(&[Key::Char('g'), Key::Char('m')], Action::LaunchMpv)]
    #[case(&[Key::Char('g'), Key::Char('f')], Action::FuzzyNotes)]
    fn describe_creates_leaf_children(#[case] path: &[Key], #[case] expected_action: Action) {
        // Given an empty keymap
        let mut keymap = Keymap::empty();

        // When describing a prefix with multiple bindings
        keymap.describe("g", "general", |g| {
            g.bind(
                "m",
                Action::LaunchMpv,
                "launch mpv",
                KeyCategory::General,
                KeyContext::Global,
            )
            .bind(
                "f",
                Action::FuzzyNotes,
                "fuzzy notes",
                KeyCategory::General,
                KeyContext::Global,
            );
        });

        // Then each path has the correct leaf action
        let node = keymap.get_node_at_path(path).unwrap();
        assert!(matches!(
            node,
            KeyNode::Leaf {
                action,
                ..
            } if *action == expected_action
        ));
    }

    #[rstest::rstest]
    #[case(Key::Char('g'), "general")]
    #[case(Key::Char('a'), "add")]
    fn describe_chains_multiple_prefixes(#[case] key: Key, #[case] expected_description: &str) {
        // Given an empty keymap
        let mut keymap = Keymap::empty();

        // When chaining multiple describe calls
        keymap
            .describe("g", "general", |g| {
                g.bind(
                    "m",
                    Action::LaunchMpv,
                    "launch mpv",
                    KeyCategory::General,
                    KeyContext::Global,
                );
            })
            .describe("a", "add", |a| {
                a.bind(
                    "u",
                    Action::AddUrl,
                    "add url",
                    KeyCategory::General,
                    KeyContext::Global,
                );
            });

        // Then each prefix has its description
        let node = keymap.get_node_at_path(&[key]).unwrap();
        assert_eq!(node.description(), expected_description);
    }

    #[test]
    fn finalize_detects_multiple_missing_descriptions() {
        // Given a keymap with multiple undescribed branches
        let mut keymap = Keymap::empty();
        keymap.bind(
            "gm",
            Action::LaunchMpv,
            "launch mpv",
            KeyCategory::General,
            KeyContext::Global,
        );
        keymap.bind(
            "au",
            Action::AddUrl,
            "add url",
            KeyCategory::General,
            KeyContext::Global,
        );

        // When finalizing
        let result = keymap.finalize();

        // Then it fails with all missing descriptions
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.missing_descriptions.len(), 2);
    }

    #[test]
    fn bind_adds_multiple_children_to_branch() {
        // Given an empty keymap
        let mut keymap = Keymap::empty();

        // When binding multiple keys under the same prefix
        keymap.describe("g", "general", |g| {
            g.bind(
                "m",
                Action::LaunchMpv,
                "launch mpv",
                KeyCategory::General,
                KeyContext::Global,
            )
            .bind(
                "f",
                Action::FuzzyNotes,
                "fuzzy notes",
                KeyCategory::General,
                KeyContext::Global,
            );
        });

        // Then the branch has multiple children
        let children = keymap.get_children_at_path(&[Key::Char('g')]).unwrap();
        assert_eq!(children.len(), 2);
    }

    #[test]
    fn bind_supports_nested_describes() {
        // Given an empty keymap
        let mut keymap = Keymap::empty();

        // When nesting describes
        keymap.describe("g", "general", |g| {
            g.describe("m", "mpv", |m| {
                m.bind(
                    "p",
                    Action::LaunchMpv,
                    "mpv play",
                    KeyCategory::General,
                    KeyContext::Global,
                );
            });
        });

        // Then the first key is a prefix
        assert!(keymap.is_prefix_key(Key::Char('g')));
    }

    #[rstest::rstest]
    #[case(&[Key::Char('g')], "general")]
    #[case(&[Key::Char('g'), Key::Char('m')], "mpv")]
    fn bind_nested_describe_has_descriptions(#[case] path: &[Key], #[case] expected: &str) {
        // Given an empty keymap
        let mut keymap = Keymap::empty();

        // When nesting describes
        keymap.describe("g", "general", |g| {
            g.describe("m", "mpv", |m| {
                m.bind(
                    "p",
                    Action::LaunchMpv,
                    "mpv play",
                    KeyCategory::General,
                    KeyContext::Global,
                );
            });
        });

        // Then each level has its description
        let node = keymap.get_node_at_path(path).unwrap();
        assert!(node.is_branch());
        assert_eq!(node.description(), expected);
    }

    #[test]
    fn bind_nested_creates_leaf_at_full_path() {
        // Given an empty keymap
        let mut keymap = Keymap::empty();

        // When nesting describes with a leaf binding
        keymap.describe("g", "general", |g| {
            g.describe("m", "mpv", |m| {
                m.bind(
                    "p",
                    Action::LaunchMpv,
                    "mpv play",
                    KeyCategory::General,
                    KeyContext::Global,
                );
            });
        });

        // Then the leaf exists at the full path
        let node = keymap
            .get_node_at_path(&[Key::Char('g'), Key::Char('m'), Key::Char('p')])
            .unwrap();
        assert!(matches!(
            node,
            KeyNode::Leaf {
                action: Action::LaunchMpv,
                ..
            }
        ));
    }

    #[test]
    fn get_children_at_path_returns_none_for_leaf() {
        // Given a keymap with a leaf binding
        let mut keymap = Keymap::empty();
        keymap.bind(
            "x",
            Action::Quit,
            "quit",
            KeyCategory::General,
            KeyContext::Global,
        );

        // When getting children at a leaf path
        let children = keymap.get_children_at_path(&[Key::Char('x')]);

        // Then no children are returned
        assert!(children.is_none());
    }

    #[test]
    fn get_node_at_path_returns_none_for_empty() {
        // Given an empty keymap
        let keymap = Keymap::empty();

        // When getting node with empty path
        let node = keymap.get_node_at_path(&[]);

        // Then no node is returned
        assert!(node.is_none());
    }

    #[test]
    fn get_node_at_path_returns_none_for_unknown_key() {
        // Given an empty keymap
        let keymap = Keymap::empty();

        // When getting node for unbound key
        let node = keymap.get_node_at_path(&[Key::Char('z')]);

        // Then no node is returned
        assert!(node.is_none());
    }

    #[rstest::rstest]
    #[case(Key::Char('g'), true, "prefix key")]
    #[case(Key::Char('m'), false, "leaf key")]
    #[case(Key::Char('x'), false, "unbound key")]
    fn is_prefix_key_returns_correct_value(
        #[case] key: Key,
        #[case] expected: bool,
        #[case] description: &str,
    ) {
        // Given a keymap with a prefix key 'g'
        let mut keymap = Keymap::empty();
        keymap.describe("g", "general", |g| {
            g.bind(
                "m",
                Action::LaunchMpv,
                "launch mpv",
                KeyCategory::General,
                KeyContext::Global,
            );
        });

        // When checking if key is a prefix
        // Then it returns the expected value
        assert_eq!(
            keymap.is_prefix_key(key),
            expected,
            "failed for {description}"
        );
    }

    #[rstest::rstest]
    #[case(Key::Char('g'), "general prefix")]
    #[case(Key::Char('a'), "add prefix")]
    fn default_keymap_has_prefix_keys(#[case] key: Key, #[case] description: &str) {
        // Given the default keymap
        let keymap = Keymap::new();

        // When checking for prefix keys
        // Then they are recognized as prefixes
        assert!(keymap.is_prefix_key(key), "failed for {description}");
    }

    #[rstest::rstest]
    #[case(&[Key::Char('g'), Key::Char('m')], Action::LaunchMpv)]
    #[case(&[Key::Char('a'), Key::Char('u')], Action::AddUrl)]
    fn default_keymap_has_sequence_leaf_bindings(
        #[case] path: &[Key],
        #[case] expected_action: Action,
    ) {
        // Given the default keymap
        let keymap = Keymap::new();

        // When getting a node at a sequence path
        let node = keymap.get_node_at_path(path).unwrap();

        // Then it has the correct action
        assert!(matches!(
            node,
            KeyNode::Leaf {
                action,
                ..
            } if *action == expected_action
        ));
    }

    #[test]
    fn default_keymap_has_all_descriptions() {
        // Given the default keymap
        let keymap = Keymap::new();

        // When finalizing
        let result = keymap.finalize();

        // Then it succeeds
        assert!(result.is_ok());
    }

    #[test]
    fn bind_with_special_key() {
        // Given an empty keymap
        let mut keymap = Keymap::empty();

        // When binding with a special key
        keymap.bind(
            "<Tab>",
            Action::SwitchPane,
            "switch pane",
            KeyCategory::PaneSwitch,
            KeyContext::Global,
        );

        // Then the node exists with the correct action
        let node = keymap.get_node_at_path(&[Key::Tab]);
        assert!(matches!(
            node,
            Some(KeyNode::Leaf {
                action: Action::SwitchPane,
                ..
            })
        ));
    }

    #[test]
    fn missing_description_display() {
        // Given a missing description with a path
        let missing = MissingDescription {
            path: vec![Key::Char('g'), Key::Char('m')],
        };

        // When displaying the error
        let display = missing.to_string();

        // Then it shows the key sequence
        assert_eq!(display, "Key sequence 'gm' is missing a description");
    }

    #[test]
    fn finalize_error_display() {
        // Given a finalize error with multiple missing descriptions
        let err = FinalizeError {
            missing_descriptions: vec![
                MissingDescription {
                    path: vec![Key::Char('g')],
                },
                MissingDescription {
                    path: vec![Key::Char('a')],
                },
            ],
        };

        // When displaying the error
        let display = err.to_string();

        // Then it contains all missing descriptions
        assert!(display.contains("Key sequence 'g' is missing a description"));
        assert!(display.contains("Key sequence 'a' is missing a description"));
    }
}
