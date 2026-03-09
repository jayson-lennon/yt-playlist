use std::fmt;

use crossterm::event::{KeyCode, KeyModifiers};

use super::{
    parse_key_sequence, Action, Key, KeyCategory, KeyChild, KeyContext, KeyNode, LeafBinding,
    ShowNoteKind,
};
use crate::tui::Pane;

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
                .describe("n", "generate show notes", |n| {
                    n.bind(
                        "h",
                        Action::GenerateShowNotes(ShowNoteKind::Html),
                        "HTML",
                        KeyCategory::General,
                        KeyContext::Global,
                    )
                    .bind(
                        "m",
                        Action::GenerateShowNotes(ShowNoteKind::Markdown),
                        "markdown",
                        KeyCategory::General,
                        KeyContext::Global,
                    );
                });
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
        F: FnOnce(&mut super::GroupBuilder),
    {
        let prefix_keys = parse_key_sequence(prefix);
        if prefix_keys.is_empty() {
            return self;
        }
        self.ensure_branch_with_description(&prefix_keys, description);
        let mut builder = super::GroupBuilder::new(self, prefix_keys);
        bindings(&mut builder);
        self
    }

    pub(super) fn ensure_branch_with_description(
        &mut self,
        keys: &[Key],
        description: &'static str,
    ) {
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

    pub(super) fn insert_into_tree(
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
    /// Returns `FinalizeError` if any key sequences have placeholder (`"..."`) or empty descriptions.
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
                if context_matches {
                    Some(*action)
                } else {
                    None
                }
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

impl Default for Keymap {
    fn default() -> Self {
        Self::new()
    }
}
