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

use crossterm::event::KeyEvent;

use super::event::HandleKeyResult;
use super::Pane;
use crate::feat::keymap::Keymap;

/// Context passed to components during key handling.
///
/// Provides access to shared state needed by some components
/// (like WhichKey which needs the keymap for tree traversal,
/// or components that need to know the focused_pane).
pub struct ComponentContext<'a> {
    /// Keymap for key binding lookups and which-key display.
    pub keymap: &'a Keymap,
    /// Currently focused pane in the UI.
    pub focused_pane: Pane,
}

/// Trait for UI components that can handle keyboard input.
///
/// Components implement this trait to handle their own input.
/// Events bubble up through the component hierarchy - if a component
/// returns `HandleKeyResult::ignored()`, the event passes to the next handler.
pub trait Component {
    /// Returns true if this component is currently active and should receive events.
    ///
    /// Active components get priority for event handling.
    fn is_active(&self) -> bool {
        false
    }

    /// Handle a key event.
    ///
    /// Returns `HandleKeyResult::consumed()` if the event was handled, or
    /// `HandleKeyResult::ignored()` to let it bubble to the next handler.
    fn handle_key(&mut self, _key: KeyEvent) -> HandleKeyResult {
        HandleKeyResult::ignored()
    }

    /// Handle a key event with context.
    ///
    /// Default implementation delegates to `handle_key`. Components that
    /// need access to the keymap (like WhichKey) override this method.
    fn handle_key_with_context(
        &mut self,
        key: KeyEvent,
        _ctx: &ComponentContext<'_>,
    ) -> HandleKeyResult {
        self.handle_key(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct InactiveComponent;

    impl Component for InactiveComponent {
        fn handle_key(&mut self, _key: KeyEvent) -> HandleKeyResult {
            HandleKeyResult::consumed()
        }
    }

    struct ActiveComponent;

    impl Component for ActiveComponent {
        fn is_active(&self) -> bool {
            true
        }

        fn handle_key(&mut self, _key: KeyEvent) -> HandleKeyResult {
            HandleKeyResult::consumed()
        }
    }

    struct IgnoringComponent;

    impl Component for IgnoringComponent {}

    struct ContextUsingComponent {
        last_key: Option<KeyEvent>,
    }

    impl Component for ContextUsingComponent {
        fn handle_key_with_context(
            &mut self,
            key: KeyEvent,
            _ctx: &ComponentContext<'_>,
        ) -> HandleKeyResult {
            self.last_key = Some(key);
            HandleKeyResult::consumed()
        }
    }

    #[test]
    fn default_is_active_returns_false() {
        // Given an inactive component.
        let component = InactiveComponent;

        // When checking is_active.
        // Then it returns false.
        assert!(!component.is_active());
    }

    #[test]
    fn overridden_is_active_returns_true() {
        // Given an active component.
        let component = ActiveComponent;

        // When checking is_active.
        // Then it returns true.
        assert!(component.is_active());
    }

    #[test]
    fn default_handle_key_returns_ignored() {
        // Given a component that ignores keys.
        let mut component = IgnoringComponent;
        let key = KeyEvent::from(crossterm::event::KeyCode::Char('a'));

        // When handling a key.
        let result = component.handle_key(key);

        // Then the result is not consumed.
        assert!(!result.is_consumed());
    }

    #[test]
    fn overridden_handle_key_returns_consumed() {
        // Given a component that consumes keys.
        let mut component = ActiveComponent;
        let key = KeyEvent::from(crossterm::event::KeyCode::Char('a'));

        // When handling a key.
        let result = component.handle_key(key);

        // Then the result is consumed.
        assert!(result.is_consumed());
    }

    #[test]
    fn handle_key_with_context_delegates_to_handle_key() {
        // Given a component with default context handling.
        let mut component = IgnoringComponent;
        let key = KeyEvent::from(crossterm::event::KeyCode::Char('a'));
        let keymap = Keymap::default();
        let ctx = ComponentContext {
            keymap: &keymap,
            focused_pane: Pane::Playlist,
        };

        // When handling a key with context.
        let result = component.handle_key_with_context(key, &ctx);

        // Then the result delegates to handle_key and is not consumed.
        assert!(!result.is_consumed());
    }

    #[test]
    fn handle_key_with_context_can_be_overridden() {
        // Given a component that overrides handle_key_with_context.
        let mut component = ContextUsingComponent { last_key: None };
        let key = KeyEvent::from(crossterm::event::KeyCode::Char('x'));
        let keymap = Keymap::default();
        let ctx = ComponentContext {
            keymap: &keymap,
            focused_pane: Pane::Playlist,
        };

        // When handling a key with context.
        let result = component.handle_key_with_context(key, &ctx);

        // Then the result is consumed and the key was stored.
        assert!(result.is_consumed());
        assert!(component.last_key.is_some());
    }
}
