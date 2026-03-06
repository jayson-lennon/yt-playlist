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
