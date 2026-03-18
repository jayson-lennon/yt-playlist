use crossterm::event::{KeyCode, KeyEvent};

use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::component::Component;
use super::event::HandleKeyResult;
use super::render::{Render, RenderContext};
use crate::tui::TuiAction;

/// URL input mode state for adding virtual items.
///
/// Manages the URL input mode where the user can add virtual items
/// (like YouTube URLs) to the library. Virtual items don't correspond
/// to local files but can be included in the playlist.
#[derive(Debug, Clone, Default)]
pub struct UrlInput {
    pub active: bool,
    pub input: String,
}

impl UrlInput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn start(&mut self) {
        self.input.clear();
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

    pub fn render(&self, frame: &mut Frame) {
        let area = popup_area(frame.area(), 60, 20);

        frame.render_widget(Clear, area);

        let chunks = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(3)])
            .split(area);

        let title = Paragraph::new("Add URL to Library").style(Style::default().fg(Color::Cyan));
        frame.render_widget(title, chunks[0]);

        let input_text = format!("{}█", self.input);
        let input = Paragraph::new(input_text).block(
            Block::default()
                .title("URL")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        );
        frame.render_widget(input, chunks[1]);
    }
}

impl Render for UrlInput {
    fn should_render(&self, _ctx: &RenderContext<'_, '_>) -> bool {
        self.is_active()
    }

    fn render(&self, ctx: &mut RenderContext<'_, '_>) {
        let area = popup_area(ctx.frame.area(), 60, 20);

        ctx.frame.render_widget(Clear, area);

        let chunks = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(3)])
            .split(area);

        let title = Paragraph::new("Add URL to Library").style(Style::default().fg(Color::Cyan));
        ctx.frame.render_widget(title, chunks[0]);

        let input_text = format!("{}█", self.input);
        let input = Paragraph::new(input_text).block(
            Block::default()
                .title("URL")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        );
        ctx.frame.render_widget(input, chunks[1]);
    }
}

impl Component for UrlInput {
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
                HandleKeyResult::with_action(TuiAction::UrlSubmit(value))
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
    fn start_clears_input_and_activates() {
        // Given a new URL input component.
        let mut url_input = UrlInput::new();

        // When starting input.
        url_input.start();

        // Then it is active with empty input.
        assert!(url_input.is_active());
        assert!(url_input.input.is_empty());
    }

    #[test]
    fn cancel_clears_state() {
        // Given an active URL input with some characters.
        let mut url_input = UrlInput::new();
        url_input.start();
        url_input.push_char('h');
        url_input.push_char('t');

        // When canceling input.
        url_input.cancel();

        // Then it becomes inactive with empty input.
        assert!(!url_input.is_active());
        assert!(url_input.input.is_empty());
    }

    #[test]
    fn push_char_appends_to_input() {
        // Given an active URL input.
        let mut url_input = UrlInput::new();
        url_input.start();

        // When pushing characters.
        url_input.push_char('a');
        url_input.push_char('b');

        // Then the input contains the appended characters.
        assert_eq!(url_input.input(), "ab");
    }

    #[test]
    fn pop_char_removes_last_character() {
        // Given an active URL input with characters.
        let mut url_input = UrlInput::new();
        url_input.start();
        url_input.push_char('h');
        url_input.push_char('t');
        url_input.push_char('t');
        url_input.push_char('p');

        // When popping a character.
        url_input.pop_char();

        // Then the last character is removed.
        assert_eq!(url_input.input(), "htt");
    }

    #[test]
    fn pop_char_does_nothing_on_empty_input() {
        // Given an active URL input with no characters.
        let mut url_input = UrlInput::new();
        url_input.start();

        // When popping a character.
        url_input.pop_char();

        // Then the input remains empty.
        assert!(url_input.input().is_empty());
    }

