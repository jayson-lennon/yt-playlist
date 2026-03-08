use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

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

    pub fn submit(&mut self) -> Option<String> {
        let url = if self.input.is_empty() {
            None
        } else {
            Some(self.input.clone())
        };
        self.active = false;
        self.input.clear();
        url
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
    }

    #[test]
    fn submit_returns_url_when_not_empty() {
        // Given an active url input with input.
        let mut url_input = UrlInput::new();
        url_input.start();
        url_input.push_char('h');
        url_input.push_char('t');
        url_input.push_char('t');
        url_input.push_char('p');

        // When submitting.
        let result = url_input.submit();

        // Then url is returned and state is cleared.
        assert_eq!(result, Some("http".to_string()));
        assert!(!url_input.is_active());
        assert!(url_input.input.is_empty());
    }

    #[test]
    fn submit_returns_none_when_empty() {
        // Given an active url input with empty input.
        let mut url_input = UrlInput::new();
        url_input.start();

        // When submitting.
        let result = url_input.submit();

        // Then None is returned.
        assert!(result.is_none());
        assert!(!url_input.is_active());
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
}
