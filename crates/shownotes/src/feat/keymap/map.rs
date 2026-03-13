use std::fmt;

use crossterm::event::{KeyCode, KeyModifiers};

use super::{
    Action, Key, KeyCategory, KeyChild, KeyContext, KeyNode, LeafBinding, ShowNoteKind,
    parse_key_sequence,
};
use crate::tui::Pane;

/// Error indicating a key sequence is missing a description.
///
/// Created during keymap finalization when a branch node (prefix key)
/// doesn't have a description set, which is required for the which-key
/// help display.
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

/// Error from keymap finalization with missing descriptions.
///
/// Contains a list of all key sequences that are missing descriptions,
/// collected during the finalization validation step.
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

/// Hierarchical keybinding map with context-aware dispatch.
///
/// Stores keybindings in a tree structure to support multi-key sequences
/// (like "gm" for launching mpv). Supports context-aware bindings that
/// only apply in specific panes, and provides the which-key help display
/// with binding descriptions.
#[derive(Debug, Clone)]
pub struct Keymap {
    bindings: Vec<KeyChild>,
    leader_key: Key,
}

impl Keymap {
    #[rustfmt::skip]
    #[allow(clippy::too_many_lines, clippy::missing_panics_doc)]
    pub fn new() -> Self {
        use super::KeyContext as Ctx;
        use super::KeyCategory as Cat;

        let mut keymap = Self {
            bindings: Vec::new(),
            leader_key: Key::Char(' '),
        };

        keymap
            .bind("?", Action::ShowHelp, "show help", Cat::General, Ctx::Global)
            .bind("/", Action::StartFilter, "filter", Cat::General, Ctx::Global)
            .bind("q", Action::Quit, "quit", Cat::General, Ctx::Global)
            .bind("j", Action::MoveDown, "down", Cat::Navigation, Ctx::Global)
            .bind("k", Action::MoveUp, "up", Cat::Navigation, Ctx::Global)
            .bind("h", Action::FocusPlaylist, "focus playlist", Cat::PaneSwitch, Ctx::Global)
            .bind("l", Action::FocusLibrary, "focus library", Cat::PaneSwitch, Ctx::Global)
            .bind("r", Action::Rename, "rename", Cat::ItemActions, Ctx::Global)
            .bind("J", Action::ReorderDown, "move down", Cat::PlaylistActions, Ctx::Playlist)
            .bind("K", Action::ReorderUp, "move up", Cat::PlaylistActions, Ctx::Playlist)
            .bind("o", Action::LaunchFile, "open file", Cat::ItemActions, Ctx::Global)
            .bind("L", Action::MoveToLibrary, "to library", Cat::ItemActions, Ctx::Playlist)
            .bind("H", Action::MoveToPlaylist, "to playlist", Cat::ItemActions, Ctx::Library)
            .bind("x", Action::Delete, "delete", Cat::ItemActions, Ctx::Library)
            .describe_prefix("e", "edit")
            .bind("en", Action::Notes, "notes", Cat::ItemActions, Ctx::Global)
            .bind("es", Action::EditSources, "edit sources", Cat::ItemActions, Ctx::Global)
            .describe_prefix("g", "general")
            .bind("gm", Action::LaunchMpv, "launch mpv", Cat::General, Ctx::Global)
            .bind("gp", Action::LoadPlaylist, "playlist to mpv", Cat::PlaylistActions, Ctx::Playlist)
            .describe_prefix("gn", "generate show notes")
            .bind("gnh", Action::GenerateShowNotes(ShowNoteKind::Html), "HTML", Cat::General, Ctx::Global)
            .bind("gnm", Action::GenerateShowNotes(ShowNoteKind::Markdown), "markdown", Cat::General, Ctx::Global)
            .describe_prefix("a", "add")
            .bind("au", Action::AddUrl, "add url", Cat::General, Ctx::Global)
            .describe_prefix("<leader>", "<leader>")
            .describe_prefix("<leader>u", "ui")
            .bind("<leader>ua", Action::ShowAlias, "show alias", Cat::General, Ctx::Global)
            .bind("<leader>up", Action::ShowPath, "show path", Cat::General, Ctx::Global)
            .describe_prefix("<leader>s", "search")
            .bind("<leader>sf", Action::FuzzyNotes, "fuzzy search notes", Cat::General, Ctx::Global);

        keymap.finalize().expect("keymap has missing descriptions");
        keymap
    }

    pub fn empty() -> Self {
        Self {
            bindings: Vec::new(),
            leader_key: Key::Char(' '),
        }
    }

    pub fn with_leader(leader: Key) -> Self {
        Self {
            bindings: Vec::new(),
            leader_key: leader,
        }
    }

    fn resolve_leader_keys(&self, keys: &[Key]) -> Vec<Key> {
        keys.iter()
            .map(|k| {
                if *k == Key::Leader {
                    self.leader_key
                } else {
                    *k
                }
            })
            .collect()
    }

    pub fn describe<F>(&mut self, prefix: &str, description: &'static str, bindings: F) -> &mut Self
    where
        F: FnOnce(&mut super::GroupBuilder),
    {
        let prefix_keys = parse_key_sequence(prefix);
        if prefix_keys.is_empty() {
            return self;
        }
        let resolved_keys = self.resolve_leader_keys(&prefix_keys);
        self.ensure_branch_with_description(&resolved_keys, description);
        let mut builder = super::GroupBuilder::new(self, resolved_keys);
        bindings(&mut builder);
        self
    }

    pub fn describe_prefix(&mut self, prefix: &str, description: &'static str) -> &mut Self {
        let prefix_keys = parse_key_sequence(prefix);
        if prefix_keys.is_empty() {
            return self;
        }
        let resolved_keys = self.resolve_leader_keys(&prefix_keys);
        self.ensure_branch_with_description(&resolved_keys, description);
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
        let resolved_keys = self.resolve_leader_keys(&keys);
        self.insert_into_tree(&resolved_keys, action, description, category, context);
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
        let resolved_keys = self.resolve_leader_keys(keys);
        if resolved_keys.is_empty() {
            return None;
        }
        let first_child = self.bindings.iter().find(|c| c.key == resolved_keys[0])?;
        Self::traverse_to_node(first_child, &resolved_keys[1..])
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
        let resolved_keys = self.resolve_leader_keys(keys);
        let node = self.get_node_at_path(&resolved_keys)?;
        match node {
            KeyNode::Branch { children, .. } => Some(children),
            KeyNode::Leaf { .. } => None,
        }
    }

    pub fn is_prefix_key(&self, key: Key) -> bool {
        self.bindings
            .iter()
            .any(|c| c.key == key && c.node.is_branch())
            || (key == self.leader_key && self.has_leader_bindings())
    }

    fn has_leader_bindings(&self) -> bool {
        self.bindings
            .iter()
            .any(|c| c.key == self.leader_key && c.node.is_branch())
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

impl Default for Keymap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leader_key_resolves_to_space_by_default() {
        let keymap = Keymap::new();
        assert_eq!(keymap.leader_key, Key::Char(' '));
    }

    #[test]
    fn custom_leader_key_resolves_correctly() {
        let keymap = Keymap::with_leader(Key::Char(','));
        assert_eq!(keymap.leader_key, Key::Char(','));
    }
}
