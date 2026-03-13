use crossterm::event::KeyEvent;

use super::event::EventResult;
use crate::feat::keymap::Keymap;

/// Context passed to components during key handling.
///
/// Provides access to shared state needed by some components
/// (like WhichKey which needs the keymap for tree traversal).
pub struct ComponentContext<'a> {
    pub keymap: &'a Keymap,
}

/// Trait for UI components that can handle keyboard input.
///
/// Components implement this trait to handle their own input.
/// Events bubble up through the component hierarchy - if a component
/// returns `EventResult::Ignored`, the event passes to the next handler.
pub trait Component {
    /// Returns true if this component is currently active and should receive events.
    ///
    /// Active components get priority for event handling.
    fn is_active(&self) -> bool {
        false
    }

    /// Handle a key event.
    ///
    /// Returns `Consumed` if the event was handled, or `Ignored` to let
    /// it bubble to the next handler.
    fn handle_key(&mut self, _key: KeyEvent) -> EventResult {
        EventResult::Ignored
    }

    /// Handle a key event with context.
    ///
    /// Default implementation delegates to `handle_key`. Components that
    /// need access to the keymap (like WhichKey) override this method.
    fn handle_key_with_context(
        &mut self,
        key: KeyEvent,
        _ctx: &ComponentContext<'_>,
    ) -> EventResult {
        self.handle_key(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct InactiveComponent;

    impl Component for InactiveComponent {
        fn handle_key(&mut self, _key: KeyEvent) -> EventResult {
            EventResult::Consumed
        }
    }

    struct ActiveComponent;

    impl Component for ActiveComponent {
        fn is_active(&self) -> bool {
            true
        }

        fn handle_key(&mut self, _key: KeyEvent) -> EventResult {
            EventResult::Consumed
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
        ) -> EventResult {
            self.last_key = Some(key);
            EventResult::Consumed
        }
    }

    #[test]
    fn default_is_active_returns_false() {
        let component = InactiveComponent;
        assert!(!component.is_active());
    }

    #[test]
    fn overridden_is_active_returns_true() {
        let component = ActiveComponent;
        assert!(component.is_active());
    }

    #[test]
    fn default_handle_key_returns_ignored() {
        let mut component = IgnoringComponent;
        let key = KeyEvent::from(crossterm::event::KeyCode::Char('a'));
        assert_eq!(component.handle_key(key), EventResult::Ignored);
    }

    #[test]
    fn overridden_handle_key_returns_consumed() {
        let mut component = ActiveComponent;
        let key = KeyEvent::from(crossterm::event::KeyCode::Char('a'));
        assert_eq!(component.handle_key(key), EventResult::Consumed);
    }

    #[test]
    fn handle_key_with_context_delegates_to_handle_key() {
        let mut component = IgnoringComponent;
        let key = KeyEvent::from(crossterm::event::KeyCode::Char('a'));
        let keymap = Keymap::default();
        let ctx = ComponentContext { keymap: &keymap };

        assert_eq!(
            component.handle_key_with_context(key, &ctx),
            EventResult::Ignored
        );
    }

    #[test]
    fn handle_key_with_context_can_be_overridden() {
        let mut component = ContextUsingComponent { last_key: None };
        let key = KeyEvent::from(crossterm::event::KeyCode::Char('x'));
        let keymap = Keymap::default();
        let ctx = ComponentContext { keymap: &keymap };

        let result = component.handle_key_with_context(key, &ctx);

        assert_eq!(result, EventResult::Consumed);
        assert!(component.last_key.is_some());
    }
}
