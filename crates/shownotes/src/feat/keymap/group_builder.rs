use super::{parse_key_sequence, Action, Key, KeyCategory, KeyContext, Keymap};

/// Builder for creating keybinding groups under a prefix.
///
/// Used within `Keymap::describe()` closures to add bindings under
/// a common prefix key. Provides a fluent interface for defining
/// multiple related keybindings.
pub struct GroupBuilder<'a> {
    keymap: &'a mut Keymap,
    prefix: Vec<Key>,
}

impl<'a> GroupBuilder<'a> {
    pub(super) fn new(keymap: &'a mut Keymap, prefix: Vec<Key>) -> Self {
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
