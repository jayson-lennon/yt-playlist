use ratatui::{
    style::{Color, Style},
    widgets::Paragraph,
};

use super::render::{Render, RenderContext};

/// Status bar displayed at the bottom of the TUI.
///
/// Shows temporary messages to the user, such as operation results
/// or helpful hints. Messages persist until explicitly cleared.
#[derive(Debug, Clone, Default)]
pub struct StatusBar {
    message: Option<String>,
}

impl StatusBar {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, message: impl Into<String>) {
        self.message = Some(message.into());
    }

    pub fn clear(&mut self) {
        self.message = None;
    }

    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }
}

impl Render for StatusBar {
    fn should_render(&self, _ctx: &RenderContext<'_, '_>) -> bool {
        true
    }

    fn render(&self, ctx: &mut RenderContext<'_, '_>) {
        let status_text = self.message.clone().unwrap_or_default();
        let status = Paragraph::new(status_text).style(Style::default().fg(Color::Yellow));
        ctx.frame.render_widget(status, ctx.area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_empty_status_bar() {
        // Given no setup needed.

        // When creating a new status bar.
        let status_bar = StatusBar::new();

        // Then it has no message.
        assert!(status_bar.message().is_none());
    }

    #[test]
    fn default_creates_empty_status_bar() {
        // Given no setup needed.

        // When creating a default status bar.
        let status_bar = StatusBar::default();

        // Then it has no message.
        assert!(status_bar.message().is_none());
    }

    #[test]
    fn set_sets_message() {
        // Given a new status bar.
        let mut status_bar = StatusBar::new();

        // When setting a message.
        status_bar.set("Test message");

        // Then the message is stored.
        assert_eq!(status_bar.message(), Some("Test message"));
    }

    #[test]
    fn set_accepts_string() {
        // Given a new status bar.
        let mut status_bar = StatusBar::new();

        // When setting a message with a String.
        status_bar.set(String::from("Test message"));

        // Then the message is stored.
        assert_eq!(status_bar.message(), Some("Test message"));
    }

    #[test]
    fn clear_removes_message() {
        // Given a status bar with a message.
        let mut status_bar = StatusBar::new();
        status_bar.set("Test message");

        // When clearing the message.
        status_bar.clear();

        // Then the message is removed.
        assert!(status_bar.message().is_none());
    }

    #[test]
    fn set_replaces_existing_message() {
        // Given a status bar with an existing message.
        let mut status_bar = StatusBar::new();
        status_bar.set("First message");

        // When setting a new message.
        status_bar.set("Second message");

        // Then the new message replaces the old one.
        assert_eq!(status_bar.message(), Some("Second message"));
    }

    #[test]
    fn clear_when_empty_does_nothing() {
        // Given an empty status bar.
        let mut status_bar = StatusBar::new();

        // When clearing the message.
        status_bar.clear();

        // Then the status bar remains empty.
        assert!(status_bar.message().is_none());
    }
}
