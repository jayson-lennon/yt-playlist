#![allow(clippy::missing_panics_doc)]
use crossterm::event::{KeyCode, KeyEvent};
use cucumber::{World, given, then, when};
use std::fmt;
use std::sync::OnceLock;

use shownotes::tui::{ComponentContext, TuiState};
use shownotes::{Keymap, Pane, TuiAction};

static KEYMAP: OnceLock<Keymap> = OnceLock::new();

fn get_keymap() -> &'static Keymap {
    KEYMAP.get_or_init(Keymap::new)
}

#[derive(Debug, World)]
#[world(init = Self::new_world)]
pub struct WhichKeyWorld {
    tui_state: TuiState,
    last_result: Option<shownotes::tui::HandleKeyResult>,
}

impl WhichKeyWorld {
    async fn new_world() -> Self {
        Self {
            tui_state: TuiState::new(),
            last_result: None,
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        let ctx = ComponentContext {
            keymap: get_keymap(),
            focused_pane: Pane::Playlist,
        };
        self.last_result = Some(self.tui_state.handle_key(key, &ctx));
    }
}

#[given(expr = r#"the which-key popup is showing"#)]
fn given_which_key_showing(world: &mut WhichKeyWorld) {
    world.tui_state.global_handler.toggle_help();
    assert!(world.tui_state.global_handler.is_showing_help());
}

#[when(expr = r#"I press the prefix key {string}"#)]
fn when_press_prefix_key(world: &mut WhichKeyWorld, key: String) {
    let c = key.chars().next().expect("key must have a character");
    world.handle_key(KeyEvent::from(KeyCode::Char(c)));
}

#[when(expr = r#"I press the action key {string}"#)]
fn when_press_action_key(world: &mut WhichKeyWorld, key: String) {
    let c = key.chars().next().expect("key must have a character");
    world.handle_key(KeyEvent::from(KeyCode::Char(c)));
}

#[when(expr = r#"I press Escape"#)]
fn when_press_escape(world: &mut WhichKeyWorld) {
    world.handle_key(KeyEvent::from(KeyCode::Esc));
}

#[then(expr = r#"the which-key popup is dismissed"#)]
fn then_popup_dismissed(world: &mut WhichKeyWorld) {
    assert!(!world.tui_state.global_handler.is_showing_help());
}

#[then(expr = r#"the which-key is pending with key {string}"#)]
fn then_which_key_pending_with_key(world: &mut WhichKeyWorld, key: String) {
    assert!(world.tui_state.global_handler.is_which_key_pending());
    let c = key.chars().next().expect("key must have a character");
    let pending = world.tui_state.global_handler.pending_keys();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0], shownotes::feat::keymap::Key::Char(c));
}

#[then(expr = r#"the result contains the Quit action"#)]
fn then_result_contains_quit(world: &mut WhichKeyWorld) {
    let result = world.last_result.as_ref().expect("no result recorded");
    assert!(result.actions.contains(&TuiAction::Quit));
}

#[then(expr = r#"the result contains no actions"#)]
fn then_result_contains_no_actions(world: &mut WhichKeyWorld) {
    let result = world.last_result.as_ref().expect("no result recorded");
    assert!(result.actions.is_empty());
}

#[tokio::main]
async fn main() {
    WhichKeyWorld::run("tests/features/which_key_popup.feature").await;
}
