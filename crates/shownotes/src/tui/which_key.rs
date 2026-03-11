use crate::feat::keymap::{Key, KeyCategory, Keymap, LeafBinding};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph},
    Frame,
};

use crate::tui::Pane;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WhichKeyPosition {
    BottomLeft,
    #[default]
    BottomRight,
}

/// Configuration for the which-key popup display.
///
/// Defines the visual styling and dimensions of the which-key help popup
/// that shows available keybindings.
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

/// Which-key popup for showing available keybindings.
///
/// Displays a help popup showing available key sequences when the user
/// presses a prefix key. Supports nested key sequences and shows
/// descriptions for each binding.
#[derive(Debug, Clone, Default)]
pub struct WhichKey {
    pub active: bool,
    pub config: WhichKeyConfig,
    pub pending_keys: Vec<Key>,
}

struct ColumnData<'a> {
    categories: Vec<(&'a str, Vec<DisplayItem<'a>>)>,
    max_key_width: usize,
    max_desc_width: usize,
}

#[derive(Debug, Clone)]
enum DisplayItem<'a> {
    Binding(&'a LeafBinding),
    Sequence { key: Key, description: String },
}

impl DisplayItem<'_> {
    fn key_display(&self) -> String {
        match self {
            DisplayItem::Binding(b) => b.key.display(),
            DisplayItem::Sequence { key, .. } => key.display(),
        }
    }

    fn description(&self) -> &str {
        match self {
            DisplayItem::Binding(b) => b.description,
            DisplayItem::Sequence { description, .. } => description,
        }
    }
}

impl ColumnData<'_> {
    fn content_width(&self) -> usize {
        self.max_key_width + 1 + self.max_desc_width
    }
}

impl WhichKey {
    pub fn new(config: WhichKeyConfig) -> Self {
        Self {
            active: false,
            config,
            pending_keys: Vec::new(),
        }
    }

    pub fn toggle(&mut self) {
        self.active = !self.active;
    }

    pub fn dismiss(&mut self) {
        self.active = false;
        self.pending_keys.clear();
    }

    pub fn push_key(&mut self, key: Key) {
        self.active = true;
        self.pending_keys.push(key);
    }

    pub fn pop_key(&mut self) -> bool {
        self.pending_keys.pop().is_some()
    }

