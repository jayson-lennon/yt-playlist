use crate::keymap::{KeyBinding, KeyCategory, Keymap};
use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Padding, Paragraph},
    Frame,
};

use crate::ui::Pane;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WhichKeyPosition {
    BottomLeft,
    #[default]
    BottomRight,
}

#[derive(Debug, Clone)]
pub struct WhichKeyConfig {
    pub max_height: u16,
    pub position: WhichKeyPosition,
}

impl Default for WhichKeyConfig {
    fn default() -> Self {
        Self {
            max_height: 10,
            position: WhichKeyPosition::default(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct WhichKey {
    pub active: bool,
    pub config: WhichKeyConfig,
}

impl WhichKey {
    pub fn new(config: WhichKeyConfig) -> Self {
        Self {
            active: false,
            config,
        }
    }

    pub fn toggle(&mut self) {
        self.active = !self.active;
    }

    pub fn dismiss(&mut self) {
        self.active = false;
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn render(&self, frame: &mut Frame, keymap: &Keymap, pane: Pane) {
        let bindings = keymap.get_bindings_for_pane(pane);
        let categories = Self::group_by_category(&bindings);

        let max_height = self
            .config
            .max_height
            .min((f32::from(frame.area().height) * 0.3).ceil() as u16);

        let (popup_area, content_width) =
            self.calculate_popup_area(frame.area(), &categories, max_height);

        frame.render_widget(Clear, popup_area);

        let block = Block::default()
            .title(" Shortcuts ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .padding(Padding::horizontal(1));
        let inner_area = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        let columns = Self::layout_columns(&categories, max_height, content_width, inner_area);

        for (col_area, col_bindings) in columns
            .iter()
            .zip(Self::column_bindings(&categories, max_height).iter())
        {
            Self::render_column(frame, *col_area, col_bindings);
        }
    }

    fn group_by_category<'a>(
        bindings: &[&'a KeyBinding],
    ) -> Vec<(KeyCategory, Vec<&'a KeyBinding>)> {
        let mut categories: Vec<(KeyCategory, Vec<&'a KeyBinding>)> = Vec::new();

        for binding in bindings {
            if let Some((_, items)) = categories
                .iter_mut()
                .find(|(cat, _)| *cat == binding.category)
            {
                items.push(*binding);
            } else {
                categories.push((binding.category, vec![*binding]));
            }
        }

        let category_order = [
            KeyCategory::General,
            KeyCategory::Navigation,
            KeyCategory::PaneSwitch,
            KeyCategory::ItemActions,
            KeyCategory::PlaylistActions,
        ];

        categories
            .sort_by_key(|(cat, _)| category_order.iter().position(|c| c == cat).unwrap_or(999));

        categories
    }

    fn category_name(category: KeyCategory) -> &'static str {
        match category {
            KeyCategory::General => "General",
            KeyCategory::Navigation => "Navigation",
            KeyCategory::PaneSwitch => "Panes",
            KeyCategory::ItemActions => "Actions",
            KeyCategory::PlaylistActions => "Playlist",
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    fn calculate_popup_area(
        &self,
        frame_area: Rect,
        categories: &[(KeyCategory, Vec<&KeyBinding>)],
        max_height: u16,
    ) -> (Rect, u16) {
        let total_bindings: usize = categories
            .iter()
            .map(|(_, items): &(_, Vec<&KeyBinding>)| items.len())
            .sum();
        let category_count = categories.len();

        let longest_key = categories
            .iter()
            .flat_map(|(_, items): &(_, Vec<&KeyBinding>)| items.iter())
            .map(|b: &&KeyBinding| b.key_display().len())
            .max()
            .unwrap_or(5);

        let longest_desc = categories
            .iter()
            .flat_map(|(_, items): &(_, Vec<&KeyBinding>)| items.iter())
            .map(|b: &&KeyBinding| b.description.len())
            .max()
            .unwrap_or(10);

        let content_width = (longest_key + longest_desc + 7) as u16;
        let column_gap = 2u16;

        let rows_per_column = max_height.saturating_sub(2);
        let items_per_column = rows_per_column as usize;

        let total_rows = total_bindings + category_count;
        let num_columns = ((total_rows + items_per_column - 1) / items_per_column.max(1)).max(1);

        let popup_width = (num_columns as u16 * content_width
            + (num_columns.saturating_sub(1) as u16) * column_gap
            + 4)
        .min(frame_area.width.saturating_sub(2));
        let popup_height = max_height.min(frame_area.height.saturating_sub(2));

        let x = match self.config.position {
            WhichKeyPosition::BottomLeft => 1,
            WhichKeyPosition::BottomRight => frame_area
                .width
                .saturating_sub(popup_width)
                .saturating_sub(1),
        };
        let y = frame_area
            .height
            .saturating_sub(popup_height)
            .saturating_sub(1);

        (Rect::new(x, y, popup_width, popup_height), content_width)
    }

    #[allow(clippy::cast_possible_truncation)]
    fn layout_columns(
        categories: &[(KeyCategory, Vec<&KeyBinding>)],
        max_height: u16,
        _content_width: u16,
        inner_area: Rect,
    ) -> Vec<Rect> {
        let rows_per_column = max_height.saturating_sub(2) as usize;
        let mut column_count = 0usize;
        let mut current_rows = 0usize;

        for (_, items) in categories {
            let category_rows = items.len() + 1;
            if current_rows + category_rows > rows_per_column && current_rows > 0 {
                column_count += 1;
                current_rows = category_rows;
            } else {
                current_rows += category_rows;
            }
        }
        if current_rows > 0 {
            column_count += 1;
        }

        let column_gap = 1u16;
        let total_gap = column_gap * column_count.saturating_sub(1) as u16;
        let available_width = inner_area.width.saturating_sub(total_gap);
        let col_width = available_width / column_count.max(1) as u16;

        let constraints: Vec<Constraint> = (0..column_count)
            .map(|_| Constraint::Length(col_width))
            .collect();

        let layout = Layout::horizontal(constraints)
            .flex(Flex::Start)
            .split(inner_area);

        layout.to_vec()
    }

    fn column_bindings<'a>(
        categories: &'a [(KeyCategory, Vec<&'a KeyBinding>)],
        max_height: u16,
    ) -> Vec<Vec<(&'a str, Vec<&'a KeyBinding>)>> {
        let rows_per_column = max_height.saturating_sub(2) as usize;
        let mut columns: Vec<Vec<(&'a str, Vec<&'a KeyBinding>)>> = Vec::new();
        let mut current_column: Vec<(&'a str, Vec<&'a KeyBinding>)> = Vec::new();
        let mut current_rows = 0usize;

        for (category, items) in categories {
            let category_name = Self::category_name(*category);
            let category_rows = items.len() + 1;

            if current_rows + category_rows > rows_per_column && current_rows > 0 {
                columns.push(current_column);
                current_column = Vec::new();
                current_rows = 0;
            }

            current_column.push((category_name, items.clone()));
            current_rows += category_rows;
        }

        if !current_column.is_empty() {
            columns.push(current_column);
        }

        columns
    }

    fn render_column(frame: &mut Frame, area: Rect, column_data: &[(&str, Vec<&KeyBinding>)]) {
        let mut y = area.y;

        for (category_name, bindings) in column_data {
            if y >= area.bottom() {
                break;
            }

            let header = Paragraph::new(*category_name).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );
            frame.render_widget(header, Rect::new(area.x, y, area.width, 1));
            y += 1;

            for binding in bindings {
                if y >= area.bottom() {
                    break;
                }

                let key = binding.key_display();
                let text = format!("{:<4} {}", key, binding.description);
                let para = Paragraph::new(text).style(Style::default().fg(Color::White));
                frame.render_widget(para, Rect::new(area.x, y, area.width, 1));
                y += 1;
            }
        }
    }
}
