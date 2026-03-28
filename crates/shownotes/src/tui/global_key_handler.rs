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

use super::component::{Component, ComponentContext};
use super::event::HandleKeyResult;
use super::render::{Render, RenderContext};
use super::which_key::{WhichKey, WhichKeyConfig};
use crate::feat::keymap::Key;

#[derive(Debug)]
pub struct GlobalKeyHandler {
    which_key: WhichKey,
}

impl GlobalKeyHandler {
    pub fn new(config: WhichKeyConfig) -> Self {
        Self {
            which_key: WhichKey::new(config),
        }
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

    pub fn is_which_key_pending(&self) -> bool {
        self.which_key.is_pending()
    }

    pub fn pending_keys(&self) -> &[Key] {
        &self.which_key.pending_keys
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
    ) -> HandleKeyResult {
        if self.which_key.is_pending() {
            return self.which_key.handle_key_with_context(event, ctx);
        }

        if self.which_key.active {
            self.which_key.dismiss();
        }

        let Some(key) = Key::from_keycode(event.code) else {
            return HandleKeyResult::ignored();
        };

        if ctx.keymap.is_prefix_key(key) {
            self.which_key.push_key(key);
            return HandleKeyResult::consumed();
        }

        if let Some(action) = ctx
            .keymap
            .get_action(event.code, event.modifiers, ctx.focused_pane)
        {
            return HandleKeyResult::with_action(action);
        }

        HandleKeyResult::ignored()
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
