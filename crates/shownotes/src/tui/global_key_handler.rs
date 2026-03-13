use crossterm::event::KeyEvent;

use super::component::{Component, ComponentContext};
use super::event::EventResult;
use super::render::{Render, RenderContext};
use super::which_key::{WhichKey, WhichKeyConfig};
use crate::feat::keymap::Key;
use crate::tui::TuiAction;

pub struct GlobalKeyHandler {
    which_key: WhichKey,
    pending_action: Option<TuiAction>,
}

impl GlobalKeyHandler {
    pub fn new(config: WhichKeyConfig) -> Self {
        Self {
            which_key: WhichKey::new(config),
            pending_action: None,
        }
    }

    pub fn take_action(&mut self) -> Option<TuiAction> {
        self.pending_action
            .take()
            .or_else(|| self.which_key.take_action())
    }

    pub fn toggle_help(&mut self) {
        self.which_key.toggle();
    }

    pub fn dismiss_help(&mut self) {
        self.which_key.dismiss();
    }

    pub fn is_showing_help(&self) -> bool {
        self.which_key.active
    }
}

impl Component for GlobalKeyHandler {
    fn is_active(&self) -> bool {
        true
    }

    fn handle_key_with_context(
        &mut self,
        event: KeyEvent,
        ctx: &ComponentContext<'_>,
    ) -> EventResult {
        if self.which_key.is_pending() {
            return self.which_key.handle_key_with_context(event, ctx);
        }

        if self.which_key.active {
            self.which_key.dismiss();
            return EventResult::Consumed;
        }

        let Some(key) = Key::from_keycode(event.code) else {
            return EventResult::Ignored;
        };

        if ctx.keymap.is_prefix_key(key) {
            self.which_key.push_key(key);
            return EventResult::Consumed;
        }

        if let Some(action) = ctx
            .keymap
            .get_action(event.code, event.modifiers, ctx.focused_pane)
        {
            self.pending_action = Some(action);
            return EventResult::Consumed;
        }

        EventResult::Ignored
    }
}

impl Render for GlobalKeyHandler {
    fn should_render(&self, _ctx: &RenderContext<'_, '_>) -> bool {
        self.which_key.is_active()
    }

    fn render(&self, ctx: &mut RenderContext<'_, '_>) {
        Render::render(&self.which_key, ctx);
    }
}
