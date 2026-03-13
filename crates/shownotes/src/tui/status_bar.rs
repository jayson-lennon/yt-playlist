use ratatui::{
    style::{Color, Style},
    widgets::Paragraph,
};

use super::render::{Render, RenderContext};

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
        let status_bar = StatusBar::new();
        assert!(status_bar.message().is_none());
    }

    #[test]
    fn default_creates_empty_status_bar() {
        let status_bar = StatusBar::default();
        assert!(status_bar.message().is_none());
    }

    #[test]
    fn set_sets_message() {
        let mut status_bar = StatusBar::new();
        status_bar.set("Test message");
        assert_eq!(status_bar.message(), Some("Test message"));
    }

    #[test]
    fn set_accepts_string() {
        let mut status_bar = StatusBar::new();
        status_bar.set(String::from("Test message"));
        assert_eq!(status_bar.message(), Some("Test message"));
    }

    #[test]
    fn clear_removes_message() {
        let mut status_bar = StatusBar::new();
        status_bar.set("Test message");
        status_bar.clear();
        assert!(status_bar.message().is_none());
    }

    #[test]
    fn set_replaces_existing_message() {
        let mut status_bar = StatusBar::new();
        status_bar.set("First message");
        status_bar.set("Second message");
        assert_eq!(status_bar.message(), Some("Second message"));
    }

    #[test]
    fn clear_when_empty_does_nothing() {
        let mut status_bar = StatusBar::new();
        status_bar.clear();
        assert!(status_bar.message().is_none());
    }
}
