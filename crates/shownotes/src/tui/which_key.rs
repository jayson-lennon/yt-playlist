use crossterm::event::KeyEvent;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph},
    Frame,
};

use super::component::{Component, ComponentContext};
use super::event::EventResult;
use crate::feat::keymap::{Action, Key, KeyCategory, KeyNode, Keymap, LeafBinding};
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
    pub pending_action: Option<Action>,
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
            pending_action: None,
        }
    }

    pub fn toggle(&mut self) {
        self.active = !self.active;
    }

    pub fn dismiss(&mut self) {
        self.active = false;
        self.pending_keys.clear();
        self.pending_action = None;
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

    pub fn take_action(&mut self) -> Option<Action> {
        self.pending_action.take()
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

impl Component for WhichKey {
    fn is_active(&self) -> bool {
        self.active || self.is_pending()
    }

    fn handle_key_with_context(
        &mut self,
        key: KeyEvent,
        ctx: &ComponentContext<'_>,
    ) -> EventResult {
        if !self.is_pending() {
            return EventResult::Ignored;
        }

        let Some(key) = Key::from_keycode(key.code) else {
            self.dismiss();
            return EventResult::Consumed;
        };

        match key {
            Key::Esc => {
                self.dismiss();
                EventResult::Consumed
            }
            Key::Backspace => {
                self.pop_key();
                if !self.is_pending() {
                    self.dismiss();
                }
                EventResult::Consumed
            }
            _ => {
                let mut path = self.pending_keys.clone();
                path.push(key);

                if let Some(node) = ctx.keymap.get_node_at_path(&path) {
                    match node {
                        KeyNode::Leaf { action, .. } => {
                            self.dismiss();
                            self.pending_action = Some(*action);
                        }
                        KeyNode::Branch { .. } => {
                            self.push_key(key);
                        }
                    }
                } else {
                    self.dismiss();
                }
                EventResult::Consumed
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent};

    use super::*;
    use crate::feat::keymap::{KeyCategory, KeyContext};

    fn make_keymap_with_sequence() -> Keymap {
        let mut keymap = Keymap::empty();
        keymap.describe("g", "general", |g| {
            g.bind(
                "m",
                Action::LaunchMpv,
                "launch mpv",
                KeyCategory::General,
                KeyContext::Global,
            );
        });
        keymap.finalize().unwrap();
        keymap
    }

    fn make_keymap_with_nested_branches() -> Keymap {
        let mut keymap = Keymap::empty();
        keymap.describe("g", "general", |g| {
            g.describe("s", "search", |s| {
                s.bind(
                    "f",
                    Action::FuzzyNotes,
                    "fuzzy notes",
                    KeyCategory::General,
                    KeyContext::Global,
                );
            });
        });
        keymap.finalize().unwrap();
        keymap
    }

    #[test]
    fn handle_key_returns_ignored_when_not_pending() {
        // Given a WhichKey that is not pending
        let mut which_key = WhichKey::default();
        let keymap = Keymap::default();
        let ctx = ComponentContext { keymap: &keymap };

        // When handling a key
        let result = which_key.handle_key_with_context(KeyEvent::from(KeyCode::Char('a')), &ctx);

        // Then the event is ignored
        assert_eq!(result, EventResult::Ignored);
    }

    #[test]
    fn handle_key_esc_dismisses() {
        // Given a WhichKey that is pending
        let mut which_key = WhichKey::default();
        which_key.push_key(Key::Char('g'));
        let keymap = Keymap::default();
        let ctx = ComponentContext { keymap: &keymap };

        // When handling Escape
        let result = which_key.handle_key_with_context(KeyEvent::from(KeyCode::Esc), &ctx);

        // Then the event is consumed and which_key is dismissed
        assert_eq!(result, EventResult::Consumed);
        assert!(!which_key.is_pending());
        assert!(!which_key.active);
    }

    #[test]
    fn handle_key_backspace_pops_key() {
        // Given a WhichKey with multiple pending keys
        let mut which_key = WhichKey::default();
        which_key.push_key(Key::Char('g'));
        which_key.push_key(Key::Char('m'));
        let keymap = make_keymap_with_sequence();
        let ctx = ComponentContext { keymap: &keymap };

        // When handling Backspace
        let result = which_key.handle_key_with_context(KeyEvent::from(KeyCode::Backspace), &ctx);

        // Then the event is consumed and one key is removed
        assert_eq!(result, EventResult::Consumed);
        assert_eq!(which_key.pending_keys, vec![Key::Char('g')]);
    }

    #[test]
    fn handle_key_backspace_dismisses_when_empty() {
        // Given a WhichKey with only one pending key
        let mut which_key = WhichKey::default();
        which_key.push_key(Key::Char('g'));
        let keymap = Keymap::default();
        let ctx = ComponentContext { keymap: &keymap };

        // When handling Backspace
        let result = which_key.handle_key_with_context(KeyEvent::from(KeyCode::Backspace), &ctx);

        // Then the event is consumed and which_key is dismissed
        assert_eq!(result, EventResult::Consumed);
        assert!(!which_key.is_pending());
        assert!(!which_key.active);
    }

    #[test]
    fn handle_key_invalid_key_dismisses() {
        // Given a WhichKey that is pending
        let mut which_key = WhichKey::default();
        which_key.push_key(Key::Char('g'));
        let keymap = Keymap::default();
        let ctx = ComponentContext { keymap: &keymap };

        // When handling an invalid key (F1 is not mapped)
        let result = which_key.handle_key_with_context(KeyEvent::from(KeyCode::F(1)), &ctx);

        // Then the event is consumed and which_key is dismissed
        assert_eq!(result, EventResult::Consumed);
        assert!(!which_key.is_pending());
    }

    #[test]
    fn handle_key_branch_pushes_key() {
        // Given a WhichKey with 'g' pending and a keymap that has nested branches
        let mut which_key = WhichKey::default();
        which_key.push_key(Key::Char('g'));
        let keymap = make_keymap_with_nested_branches();
        let ctx = ComponentContext { keymap: &keymap };

        // When pressing 's' which leads to another branch (not a leaf)
        let result = which_key.handle_key_with_context(KeyEvent::from(KeyCode::Char('s')), &ctx);

        // Then the key is pushed and we're still pending
        assert_eq!(result, EventResult::Consumed);
        assert_eq!(which_key.pending_keys, vec![Key::Char('g'), Key::Char('s')]);
        assert!(which_key.is_pending());
    }

    #[test]
    fn handle_key_leaf_sets_pending_action() {
        // Given a WhichKey that is pending with a keymap that has a leaf at 'gm'
        let mut which_key = WhichKey::default();
        which_key.push_key(Key::Char('g'));
        let keymap = make_keymap_with_sequence();
        let ctx = ComponentContext { keymap: &keymap };

        // When pressing 'm' to complete the sequence "gm"
        let result = which_key.handle_key_with_context(KeyEvent::from(KeyCode::Char('m')), &ctx);

        // Then the event is consumed, action is set, and which_key is dismissed
        assert_eq!(result, EventResult::Consumed);
        assert_eq!(which_key.pending_action, Some(Action::LaunchMpv));
        assert!(!which_key.is_pending());
    }

    #[test]
    fn handle_key_unknown_sequence_dismisses() {
        // Given a WhichKey that is pending
        let mut which_key = WhichKey::default();
        which_key.push_key(Key::Char('g'));
        let keymap = make_keymap_with_sequence();
        let ctx = ComponentContext { keymap: &keymap };

        // When pressing a key that doesn't match any binding
        let result = which_key.handle_key_with_context(KeyEvent::from(KeyCode::Char('z')), &ctx);

        // Then the event is consumed and which_key is dismissed
        assert_eq!(result, EventResult::Consumed);
        assert!(!which_key.is_pending());
        assert!(which_key.pending_action.is_none());
    }

    #[test]
    fn take_action_returns_and_clears_action() {
        // Given a WhichKey with a pending action
        let mut which_key = WhichKey {
            pending_action: Some(Action::Quit),
            ..Default::default()
        };

        // When taking the action
        let action = which_key.take_action();

        // Then the action is returned and cleared
        assert_eq!(action, Some(Action::Quit));
        assert!(which_key.pending_action.is_none());
    }

    #[test]
    fn is_active_returns_true_when_active() {
        // Given a WhichKey that is active but not pending
        let which_key = WhichKey {
            active: true,
            ..Default::default()
        };

        // When checking is_active
        // Then it returns true
        assert!(which_key.is_active());
    }

    #[test]
    fn is_active_returns_true_when_pending() {
        // Given a WhichKey that is pending
        let mut which_key = WhichKey::default();
        which_key.push_key(Key::Char('g'));

        // When checking is_active
        // Then it returns true
        assert!(which_key.is_active());
    }

    #[test]
    fn is_active_returns_false_when_not_active_or_pending() {
        // Given a WhichKey that is neither active nor pending
        let which_key = WhichKey::default();

        // When checking is_active
        // Then it returns false
        assert!(!which_key.is_active());
    }
}