    #[test]
    fn multiple_start_cancel_cycles() {
        // Given a URL input component.
        let mut url_input = UrlInput::new();

        // When starting and canceling multiple times.
        url_input.start();
        url_input.push_char('a');
        url_input.cancel();
        assert!(!url_input.is_active());

        url_input.start();
        url_input.push_char('b');
        url_input.cancel();
        assert!(!url_input.is_active());

        // Then the input is empty after all cycles.
        assert!(url_input.input.is_empty());
    }

    #[test]
    fn default_creates_inactive_url_input() {
        // Given a default URL input.
        let url_input = UrlInput::default();

        // Then it is inactive with empty input.
        assert!(!url_input.is_active());
        assert!(url_input.input.is_empty());
    }

    #[test]
    fn handle_key_returns_ignored_when_inactive() {
        // Given an inactive URL input.
        let mut url_input = UrlInput::new();

        // When handling a key press.
        let key = KeyEvent::from(KeyCode::Char('a'));
        let result = url_input.handle_key(key);

        // Then the key is not consumed.
        assert!(!result.is_consumed());
    }

    #[test]
    fn handle_key_esc_cancels_url_input() {
        // Given an active URL input with characters.
        let mut url_input = UrlInput::new();
        url_input.start();
        url_input.push_char('h');

        // When pressing Escape.
        let key = KeyEvent::from(KeyCode::Esc);
        let result = url_input.handle_key(key);

        // Then the input is canceled and cleared.
        assert!(result.is_consumed());
        assert!(!url_input.is_active());
        assert!(url_input.input.is_empty());
    }

    #[test]
    fn handle_key_enter_returns_url_submit_action() {
        // Given an active URL input with characters.
        let mut url_input = UrlInput::new();
        url_input.start();
        url_input.push_char('h');
        url_input.push_char('t');

        // When pressing Enter.
        let key = KeyEvent::from(KeyCode::Enter);
        let result = url_input.handle_key(key);

        // Then a UrlSubmit action is returned and input is deactivated.
        assert!(result.is_consumed());
        assert_eq!(result.actions.len(), 1);
        assert_eq!(
            result.actions.first(),
            Some(&TuiAction::UrlSubmit("ht".to_string()))
        );
        assert!(!url_input.is_active());
    }

    #[test]
    fn handle_key_enter_submits_empty_string() {
        // Given an active URL input with no characters.
        let mut url_input = UrlInput::new();
        url_input.start();

        // When pressing Enter.
        let key = KeyEvent::from(KeyCode::Enter);
        let result = url_input.handle_key(key);

        // Then an empty UrlSubmit action is returned.
        assert!(result.is_consumed());
        assert_eq!(result.actions.len(), 1);
        assert_eq!(
            result.actions.first(),
            Some(&TuiAction::UrlSubmit(String::new()))
        );
        assert!(!url_input.is_active());
    }

    #[test]
    fn handle_key_backspace_pops_char() {
        // Given an active URL input with characters.
        let mut url_input = UrlInput::new();
        url_input.start();
        url_input.push_char('a');
        url_input.push_char('b');

        // When pressing Backspace.
        let key = KeyEvent::from(KeyCode::Backspace);
        let result = url_input.handle_key(key);

        // Then the last character is removed.
        assert!(result.is_consumed());
        assert_eq!(url_input.input(), "a");
    }

    #[test]
    fn handle_key_char_pushes_char() {
        // Given an active URL input.
        let mut url_input = UrlInput::new();
        url_input.start();

        // When pressing a character key.
        let key = KeyEvent::from(KeyCode::Char('x'));
        let result = url_input.handle_key(key);

        // Then the character is added to the input.
        assert!(result.is_consumed());
        assert_eq!(url_input.input(), "x");
    }

    #[test]
    fn handle_key_consumes_all_keys_when_active() {
        // Given an active URL input.
        let mut url_input = UrlInput::new();
        url_input.start();

        // When pressing any key (e.g., F1).
        let key = KeyEvent::from(KeyCode::F(1));
        let result = url_input.handle_key(key);

        // Then the key is consumed.
        assert!(result.is_consumed());
    }
}
