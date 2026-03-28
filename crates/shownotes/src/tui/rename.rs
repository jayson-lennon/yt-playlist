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

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::common::PlaylistItem;
use super::component::Component;
use super::event::HandleKeyResult;
use super::render::{Render, RenderContext};
use crate::tui::TuiAction;

/// Rename mode state for editing item aliases.
///
/// Manages the rename input mode where the user can set or modify
/// the alias for a playlist item. The alias is displayed instead of
/// the filename when showing the item in the TUI.
#[derive(Debug, Clone, Default)]
pub struct Rename {
    pub active: bool,
    pub input: String,
}

impl Rename {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn start(&mut self, current_alias: Option<&str>) {
        self.input = current_alias.unwrap_or_default().to_string();
        self.active = true;
    }

    pub fn cancel(&mut self) {
        self.active = false;
        self.input.clear();
    }

    pub fn dismiss(&mut self) {
        self.active = false;
        self.input.clear();
    }

    pub fn push_char(&mut self, c: char) {
        self.input.push(c);
    }

    pub fn pop_char(&mut self) {
        self.input.pop();
    }

    pub fn input(&self) -> &str {
        &self.input
    }

    pub fn render(&self, frame: &mut Frame, selected_item: Option<&PlaylistItem>) {
        let area = popup_area(frame.area(), 50, 20);

        frame.render_widget(Clear, area);

        let chunks = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(3)])
            .split(area);

        let filename = selected_item.map_or_else(
            || "Unknown".to_string(),
            |item| {
                if item.is_virtual || item.path.is_url() {
                    item.path.to_string_lossy().into_owned()
                } else {
                    item.path.file_stem().map_or_else(
                        || item.path.to_string_lossy().into_owned(),
                        std::string::ToString::to_string,
                    )
                }
            },
        );

        let title = Paragraph::new(filename).style(Style::default().fg(Color::Cyan));
        frame.render_widget(title, chunks[0]);

        let input_text = format!("{}█", self.input);
        let input = Paragraph::new(input_text).block(
            Block::default()
                .title("Alias")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        );
        frame.render_widget(input, chunks[1]);
    }
}

impl Render for Rename {
    fn should_render(&self, _ctx: &RenderContext<'_, '_>) -> bool {
        self.is_active()
    }

    fn render(&self, ctx: &mut RenderContext<'_, '_>) {
        let area = popup_area(ctx.frame.area(), 50, 20);
        let selected_item = ctx.tui_state.get_selected_item();

        ctx.frame.render_widget(Clear, area);

        let chunks = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(3)])
            .split(area);

        let filename = selected_item.map_or_else(
            || "Unknown".to_string(),
            |item| {
                if item.is_virtual || item.path.is_url() {
                    item.path.to_string_lossy().into_owned()
                } else {
                    item.path.file_stem().map_or_else(
                        || item.path.to_string_lossy().into_owned(),
                        std::string::ToString::to_string,
                    )
                }
            },
        );

        let title = Paragraph::new(filename).style(Style::default().fg(Color::Cyan));
        ctx.frame.render_widget(title, chunks[0]);

        let input_text = format!("{}█", self.input);
        let input = Paragraph::new(input_text).block(
            Block::default()
                .title("Alias")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        );
        ctx.frame.render_widget(input, chunks[1]);
    }
}

impl Component for Rename {
    fn is_active(&self) -> bool {
        self.active
    }

    fn handle_key(&mut self, key: KeyEvent) -> HandleKeyResult {
        if !self.active {
            return HandleKeyResult::ignored();
        }

        match key.code {
            KeyCode::Esc => {
                self.cancel();
                HandleKeyResult::consumed()
            }
            KeyCode::Enter => {
                let value = self.input.clone();
                self.dismiss();
                HandleKeyResult::with_action(TuiAction::RenameSubmit(value))
            }
            KeyCode::Backspace => {
                self.pop_char();
                HandleKeyResult::consumed()
            }
            KeyCode::Char(c) => {
                self.push_char(c);
                HandleKeyResult::consumed()
            }
            _ => HandleKeyResult::consumed(),
        }
    }
}

fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_initializes_with_empty_string_when_no_alias() {
        // Given a rename component.
        let mut rename = Rename::new();

        // When starting without an existing alias.
        rename.start(None);

        // Then input is empty and active.
        assert!(rename.is_active());
        assert!(rename.input.is_empty());
    }

    #[test]
    fn start_initializes_with_current_alias() {
        // Given a rename component.
        let mut rename = Rename::new();

        // When starting with an existing alias.
        rename.start(Some("My Video"));

        // Then input is set to the alias and active.
        assert!(rename.is_active());
        assert_eq!(rename.input(), "My Video");
    }

    #[test]
    fn cancel_clears_state() {
        // Given an active rename with input.
        let mut rename = Rename::new();
        rename.start(Some("test"));
        rename.push_char('!');

        // When canceling.
        rename.cancel();

        // Then state is cleared and inactive.
        assert!(!rename.is_active());
        assert!(rename.input.is_empty());
    }

    #[test]
    fn push_char_appends_to_input() {
        // Given an active rename.
        let mut rename = Rename::new();
        rename.start(None);

        // When pushing characters.
        rename.push_char('a');
        rename.push_char('b');

        // Then characters are appended.
        assert_eq!(rename.input(), "ab");
    }

    #[test]
    fn pop_char_removes_last_character() {
        // Given an active rename with input.
        let mut rename = Rename::new();
        rename.start(Some("test"));

        // When popping a character.
        rename.pop_char();

        // Then last character is removed.
        assert_eq!(rename.input(), "tes");
    }

    #[test]
    fn pop_char_does_nothing_on_empty_input() {
        // Given an active rename with empty input.
        let mut rename = Rename::new();
        rename.start(None);

        // When popping a character.
        rename.pop_char();

        // Then input remains empty.
        assert!(rename.input().is_empty());
    }

    #[test]
    fn can_edit_existing_alias() {
        // Given a rename started with an existing alias.
        let mut rename = Rename::new();
        rename.start(Some("video"));

        // When editing (pop twice, then add).
        rename.pop_char();
        rename.pop_char();
        rename.push_char('a');

        // Then the alias is modified ("vid" + "a" = "vida").
        assert_eq!(rename.input(), "vida");
    }

    #[test]
    fn multiple_start_cancel_cycles() {
        // Given a rename component.
        let mut rename = Rename::new();

        // When doing multiple cycles.
        rename.start(Some("first"));
        rename.cancel();
        assert!(!rename.is_active());

        rename.start(Some("second"));
        rename.cancel();
        assert!(!rename.is_active());

        // Then state remains clean.
        assert!(rename.input.is_empty());
    }

    #[test]
    fn default_creates_inactive_rename() {
        // Given a default rename.
        let rename = Rename::default();

        // Then it is inactive.
        assert!(!rename.is_active());
        assert!(rename.input.is_empty());
    }

    #[test]
    fn handle_key_returns_ignored_when_inactive() {
        // Given an inactive rename component.
        let mut rename = Rename::new();
        let key = KeyEvent::from(KeyCode::Char('a'));

        // When handling a key.
        let result = rename.handle_key(key);

        // Then the event is ignored.
        assert!(!result.is_consumed());
    }

    #[test]
    fn handle_key_esc_cancels_rename() {
        // Given an active rename component.
        let mut rename = Rename::new();
        rename.start(Some("test"));
        let key = KeyEvent::from(KeyCode::Esc);

        // When handling escape key.
        let result = rename.handle_key(key);

        // Then rename is canceled and event consumed.
        assert!(result.is_consumed());
        assert!(!rename.is_active());
    }

    #[test]
    fn handle_key_enter_submits_rename() {
        // Given an active rename component with input.
        let mut rename = Rename::new();
        rename.start(None);
        rename.push_char('n');
        rename.push_char('e');
        rename.push_char('w');
        let key = KeyEvent::from(KeyCode::Enter);

        // When handling enter key.
        let result = rename.handle_key(key);

        // Then rename is submitted with action and event consumed.
        assert!(result.is_consumed());
        assert!(!rename.is_active());
        assert_eq!(result.actions.len(), 1);
        assert!(matches!(
            &result.actions[0],
            TuiAction::RenameSubmit(s) if s == "new"
        ));
    }

    #[test]
    fn handle_key_backspace_pops_char() {
        // Given an active rename component with input.
        let mut rename = Rename::new();
        rename.start(Some("test"));
        let key = KeyEvent::from(KeyCode::Backspace);

        // When handling backspace key.
        let result = rename.handle_key(key);

        // Then character is removed and event consumed.
        assert!(result.is_consumed());
        assert_eq!(rename.input(), "tes");
    }

    #[test]
    fn handle_key_char_pushes_char() {
        // Given an active rename component.
        let mut rename = Rename::new();
        rename.start(None);
        let key = KeyEvent::from(KeyCode::Char('x'));

        // When handling char key.
        let result = rename.handle_key(key);

        // Then character is added and event consumed.
        assert!(result.is_consumed());
        assert_eq!(rename.input(), "x");
    }

    #[test]
    fn handle_key_consumes_all_keys_when_active() {
        // Given an active rename component.
        let mut rename = Rename::new();
        rename.start(None);

        // When handling various keys.
        let tab_result = rename.handle_key(KeyEvent::from(KeyCode::Tab));
        let up_result = rename.handle_key(KeyEvent::from(KeyCode::Up));
        let f1_result = rename.handle_key(KeyEvent::from(KeyCode::F(1)));

        // Then all keys are consumed.
        assert!(tab_result.is_consumed());
        assert!(up_result.is_consumed());
        assert!(f1_result.is_consumed());
    }
}
