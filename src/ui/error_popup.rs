use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

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
