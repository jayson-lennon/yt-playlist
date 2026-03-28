// Copyright (C) 2026 Jayson Lennon
// 
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
// 
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
// 
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use super::Key;
use crate::tui::TuiAction;

/// Where a keybinding can be used.
///
/// Determines in which pane(s) a keybinding is active. Global bindings
/// work everywhere, while Playlist and Library bindings only apply
/// when that pane is focused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyContext {
    Global,
    Playlist,
    Library,
}

/// Display group for keybindings in the help box.
///
/// Keybindings are grouped by category in the which-key display to
/// help users find related actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCategory {
    Navigation,
    PaneSwitch,
    ItemActions,
    PlaylistActions,
    General,
}

/// A node in the keybinding tree.
///
/// Either a leaf node containing an action and its metadata, or a branch
/// node containing child nodes for multi-key sequences.
#[derive(Debug, Clone)]
pub enum KeyNode {
    Leaf {
        action: TuiAction,
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

/// A key and its associated node in the tree.
///
/// Pairs a key with the node that should be activated when that key
/// is pressed, enabling tree traversal for multi-key sequences.
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
        action: TuiAction,
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

/// A flattened leaf binding for display purposes.
///
/// Contains all the information needed to display a single keybinding
/// in the which-key help popup.
#[derive(Debug, Clone)]
pub struct LeafBinding {
    pub key: Key,
    pub action: TuiAction,
    pub description: &'static str,
    pub category: KeyCategory,
    pub context: KeyContext,
}
