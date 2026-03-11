use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Paragraph,
    Frame,
};

/// Filter state for searching/filtering items in a pane.
///
/// Manages the filter input mode where the user types a search pattern,
/// and tracks whether a filter has been applied. Uses fuzzy matching
/// to filter items by their display names.
#[derive(Debug, Clone, Default)]
pub struct Filter {
    pub active: bool,
    pub input: String,
    pub applied: Option<String>,
    pub previous: Option<String>,
}

impl Filter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn has_applied(&self) -> bool {
        self.applied.is_some()
    }

    pub fn start(&mut self) {
        self.previous = self.applied.take();
        self.input.clear();
        self.active = true;
    }

    pub fn cancel(&mut self) {
        self.applied = self.previous.take();
        self.input.clear();
        self.active = false;
    }

    pub fn submit(&mut self) {
        self.applied = if self.input.is_empty() {
            None
        } else {
            Some(self.input.clone())
        };
        self.previous = None;
        self.input.clear();
        self.active = false;
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

    pub fn applied(&self) -> Option<&str> {
        self.applied.as_deref()
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let filter_text = format!("Filter: {}█", self.input);
        let footer = Paragraph::new(filter_text).style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
        frame.render_widget(footer, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filter_with_applied(applied: &str) -> Filter {
        Filter {
            active: false,
            input: String::new(),
            applied: Some(applied.to_string()),
            previous: None,
        }
    }

    #[test]
    fn start_clears_input_and_saves_previous() {
        // Given a filter with an applied value.
        let mut filter = filter_with_applied("old");

        // When starting the filter.
        filter.start();

        // Then input is cleared and previous is saved.
        assert!(filter.is_active());
        assert!(filter.input.is_empty());
        assert_eq!(filter.previous, Some("old".to_string()));
        assert!(filter.applied.is_none());
    }

    #[test]
    fn cancel_restores_previous_filter() {
        // Given an active filter with a previous value.
        let mut filter = Filter {
            active: true,
            input: "new".to_string(),
            applied: None,
            previous: Some("old".to_string()),
        };

        // When canceling the filter.
        filter.cancel();

        // Then previous is restored and filter is inactive.
        assert!(!filter.is_active());
        assert!(filter.input.is_empty());
        assert_eq!(filter.applied, Some("old".to_string()));
        assert!(filter.previous.is_none());
    }

    #[test]
    fn cancel_clears_state_when_no_previous() {
        // Given an active filter with no previous value.
        let mut filter = Filter {
            active: true,
            input: "new".to_string(),
            applied: None,
            previous: None,
        };

        // When canceling the filter.
        filter.cancel();

        // Then everything is cleared.
        assert!(!filter.is_active());
        assert!(filter.input.is_empty());
        assert!(filter.applied.is_none());
        assert!(filter.previous.is_none());
    }

    #[test]
    fn submit_sets_applied_filter() {
        // Given an active filter with input.
        let mut filter = Filter {
            active: true,
            input: "search".to_string(),
            applied: None,
            previous: Some("old".to_string()),
        };

        // When submitting the filter.
        filter.submit();

        // Then applied is set and filter is inactive.
        assert!(!filter.is_active());
        assert!(filter.input.is_empty());
        assert_eq!(filter.applied, Some("search".to_string()));
        assert!(filter.previous.is_none());
    }

    #[test]
    fn submit_clears_applied_when_input_empty() {
        // Given an active filter with empty input.
        let mut filter = Filter {
            active: true,
            input: String::new(),
            applied: None,
            previous: Some("old".to_string()),
        };

        // When submitting the filter.
        filter.submit();

        // Then applied is None.
        assert!(!filter.is_active());
        assert!(filter.input.is_empty());
        assert!(filter.applied.is_none());
        assert!(filter.previous.is_none());
    }

    #[test]
    fn push_char_appends_to_input() {
        // Given an active filter.
        let mut filter = Filter {
            active: true,
            input: "ab".to_string(),
            applied: None,
            previous: None,
        };

        // When pushing a character.
        filter.push_char('c');

        // Then the character is appended.
        assert_eq!(filter.input, "abc");
    }

    #[test]
    fn pop_char_removes_last_character() {
        // Given an active filter with input.
        let mut filter = Filter {
            active: true,
            input: "abc".to_string(),
            applied: None,
            previous: None,
        };

        // When popping a character.
        filter.pop_char();

        // Then the last character is removed.
        assert_eq!(filter.input, "ab");
    }

    #[test]
    fn pop_char_does_nothing_on_empty_input() {
        // Given an active filter with empty input.
        let mut filter = Filter {
            active: true,
            input: String::new(),
            applied: None,
            previous: None,
        };

        // When popping a character.
        filter.pop_char();

        // Then input remains empty.
        assert!(filter.input.is_empty());
    }

    #[test]
    fn has_applied_returns_true_when_set() {
        // Given a filter with applied value.
        let filter = filter_with_applied("test");

        // Then has_applied returns true.
        assert!(filter.has_applied());
    }

    #[test]
    fn has_applied_returns_false_when_not_set() {
        // Given a filter without applied value.
        let filter = Filter::new();

        // Then has_applied returns false.
        assert!(!filter.has_applied());
    }

    #[test]
    fn multiple_start_cancel_cycles() {
        // Given a filter.
        let mut filter = Filter::new();

        // When doing multiple start/cancel cycles.
        filter.start();
        filter.push_char('a');
        filter.cancel();
        assert!(!filter.is_active());

        filter.start();
        filter.push_char('b');
        filter.cancel();
        assert!(!filter.is_active());

        // Then filter remains in a clean state.
        assert!(filter.input.is_empty());
        assert!(filter.applied.is_none());
    }

    #[test]
    fn start_submit_start_preserves_applied() {
        // Given a filter.
        let mut filter = Filter::new();

        // When submitting a filter then starting again.
        filter.start();
        filter.push_char('x');
        filter.submit();
        assert_eq!(filter.applied, Some("x".to_string()));

        filter.start();
        assert_eq!(filter.previous, Some("x".to_string()));
        assert!(filter.applied.is_none());

        // And canceling restores it.
        filter.cancel();
        assert_eq!(filter.applied, Some("x".to_string()));
    }
}
