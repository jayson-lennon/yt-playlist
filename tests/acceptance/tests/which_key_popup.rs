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

#![allow(clippy::missing_panics_doc)]
use acceptance::ShownotesWorld;
use crossterm::event::{Event, KeyCode, KeyEvent};
use cucumber::{given, then, when, World};
use shownotes::feat::keymap::Key;

#[derive(Debug, World)]
#[world(init = Self::new_world)]
pub struct WhichKeyWorld {
    inner: ShownotesWorld,
}

impl WhichKeyWorld {
    fn new_world() -> Self {
        Self {
            inner: ShownotesWorld::new(),
        }
    }
}

#[given(expr = r#"the which-key popup is showing"#)]
fn given_which_key_showing(world: &mut WhichKeyWorld) {
    let app = world.inner.app.as_mut().expect("app not initialized");
    app.tui_state.global_handler.toggle_help();
    assert!(app.tui_state.global_handler.is_showing_help());
}

#[when(expr = r#"I press the prefix key {string}"#)]
fn when_press_prefix_key(world: &mut WhichKeyWorld, key: String) {
    let c = key.chars().next().expect("key must have a character");
    world
        .inner
        .handle_event(Event::Key(KeyEvent::from(KeyCode::Char(c))));
}

#[when(expr = r#"I press the action key {string}"#)]
fn when_press_action_key(world: &mut WhichKeyWorld, key: String) {
    let c = key.chars().next().expect("key must have a character");
    world
        .inner
        .handle_event(Event::Key(KeyEvent::from(KeyCode::Char(c))));
}

#[when(expr = r#"I press Escape"#)]
fn when_press_escape(world: &mut WhichKeyWorld) {
    world
        .inner
        .handle_event(Event::Key(KeyEvent::from(KeyCode::Esc)));
}

#[then(expr = r#"the which-key popup is dismissed"#)]
fn then_popup_dismissed(world: &mut WhichKeyWorld) {
    let app = world.inner.app.as_mut().expect("app not initialized");
    assert!(!app.tui_state.global_handler.is_showing_help());
}

#[then(expr = r#"the which-key is pending with key {string}"#)]
fn then_which_key_pending_with_key(world: &mut WhichKeyWorld, key: String) {
    let app = world.inner.app.as_mut().expect("app not initialized");
    assert!(app.tui_state.global_handler.is_which_key_pending());
    let c = key.chars().next().expect("key must have a character");
    let pending = app.tui_state.global_handler.pending_keys();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0], Key::Char(c));
}

#[then(expr = r#"the result contains the Quit action"#)]
fn then_result_contains_quit(world: &mut WhichKeyWorld) {
    let app = world.inner.app.as_mut().expect("app not initialized");
    assert!(app.should_quit);
}

#[then(expr = r#"the result contains no actions"#)]
fn then_result_contains_no_actions(world: &mut WhichKeyWorld) {
    let app = world.inner.app.as_mut().expect("app not initialized");
    assert!(!app.tui_state.global_handler.is_showing_help());
    assert!(!app.should_quit);
}

fn main() {
    let rt = tokio::runtime::Runtime::new().expect("failed to create runtime");
    rt.block_on(WhichKeyWorld::run("tests/features/which_key_popup.feature"));
}
