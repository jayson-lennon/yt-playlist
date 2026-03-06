use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Paragraph,
    Frame,
};

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
