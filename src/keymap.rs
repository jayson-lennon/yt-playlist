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

#[derive(Debug, Clone)]
pub enum KeyNode {
    Leaf {
        action: Action,
        description: &'static str,
    },
    Branch {
        description: &'static str,
        children: Vec<KeyChild>,
    },
}

impl KeyNode {
    pub fn description(&self) -> &str {
        match self {
            KeyNode::Leaf { description, .. } => description,
            KeyNode::Branch { description, .. } => description,
        }
    }

    pub fn is_branch(&self) -> bool {
        matches!(self, KeyNode::Branch { .. })
    }
}

#[derive(Debug, Clone)]
pub struct KeyChild {
    pub key: char,
    pub node: KeyNode,
}

impl KeyChild {
    pub fn new(key: char, node: KeyNode) -> Self {
        Self { key, node }
    }

    pub fn leaf(key: char, action: Action, description: &'static str) -> Self {
        Self {
            key,
            node: KeyNode::Leaf {
                action,
                description,
            },
        }
    }

    pub fn branch(key: char, description: &'static str, children: Vec<KeyChild>) -> Self {
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
    sequence_bindings: Vec<KeyChild>,
}

impl Keymap {
    pub fn new() -> Self {
        Self {
            bindings: Self::default_bindings(),
            sequence_bindings: Self::default_sequence_bindings(),
        }
    }

    fn default_sequence_bindings() -> Vec<KeyChild> {
        let mut keymap = Self::empty();
        keymap
            .add_sequence("gm", Action::LaunchMpv, "launch mpv")
            .add_sequence("gf", Action::FuzzyNotes, "fuzzy search notes")
            .add_sequence("gn", Action::GenerateShowNotes, "generate show notes")
            .add_sequence("au", Action::AddUrl, "add url");
        keymap.sequence_bindings
    }

    pub fn empty() -> Self {
        Self {
            bindings: Vec::new(),
            sequence_bindings: Vec::new(),
        }
    }

    pub fn add_sequence(
        &mut self,
        sequence: &str,
        action: Action,
        description: &'static str,
    ) -> &mut Self {
        let chars: Vec<char> = sequence.chars().collect();
        if chars.is_empty() {
            return self;
        }
        self.insert_into_tree(&chars, action, description);
        self
    }

    fn insert_into_tree(&mut self, keys: &[char], action: Action, description: &'static str) {
        if keys.is_empty() {
            return;
        }

        let first_key = keys[0];

        if let Some(child) = self
            .sequence_bindings
            .iter_mut()
            .find(|c| c.key == first_key)
        {
            Self::insert_into_child(child, keys, action, description);
        } else {
            let new_child = Self::build_tree(keys, action, description);
            self.sequence_bindings.push(new_child);
        }
    }

    fn build_tree(keys: &[char], action: Action, description: &'static str) -> KeyChild {
        if keys.len() == 1 {
            KeyChild::leaf(keys[0], action, description)
        } else {
            let first = keys[0];
            let rest = &keys[1..];
            let child = Self::build_tree(rest, action, description);
            KeyChild::branch(first, "", vec![child])
        }
    }

    fn insert_into_child(
        child: &mut KeyChild,
        keys: &[char],
        action: Action,
        description: &'static str,
    ) {
        if keys.len() == 1 {
            child.node = KeyNode::Leaf {
                action,
                description,
            };
            return;
        }

        let next_key = keys[1];
        let rest = &keys[2..];

        match &mut child.node {
            KeyNode::Leaf { .. } => {
                let new_child = if rest.is_empty() {
                    KeyChild::leaf(next_key, action, description)
                } else {
                    let nested = Self::build_tree(&keys[1..], action, description);
                    nested
                };
                child.node = KeyNode::Branch {
                    description: "",
                    children: vec![new_child],
                };
            }
            KeyNode::Branch { children, .. } => {
                if let Some(next_child) = children.iter_mut().find(|c| c.key == next_key) {
                    Self::insert_into_child(next_child, &keys[1..], action, description);
                } else {
                    let new_child = Self::build_tree(&keys[1..], action, description);
                    children.push(new_child);
                }
            }
        }
    }

    pub fn get_node_at_path(&self, keys: &[char]) -> Option<&KeyNode> {
        if keys.is_empty() {
            return None;
        }

        let first_child = self.sequence_bindings.iter().find(|c| c.key == keys[0])?;
        Self::traverse_to_node(first_child, &keys[1..])
    }