    pub fn is_pending(&self) -> bool {
        !self.pending_keys.is_empty()
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn render(&self, frame: &mut Frame, keymap: &Keymap, pane: Pane) {
        if self.is_pending() {
            self.render_sequence(frame, keymap);
        } else {
            self.render_main(frame, keymap, pane);
        }
    }

    fn format_path(&self) -> String {
        self.pending_keys
            .iter()
            .map(Key::display)
            .collect::<Vec<_>>()
            .join(" > ")
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn render_sequence(&self, frame: &mut Frame, keymap: &Keymap) {
        let children = keymap.get_children_at_path(&self.pending_keys);

        let items: Vec<(Key, &str)> = match children {
            Some(children) => children
                .iter()
                .map(|c| (c.key, c.node.description()))
                .collect(),
            None => return,
        };

        if items.is_empty() {
            return;
        }

        let max_height = self
            .config
            .max_height
            .min((f32::from(frame.area().height) * 0.3).ceil() as u16);

        let max_key_width = items
            .iter()
            .map(|(k, _)| k.display().len())
            .max()
            .unwrap_or(1);
        let max_desc_width = items.iter().map(|(_, d)| d.len()).max().unwrap_or(10);

        let content_width = max_key_width + 1 + max_desc_width;
        #[allow(clippy::cast_possible_truncation)]
        let popup_width =
            (content_width + 4).min(usize::from(frame.area().width.saturating_sub(2))) as u16;
        let popup_height = max_height.min(frame.area().height.saturating_sub(2));

        let x = match self.config.position {
            WhichKeyPosition::BottomLeft => 1,
            WhichKeyPosition::BottomRight => frame
                .area()
                .width
                .saturating_sub(popup_width)
                .saturating_sub(1),
        };
        let y = frame
            .area()
            .height
            .saturating_sub(popup_height)
            .saturating_sub(1);

        let popup_area = Rect::new(x, y, popup_width, popup_height);

        frame.render_widget(Clear, popup_area);

        let title = format!(" {} ", self.format_path());
        let block = Block::default()
            .title(title.as_str())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .padding(Padding::horizontal(1));
        let inner_area = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        let mut y = inner_area.y;
        for (key, desc) in &items {
            if y >= inner_area.bottom() {
                break;
            }

            let key_display = key.display();
            let key_span = Span::styled(
                format!("{key_display:>max_key_width$}"),
                Style::default().fg(Color::Cyan),
            );
            let desc_span = Span::raw(format!(" {desc}"));
            let line = Line::from(vec![key_span, desc_span]);
            let para = Paragraph::new(line);
            frame.render_widget(para, Rect::new(inner_area.x, y, inner_area.width, 1));
            y += 1;
        }
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn render_main(&self, frame: &mut Frame, keymap: &Keymap, pane: Pane) {
        let bindings = keymap.get_bindings_for_pane(pane);
        let categories = Self::group_by_category(&bindings, keymap);

        let max_height = self
            .config
            .max_height
            .min((f32::from(frame.area().height) * 0.3).ceil() as u16);

        let columns = Self::build_columns(&categories, max_height);
        let popup_area = self.calculate_popup_area(frame.area(), &columns, max_height);

        frame.render_widget(Clear, popup_area);

        let block = Block::default()
            .title(" Shortcuts ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .padding(Padding::horizontal(1));
        let inner_area = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        let column_areas = Self::layout_columns(&columns, inner_area);

        for (col_area, col_data) in column_areas.iter().zip(columns.iter()) {
            Self::render_column(frame, *col_area, col_data);
        }
    }

    fn group_by_category<'a>(
        bindings: &'a [LeafBinding],
        keymap: &'a Keymap,
    ) -> Vec<(KeyCategory, Vec<DisplayItem<'a>>)> {
        let mut categories: Vec<(KeyCategory, Vec<DisplayItem<'a>>)> = Vec::new();

        for binding in bindings {
            if let Some((_, items)) = categories
                .iter_mut()
                .find(|(cat, _)| *cat == binding.category)
            {
                items.push(DisplayItem::Binding(binding));
            } else {
                categories.push((binding.category, vec![DisplayItem::Binding(binding)]));
            }
        }

        for child in keymap.get_bindings() {
            if child.node.is_branch() {
                if let Some((_, items)) = categories
                    .iter_mut()
                    .find(|(cat, _)| *cat == KeyCategory::General)
                {
                    items.push(DisplayItem::Sequence {
                        key: child.key,
                        description: child.node.description().to_string(),
                    });
                } else if categories.is_empty()
                    || !categories
                        .iter()
                        .any(|(cat, _)| *cat == KeyCategory::General)
                {
                    categories.push((
                        KeyCategory::General,
                        vec![DisplayItem::Sequence {
                            key: child.key,
                            description: child.node.description().to_string(),
                        }],
                    ));
                }
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

    fn build_columns<'a>(
        categories: &'a [(KeyCategory, Vec<DisplayItem<'a>>)],
        max_height: u16,
    ) -> Vec<ColumnData<'a>> {
        let rows_per_column = max_height.saturating_sub(2) as usize;
        let mut columns: Vec<ColumnData<'a>> = Vec::new();
        let mut current_categories: Vec<(&'a str, Vec<DisplayItem<'a>>)> = Vec::new();
        let mut current_rows = 0usize;

        for (category, items) in categories {
            let category_name = Self::category_name(*category);
            let category_rows = items.len() + 1;

            if current_rows + category_rows > rows_per_column && current_rows > 0 {
                columns.push(Self::build_column_data(current_categories));
                current_categories = Vec::new();
                current_rows = 0;
            }

            current_categories.push((category_name, items.clone()));
            current_rows += category_rows;
        }

        if !current_categories.is_empty() {
            columns.push(Self::build_column_data(current_categories));
        }

        columns
    }

    fn build_column_data<'a>(categories: Vec<(&'a str, Vec<DisplayItem<'a>>)>) -> ColumnData<'a> {
        let max_key_width = categories
            .iter()
            .flat_map(|(_, items)| items.iter())
            .map(|item| item.key_display().len())
            .max()
            .unwrap_or(5);

        let max_desc_width = categories
            .iter()
            .flat_map(|(_, items)| items.iter())
            .map(|item| item.description().len())
            .max()
            .unwrap_or(10);

        ColumnData {
            categories,
            max_key_width,
            max_desc_width,
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    fn calculate_popup_area(
        &self,
        frame_area: Rect,
        columns: &[ColumnData<'_>],
        max_height: u16,
    ) -> Rect {
        let column_gap = 1u16;
        let total_content_width: u16 = columns.iter().map(|c| c.content_width() as u16).sum();
        let total_gap = column_gap * columns.len().saturating_sub(1) as u16;

        let popup_width =
            (total_content_width + total_gap + 4).min(frame_area.width.saturating_sub(2));
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

        Rect::new(x, y, popup_width, popup_height)
    }

    #[allow(clippy::cast_possible_truncation)]
    fn layout_columns(columns: &[ColumnData<'_>], inner_area: Rect) -> Vec<Rect> {
        let column_gap = 1u16;
        let mut result = Vec::with_capacity(columns.len());
        let mut x = inner_area.x;

        for column_data in columns {
            let width = column_data.content_width() as u16;
            result.push(Rect::new(x, inner_area.y, width, inner_area.height));
            x += width + column_gap;
        }

        result
    }

    fn render_column(frame: &mut Frame, area: Rect, column_data: &ColumnData<'_>) {
        let mut y = area.y;

        for (category_name, items) in &column_data.categories {
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

            for item in items {
                if y >= area.bottom() {
                    break;
                }

                let key = item.key_display();
                let key_span = Span::styled(
                    format!("{:>width$}", key, width = column_data.max_key_width),
                    Style::default().fg(Color::Cyan),
                );
                let desc_span = Span::raw(format!(" {}", item.description()));
                let line = Line::from(vec![key_span, desc_span]);
                let para = Paragraph::new(line);
                frame.render_widget(para, Rect::new(area.x, y, area.width, 1));
                y += 1;
            }
        }
    }
}
