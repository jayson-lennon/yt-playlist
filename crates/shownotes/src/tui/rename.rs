use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::common::PlaylistItem;

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

    pub fn submit(&mut self) -> Option<String> {
        let alias = if self.input.is_empty() {
            None
        } else {
            Some(self.input.clone())
        };
        self.active = false;
        self.input.clear();
        alias
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
                item.path.file_name().map_or_else(
                    || item.path.to_string_lossy().into_owned(),
                    |n| n.to_string_lossy().into_owned(),
                )
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
    fn submit_returns_alias_when_not_empty() {
        // Given an active rename with input.
        let mut rename = Rename::new();
        rename.start(None);
        rename.push_char('n');
        rename.push_char('e');
        rename.push_char('w');

        // When submitting.
        let result = rename.submit();

        // Then alias is returned and state is cleared.
        assert_eq!(result, Some("new".to_string()));
        assert!(!rename.is_active());
        assert!(rename.input.is_empty());
    }

    #[test]
    fn submit_returns_none_when_empty() {
        // Given an active rename with empty input.
        let mut rename = Rename::new();
        rename.start(None);

        // When submitting.
        let result = rename.submit();

        // Then None is returned.
        assert!(result.is_none());
        assert!(!rename.is_active());
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
}
