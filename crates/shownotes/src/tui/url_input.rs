use crossterm::event::{KeyCode, KeyEvent};

use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::component::Component;
use super::event::EventResult;
use super::render::{Render, RenderContext};

/// URL input mode state for adding virtual items.
///
/// Manages the URL input mode where the user can add virtual items
/// (like YouTube URLs) to the library. Virtual items don't correspond
/// to local files but can be included in the playlist.
#[derive(Debug, Clone, Default)]
#[allow(clippy::option_option)]
pub struct UrlInput {
    pub active: bool,
    pub input: String,
    submitted: Option<Option<String>>,
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
        self.submitted = None;
    }

    pub fn submit(&mut self) {
        let url = if self.input.is_empty() {
            None
        } else {
            Some(self.input.clone())
        };
        self.active = false;
        self.input.clear();
        self.submitted = Some(url);
    }

    pub fn take_submitted(&mut self) -> Option<Option<String>> {
        self.submitted.take()
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

    fn handle_key(&mut self, key: KeyEvent) -> EventResult {
        if !self.active {
            return EventResult::Ignored;
        }

        match key.code {
            KeyCode::Esc => {
                self.cancel();
                EventResult::Consumed
            }
            KeyCode::Enter => {
                self.submit();
                EventResult::Consumed
            }
            KeyCode::Backspace => {
                self.pop_char();
                EventResult::Consumed
            }
            KeyCode::Char(c) => {
                self.push_char(c);
                EventResult::Consumed
            }
            _ => EventResult::Consumed,
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
        // Given a url input.
        let mut url_input = UrlInput::new();

        // When starting.
        url_input.start();

        // Then input is empty and active.
        assert!(url_input.is_active());
        assert!(url_input.input.is_empty());
    }

    #[test]
    fn cancel_clears_state() {
        // Given an active url input with input.
        let mut url_input = UrlInput::new();
        url_input.start();
        url_input.push_char('h');
        url_input.push_char('t');

        // When canceling.
        url_input.cancel();

        // Then state is cleared and inactive.
        assert!(!url_input.is_active());
        assert!(url_input.input.is_empty());
        assert!(url_input.take_submitted().is_none());
    }

    #[test]
    fn submit_stores_url_when_not_empty() {
        // Given an active url input with input.
        let mut url_input = UrlInput::new();
        url_input.start();
        url_input.push_char('h');
        url_input.push_char('t');
        url_input.push_char('t');
        url_input.push_char('p');

        // When submitting.
        url_input.submit();

        // Then url is stored and state is cleared.
        assert_eq!(url_input.take_submitted(), Some(Some("http".to_string())));
        assert!(!url_input.is_active());
        assert!(url_input.input.is_empty());
    }

    #[test]
    fn submit_stores_none_when_empty() {
        // Given an active url input with empty input.
        let mut url_input = UrlInput::new();
        url_input.start();

        // When submitting.
        url_input.submit();

        // Then Some(None) is stored.
        assert_eq!(url_input.take_submitted(), Some(None));
        assert!(!url_input.is_active());
    }

    #[test]
    fn take_submitted_returns_none_when_not_submitted() {
        // Given a url input.
        let mut url_input = UrlInput::new();

        // When taking submitted without submitting.
        let result = url_input.take_submitted();

        // Then None is returned.
        assert!(result.is_none());
    }

    #[test]
    fn take_submitted_clears_the_stored_value() {
        // Given a url input with a submitted value.
        let mut url_input = UrlInput::new();
        url_input.start();
        url_input.push_char('h');
        url_input.submit();

        // When taking submitted twice.
        let first = url_input.take_submitted();
        let second = url_input.take_submitted();

        // Then first returns the value and second returns None.
        assert_eq!(first, Some(Some("h".to_string())));
        assert!(second.is_none());
    }

    #[test]
    fn push_char_appends_to_input() {
        // Given an active url input.
        let mut url_input = UrlInput::new();
        url_input.start();

        // When pushing characters.
        url_input.push_char('a');
        url_input.push_char('b');

        // Then characters are appended.
        assert_eq!(url_input.input(), "ab");
    }

    #[test]
    fn pop_char_removes_last_character() {
        // Given an active url input with text.
        let mut url_input = UrlInput::new();
        url_input.start();
        url_input.push_char('h');
        url_input.push_char('t');
        url_input.push_char('t');
        url_input.push_char('p');

        // When popping a character.
        url_input.pop_char();

        // Then last character is removed.
        assert_eq!(url_input.input(), "htt");
    }

    #[test]
    fn pop_char_does_nothing_on_empty_input() {
        // Given an active url input with empty input.
        let mut url_input = UrlInput::new();
        url_input.start();

        // When popping a character.
        url_input.pop_char();

        // Then input remains empty.
        assert!(url_input.input().is_empty());
    }

    #[test]
    fn multiple_start_cancel_cycles() {
        // Given a url input.
        let mut url_input = UrlInput::new();

        // When doing multiple cycles.
        url_input.start();
        url_input.push_char('a');
        url_input.cancel();
        assert!(!url_input.is_active());

        url_input.start();
        url_input.push_char('b');
        url_input.cancel();
        assert!(!url_input.is_active());

        // Then state remains clean.
        assert!(url_input.input.is_empty());
    }

    #[test]
    fn default_creates_inactive_url_input() {
        // Given a default url input.
        let url_input = UrlInput::default();

        // Then it is inactive.
        assert!(!url_input.is_active());
        assert!(url_input.input.is_empty());
    }

    #[test]
    fn handle_key_returns_ignored_when_inactive() {
        // Given an inactive url input.
        let mut url_input = UrlInput::new();

        // When handling a key.
        let key = KeyEvent::from(KeyCode::Char('a'));
        let result = url_input.handle_key(key);

        // Then the event is ignored.
        assert_eq!(result, EventResult::Ignored);
    }

    #[test]
    fn handle_key_esc_cancels_url_input() {
        // Given an active url input with text.
        let mut url_input = UrlInput::new();
        url_input.start();
        url_input.push_char('h');

        // When pressing escape.
        let key = KeyEvent::from(KeyCode::Esc);
        let result = url_input.handle_key(key);

        // Then the input is canceled and event consumed.
        assert_eq!(result, EventResult::Consumed);
        assert!(!url_input.is_active());
        assert!(url_input.input.is_empty());
    }

    #[test]
    fn handle_key_enter_submits_url_input() {
        // Given an active url input with text.
        let mut url_input = UrlInput::new();
        url_input.start();
        url_input.push_char('h');
        url_input.push_char('t');

        // When pressing enter.
        let key = KeyEvent::from(KeyCode::Enter);
        let result = url_input.handle_key(key);

        // Then the event is consumed and input is submitted.
        assert_eq!(result, EventResult::Consumed);
        assert!(!url_input.is_active());
    }

    #[test]
    fn handle_key_backspace_pops_char() {
        // Given an active url input with text.
        let mut url_input = UrlInput::new();
        url_input.start();
        url_input.push_char('a');
        url_input.push_char('b');

        // When pressing backspace.
        let key = KeyEvent::from(KeyCode::Backspace);
        let result = url_input.handle_key(key);

        // Then the last character is removed and event consumed.
        assert_eq!(result, EventResult::Consumed);
        assert_eq!(url_input.input(), "a");
    }

    #[test]
    fn handle_key_char_pushes_char() {
        // Given an active url input.
        let mut url_input = UrlInput::new();
        url_input.start();

        // When pressing a character key.
        let key = KeyEvent::from(KeyCode::Char('x'));
        let result = url_input.handle_key(key);

        // Then the character is added and event consumed.
        assert_eq!(result, EventResult::Consumed);
        assert_eq!(url_input.input(), "x");
    }

    #[test]
    fn handle_key_consumes_all_keys_when_active() {
        // Given an active url input.
        let mut url_input = UrlInput::new();
        url_input.start();

        // When pressing any key (e.g., function key).
        let key = KeyEvent::from(KeyCode::F(1));
        let result = url_input.handle_key(key);

        // Then the event is consumed.
        assert_eq!(result, EventResult::Consumed);
    }
}