    fn traverse_to_node<'a>(child: &'a KeyChild, remaining_keys: &[char]) -> Option<&'a KeyNode> {
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

    pub fn get_children_at_path(&self, keys: &[char]) -> Option<&[KeyChild]> {
        let node = self.get_node_at_path(keys)?;
        match node {
            KeyNode::Branch { children, .. } => Some(children),
            KeyNode::Leaf { .. } => None,
        }
    }

    pub fn is_prefix_key(&self, key: char) -> bool {
        self.sequence_bindings.iter().any(|c| c.key == key)
    }

    pub fn get_sequence_bindings(&self) -> &[KeyChild] {
        &self.sequence_bindings
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
                KeyCode::Char('e'),
                Action::EditSources,
                "edit sources",
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
        let binding = KeyBinding::new(
            KeyCode::Char('a'),
            Action::Quit,
            "quit",
            KeyCategory::General,
            KeyContext::Global,
        );

        let display = binding.key_display();

        assert_eq!(display, "a");
    }

    #[test]
    fn key_display_shows_space() {
        let binding = KeyBinding::new(
            KeyCode::Char(' '),
            Action::ToggleItem,
            "toggle",
            KeyCategory::ItemActions,
            KeyContext::Global,
        );

        let display = binding.key_display();

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
        let keymap = Keymap::new();

        let action = keymap.get_action(KeyCode::Char('q'), KeyModifiers::empty(), Pane::Playlist);

        assert_eq!(action, Some(Action::Quit));
    }

    #[test]
    fn get_action_returns_action_in_library_pane() {
        let keymap = Keymap::new();

        let action = keymap.get_action(KeyCode::Char('q'), KeyModifiers::empty(), Pane::Library);

        assert_eq!(action, Some(Action::Quit));
    }

    #[test]
    fn get_action_respects_playlist_context() {
        let keymap = Keymap::new();

        let action = keymap.get_action(KeyCode::Char('J'), KeyModifiers::empty(), Pane::Playlist);

        assert_eq!(action, Some(Action::ReorderDown));
    }

    #[test]
    fn get_action_blocks_playlist_context_in_library() {
        let keymap = Keymap::new();

        let action = keymap.get_action(KeyCode::Char('J'), KeyModifiers::empty(), Pane::Library);

        assert!(action.is_none());
    }

    #[test]
    fn get_action_respects_library_context() {
        let keymap = Keymap::new();

        let action = keymap.get_action(KeyCode::Char('H'), KeyModifiers::empty(), Pane::Library);

        assert_eq!(action, Some(Action::MoveToPlaylist));
    }

    #[test]
    fn get_action_blocks_library_context_in_playlist() {
        let keymap = Keymap::new();

        let action = keymap.get_action(KeyCode::Char('H'), KeyModifiers::empty(), Pane::Playlist);

        assert!(action.is_none());
    }

    #[test]
    fn get_action_returns_none_for_unbound_key() {
        let keymap = Keymap::new();

        let action = keymap.get_action(KeyCode::Char('z'), KeyModifiers::empty(), Pane::Playlist);

        assert!(action.is_none());
    }

    #[test]
    fn get_bindings_for_pane_includes_global_bindings() {
        let keymap = Keymap::new();

        let bindings = keymap.get_bindings_for_pane(Pane::Playlist);

        assert!(bindings.iter().any(|b| b.action == Action::Quit));
    }

    #[test]
    fn get_bindings_for_playlist_pane_includes_playlist_bindings() {
        let keymap = Keymap::new();

        let bindings = keymap.get_bindings_for_pane(Pane::Playlist);

        assert!(bindings.iter().any(|b| b.action == Action::ReorderUp));
    }

    #[test]
    fn get_bindings_for_library_pane_excludes_playlist_bindings() {
        let keymap = Keymap::new();

        let bindings = keymap.get_bindings_for_pane(Pane::Library);

        assert!(!bindings.iter().any(|b| b.action == Action::ReorderUp));
    }

    #[test]
    fn get_bindings_for_library_pane_includes_library_bindings() {
        let keymap = Keymap::new();

        let bindings = keymap.get_bindings_for_pane(Pane::Library);

        assert!(bindings.iter().any(|b| b.action == Action::MoveToPlaylist));
    }

    #[test]
    fn default_creates_keymap() {
        let keymap = Keymap::default();

        let bindings = keymap.get_bindings_for_pane(Pane::Playlist);
        assert!(!bindings.is_empty());
    }

