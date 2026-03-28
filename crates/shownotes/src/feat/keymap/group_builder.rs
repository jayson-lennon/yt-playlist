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

use super::{parse_key_sequence, Key, KeyCategory, KeyContext, Keymap};
use crate::tui::TuiAction;

/// Builder for creating keybinding groups under a prefix.
///
/// Used within `Keymap::describe()` closures to add bindings under
/// a common prefix key. Provides a fluent interface for defining
/// multiple related keybindings.
///
/// # Lifetime Parameter
///
/// `'a` ties the builder to the parent [`Keymap`] reference, ensuring
/// bindings are inserted directly into the keymap during construction.
///
/// # Key Methods
///
/// - [`bind`](Self::bind): Add a single keybinding under the prefix
/// - [`describe`](Self::describe): Create a nested group with a description
/// - [`describe_prefix`](Self::describe_prefix): Add a prefix description without bindings
pub struct GroupBuilder<'a> {
    keymap: &'a mut Keymap,
    prefix: Vec<Key>,
}

impl<'a> GroupBuilder<'a> {
    pub(super) fn new(keymap: &'a mut Keymap, prefix: Vec<Key>) -> Self {
        Self { keymap, prefix }
    }

    /// Adds a keybinding under the current prefix.
    ///
    /// The sequence is combined with the builder's prefix to form the full
    /// key sequence. Returns `&mut Self` for method chaining.
    pub fn bind(
        &mut self,
        sequence: &str,
        action: TuiAction,
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

    /// Creates a nested group with a description for the prefix key.
    ///
    /// Useful for organizing related bindings under a sub-prefix with its
    /// own description in the which-key display.
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

    /// Adds a description for a prefix key without adding bindings.
    ///
    /// Use this when a prefix needs a description for the which-key display
    /// but bindings are added separately.
    pub fn describe_prefix(&mut self, prefix: &str, description: &'static str) -> &mut Self {
        let keys = parse_key_sequence(prefix);
        if keys.is_empty() {
            return self;
        }

        let full_prefix: Vec<Key> = self.prefix.iter().copied().chain(keys).collect();
        self.keymap
            .ensure_branch_with_description(&full_prefix, description);
        self
    }
}
