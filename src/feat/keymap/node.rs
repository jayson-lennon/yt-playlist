use super::{Action, Key};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyContext {
    Global,
    Playlist,
    Library,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCategory {
    Navigation,
    PaneSwitch,
    ItemActions,
    PlaylistActions,
    General,
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

#[derive(Debug, Clone)]
pub struct LeafBinding {
    pub key: Key,
    pub action: Action,
    pub description: &'static str,
    pub category: KeyCategory,
    pub context: KeyContext,
}