    #[test]
    fn add_sequence_creates_leaf() {
        let mut keymap = Keymap::empty();

        keymap.add_sequence("x", Action::Quit, "quit");

        let node = keymap.get_node_at_path(&['x']);
        assert!(node.is_some());
        if let Some(KeyNode::Leaf {
            action,
            description,
        }) = node
        {
            assert_eq!(*action, Action::Quit);
            assert_eq!(*description, "quit");
        } else {
            panic!("Expected Leaf node");
        }
    }

    #[test]
    fn add_sequence_creates_branch() {
        let mut keymap = Keymap::empty();

        keymap.add_sequence("gm", Action::LaunchMpv, "launch mpv");

        assert!(keymap.is_prefix_key('g'));

        let node = keymap.get_node_at_path(&['g']);
        assert!(node.is_some());
        assert!(node.unwrap().is_branch());

        let leaf_node = keymap.get_node_at_path(&['g', 'm']);
        assert!(leaf_node.is_some());
        if let Some(KeyNode::Leaf { action, .. }) = leaf_node {
            assert_eq!(*action, Action::LaunchMpv);
        } else {
            panic!("Expected Leaf node at g>m");
        }
    }

    #[test]
    fn add_sequence_merges_branches() {
        let mut keymap = Keymap::empty();

        keymap
            .add_sequence("gm", Action::LaunchMpv, "launch mpv")
            .add_sequence("gf", Action::FuzzyNotes, "fuzzy notes");

        let children = keymap.get_children_at_path(&['g']);
        assert!(children.is_some());
        let children = children.unwrap();
        assert_eq!(children.len(), 2);

        let node_m = keymap.get_node_at_path(&['g', 'm']);
        assert!(matches!(
            node_m,
            Some(KeyNode::Leaf {
                action: Action::LaunchMpv,
                ..
            })
        ));

        let node_f = keymap.get_node_at_path(&['g', 'f']);
        assert!(matches!(
            node_f,
            Some(KeyNode::Leaf {
                action: Action::FuzzyNotes,
                ..
            })
        ));
    }

    #[test]
    fn add_sequence_depth_3() {
        let mut keymap = Keymap::empty();

        keymap.add_sequence("gmp", Action::LaunchMpv, "mpv play");

        assert!(keymap.is_prefix_key('g'));

        let node_g = keymap.get_node_at_path(&['g']);
        assert!(node_g.unwrap().is_branch());

        let children_g = keymap.get_children_at_path(&['g']).unwrap();
        assert!(children_g.iter().any(|c| c.key == 'm'));

        let node_m = keymap.get_node_at_path(&['g', 'm']);
        assert!(node_m.unwrap().is_branch());

        let node_p = keymap.get_node_at_path(&['g', 'm', 'p']);
        assert!(matches!(
            node_p,
            Some(KeyNode::Leaf {
                action: Action::LaunchMpv,
                ..
            })
        ));
    }

    #[test]
    fn get_children_at_path_returns_none_for_leaf() {
        let mut keymap = Keymap::empty();
        keymap.add_sequence("x", Action::Quit, "quit");

        let children = keymap.get_children_at_path(&['x']);
        assert!(children.is_none());
    }

    #[test]
    fn get_node_at_path_returns_none_for_empty() {
        let keymap = Keymap::empty();

        let node = keymap.get_node_at_path(&[]);
        assert!(node.is_none());
    }

    #[test]
    fn get_node_at_path_returns_none_for_unknown_key() {
        let keymap = Keymap::empty();

        let node = keymap.get_node_at_path(&['z']);
        assert!(node.is_none());
    }

    #[test]
    fn is_prefix_key_returns_true_for_sequence_start() {
        let mut keymap = Keymap::empty();
        keymap.add_sequence("gm", Action::LaunchMpv, "launch mpv");

        assert!(keymap.is_prefix_key('g'));
        assert!(!keymap.is_prefix_key('m'));
        assert!(!keymap.is_prefix_key('x'));
    }

    #[test]
    fn default_keymap_has_sequences() {
        let keymap = Keymap::new();

        assert!(keymap.is_prefix_key('g'));
        assert!(keymap.is_prefix_key('a'));

        let node_gm = keymap.get_node_at_path(&['g', 'm']);
        assert!(matches!(
            node_gm,
            Some(KeyNode::Leaf {
                action: Action::LaunchMpv,
                ..
            })
        ));

        let node_au = keymap.get_node_at_path(&['a', 'u']);
        assert!(matches!(
            node_au,
            Some(KeyNode::Leaf {
                action: Action::AddUrl,
                ..
            })
        ));
    }
}
