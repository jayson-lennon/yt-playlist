use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

/// Error popup display state.
///
/// Manages the display of error messages in a modal popup overlay.
/// When active, any key press dismisses the popup and returns to
/// normal operation.
#[derive(Debug, Clone, Default)]
pub struct ErrorPopup {
    pub active: bool,
    pub message: String,
}

impl ErrorPopup {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn show(&mut self, message: String) {
        self.message = message;
        self.active = true;
    }

    pub fn dismiss(&mut self) {
        self.active = false;
        self.message.clear();
    }

    pub fn render(&self, frame: &mut Frame) {
        let area = popup_area(frame.area(), 60, 50);

        frame.render_widget(Clear, area);

        let chunks = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([Constraint::Min(1)])
            .split(area);

        let error_block = Block::default()
            .title(" Error ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red));

        let error_text = Paragraph::new(self.message.clone())
            .block(error_block)
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: false });

        frame.render_widget(error_text, chunks[0]);
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
    fn new_creates_inactive_popup() {
        // Given a new error popup.
        let popup = ErrorPopup::new();

        // Then it is inactive with empty message.
        assert!(!popup.is_active());
        assert!(popup.message.is_empty());
    }

    #[test]
    fn default_creates_inactive_popup() {
        // Given a default error popup.
        let popup = ErrorPopup::default();

        // Then it is inactive.
        assert!(!popup.is_active());
    }

    #[test]
    fn show_sets_message_and_activates() {
        // Given an inactive popup.
        let mut popup = ErrorPopup::new();

        // When showing an error.
        popup.show("Something went wrong".to_string());

        // Then it is active with the message.
        assert!(popup.is_active());
        assert_eq!(popup.message, "Something went wrong");
    }

    #[test]
    fn dismiss_clears_message_and_deactivates() {
        // Given an active popup with a message.
        let mut popup = ErrorPopup::new();
        popup.show("Error message".to_string());

        // When dismissing.
        popup.dismiss();

        // Then it is inactive with empty message.
        assert!(!popup.is_active());
        assert!(popup.message.is_empty());
    }

    #[test]
    fn show_replaces_existing_message() {
        // Given an active popup with a message.
        let mut popup = ErrorPopup::new();
        popup.show("First error".to_string());

        // When showing a new error.
        popup.show("Second error".to_string());

        // Then the message is replaced.
        assert!(popup.is_active());
        assert_eq!(popup.message, "Second error");
    }

    #[test]
    fn dismiss_when_inactive_does_nothing() {
        // Given an inactive popup.
        let mut popup = ErrorPopup::new();

        // When dismissing.
        popup.dismiss();

        // Then it remains inactive.
        assert!(!popup.is_active());
    }

    #[test]
    fn show_dismiss_cycle() {
        // Given a popup.
        let mut popup = ErrorPopup::new();

        // When doing multiple show/dismiss cycles.
        popup.show("Error 1".to_string());
        assert!(popup.is_active());
        popup.dismiss();
        assert!(!popup.is_active());

        popup.show("Error 2".to_string());
        assert!(popup.is_active());
        popup.dismiss();
        assert!(!popup.is_active());

        // Then state is clean.
        assert!(popup.message.is_empty());
    }
}
