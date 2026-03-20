use std::time::Duration;

pub use crate::common::domain::{get_mime_type, ItemPath, PlaylistItem};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
};

/// Controls how playlist items are displayed in the TUI.
///
/// Determines what text is shown as the primary identifier for each
/// playlist item in the list view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ItemDisplayMode {
    /// Show the file path or filename as the primary text.
    Path,
    /// Show the user-defined alias if available, falling back to filename.
    #[default]
    Alias,
}

/// Identifies which pane is currently focused in the TUI.
///
/// The application displays two side-by-side panes: the playlist being
/// edited and the library of available files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    /// The active playlist being edited by the user.
    Playlist,
    /// Available files from the working directory that can be added.
    Library,
}

pub fn format_duration(duration: Option<Duration>) -> String {
    match duration {
        Some(d) => {
            let total_secs = d.as_secs();
            let hours = total_secs / 3600;
            let mins = (total_secs % 3600) / 60;
            let secs = total_secs % 60;
            format!("[{hours:02}:{mins:02}:{secs:02}]")
        }
        None => "[--:--:--]".to_string(),
    }
}

pub fn get_display_name(item: &PlaylistItem) -> String {
    item.alias.clone().unwrap_or_else(|| {
        item.path.file_stem().map_or_else(
            || item.path.to_string_lossy().into_owned(),
            std::string::ToString::to_string,
        )
    })
}

pub fn format_mime_type(mime: Option<&str>) -> String {
    match mime {
        Some(m) => {
            if let Some(subtype) = m.strip_prefix("application/") {
                subtype.to_string()
            } else {
                m.to_string()
            }
        }
        None => "unknown".to_string(),
    }
}

pub fn format_item_line(
    item: &PlaylistItem,
    display_mode: ItemDisplayMode,
    pane_width: u16,
    playlist_count: usize,
    min_count: usize,
) -> String {
    let mime_str = format_mime_type(item.mime_type.as_deref());
    let duration_str = format_duration(item.duration);

    let filename = if item.is_virtual || item.path.is_url() {
        item.path.to_string_lossy().into_owned()
    } else {
        item.path.file_stem().map_or_else(
            || item.path.to_string_lossy().into_owned(),
            std::string::ToString::to_string,
        )
    };

    let primary = match display_mode {
        ItemDisplayMode::Alias => item.alias.clone().unwrap_or(filename),
        ItemDisplayMode::Path => filename,
    };

    let base_line = match display_mode {
        ItemDisplayMode::Alias => format!("[{mime_str}] {duration_str} {primary}"),
        ItemDisplayMode::Path => match &item.alias {
            Some(alias) => format!("[{mime_str}] {duration_str} {primary} / {alias}"),
            None => format!("[{mime_str}] {duration_str} {primary}"),
        },
    };

    if playlist_count >= min_count && pane_width > 0 {
        let count_str = format!("({playlist_count})");
        let base_len = base_line.chars().count();
        let count_len = count_str.chars().count();

        if base_len + count_len < pane_width as usize {
            let padding = pane_width as usize - base_len - count_len;
            format!("{}{}{}", base_line, " ".repeat(padding), count_str)
        } else {
            base_line
        }
    } else {
        base_line
    }
}

pub fn filter_items<'a>(
    items: &'a [PlaylistItem],
    filter_input: &str,
    applied_filter: Option<&String>,
) -> Vec<(usize, &'a PlaylistItem)> {
    let active_filter = if filter_input.is_empty() {
        applied_filter.map(String::as_str)
    } else {
        Some(filter_input)
    };

    match active_filter {
        None => items.iter().enumerate().collect(),
        Some(pattern) => {
            let matcher = SkimMatcherV2::default();
            let mut results: Vec<(i64, usize, &PlaylistItem)> = items
                .iter()
                .enumerate()
                .filter_map(|(idx, item)| {
                    let name = get_display_name(item);
                    matcher
                        .fuzzy_match(&name, pattern)
                        .map(|score| (score, idx, item))
                })
                .collect();
            results.sort_by(|a, b| b.0.cmp(&a.0));
            results
                .into_iter()
                .map(|(_, idx, item)| (idx, item))
                .collect()
        }
    }
}

pub fn split_pane_layout(area: Rect) -> (Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);
    (chunks[0], chunks[1])
}

pub fn item_style(is_selected: bool, file_missing: bool, has_sources: bool) -> Style {
    if is_selected {
        if file_missing {
            Style::default()
                .fg(Color::Red)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        }
    } else if file_missing {
        Style::default().fg(Color::Red)
    } else if !has_sources {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    }
}

pub fn focused_border_style(is_focused: bool) -> Style {
    if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    }
}

pub fn focused_text_style(is_focused: bool) -> Style {
    if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    }
}

pub fn pane_title(base: &str, has_filter: bool, is_focused: bool) -> String {
    if has_filter {
        if is_focused {
            format!(" {base} [filtered] [*] ")
        } else {
            format!(" {base} [filtered] ")
        }
    } else if is_focused {
        format!(" {base} [*] ")
    } else {
        format!(" {base} ")
    }
}

pub fn total_duration<'a>(items: impl Iterator<Item = &'a PlaylistItem>) -> Duration {
    items.filter_map(|item| item.duration).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use marked_path::CanonicalPath;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_file(dir: &TempDir, name: &str) -> CanonicalPath {
        let path = dir.path().join(name);
        std::fs::File::create(&path)
            .unwrap()
            .write_all(b"")
            .unwrap();
        CanonicalPath::from_path(&path).unwrap()
    }

    fn create_test_dir(dir: &TempDir, name: &str) -> CanonicalPath {
        let path = dir.path().join(name);
        std::fs::create_dir(&path).unwrap();
        CanonicalPath::from_path(&path).unwrap()
    }

    fn item(path: &CanonicalPath) -> PlaylistItem {
        PlaylistItem {
            path: ItemPath::File(path.clone()),
            duration: None,
            alias: None,
            mime_type: None,
            is_virtual: false,
            playlist_count: 0,
            has_sources: true,
        }
    }

    fn item_with_alias(path: &CanonicalPath, alias: &str) -> PlaylistItem {
        PlaylistItem {
            path: ItemPath::File(path.clone()),
            duration: None,
            alias: Some(alias.to_string()),
            mime_type: None,
            is_virtual: false,
            playlist_count: 0,
            has_sources: true,
        }
    }

    #[allow(dead_code)]
    fn item_with_duration(path: &CanonicalPath, secs: u64) -> PlaylistItem {
        PlaylistItem {
            path: ItemPath::File(path.clone()),
            duration: Some(Duration::from_secs(secs)),
            alias: None,
            mime_type: None,
            is_virtual: false,
            playlist_count: 0,
            has_sources: true,
        }
    }

    #[test]
    fn format_duration_formats_seconds() {
        // Given a duration of 65 seconds.
        let duration = Duration::from_secs(65);

        // When formatting the duration.
        let result = format_duration(Some(duration));

        // Then it displays as [00:01:05].
        assert_eq!(result, "[00:01:05]");
    }

    #[test]
    fn format_duration_formats_hours() {
        // Given a duration of 3661 seconds (1 hour, 1 minute, 1 second).
        let duration = Duration::from_secs(3661);

        // When formatting the duration.
        let result = format_duration(Some(duration));

        // Then it displays as [01:01:01].
        assert_eq!(result, "[01:01:01]");
    }

    #[test]
    fn format_duration_handles_zero() {
        // Given a duration of 0 seconds.
        let duration = Duration::from_secs(0);

        // When formatting the duration.
        let result = format_duration(Some(duration));

        // Then it displays as [00:00:00].
        assert_eq!(result, "[00:00:00]");
    }

    #[test]
    fn format_duration_handles_none() {
        // Given no duration.

        // When formatting None.
        let result = format_duration(None);

        // Then it displays as [--:--:--].
        assert_eq!(result, "[--:--:--]");
    }

    #[rstest::rstest]
    #[case(0, "[00:00:00]")]
    #[case(1, "[00:00:01]")]
    #[case(59, "[00:00:59]")]
    #[case(60, "[00:01:00]")]
    #[case(3599, "[00:59:59]")]
    #[case(3600, "[01:00:00]")]
    #[case(86399, "[23:59:59]")]
    fn format_duration_various_cases(#[case] secs: u64, #[case] expected: &str) {
        // Given / When / Then inline for simple cases
        assert_eq!(format_duration(Some(Duration::from_secs(secs))), expected);
    }

    #[test]
    fn get_display_name_returns_alias_when_set() {
        // Given an item with an alias.
        let dir = TempDir::new().unwrap();
        let path = create_test_file(&dir, "video.mp4");
        let item = item_with_alias(&path, "My Video");

        // When getting the display name.
        let result = get_display_name(&item);

        // Then it returns the alias.
        assert_eq!(result, "My Video");
    }

    #[test]
    fn get_display_name_returns_filename_when_no_alias() {
        // Given an item without an alias.
        let dir = TempDir::new().unwrap();
        let path = create_test_file(&dir, "video.mp4");
        let item = item(&path);

        // When getting the display name.
        let result = get_display_name(&item);

        // Then it returns the file stem.
        assert_eq!(result, "video");
    }

    #[test]
    fn get_display_name_handles_path_without_filename() {
        // Given an item with a directory path.
        let dir = TempDir::new().unwrap();
        let subdir = create_test_dir(&dir, "dir");
        let item = item(&subdir);

        // When getting the display name.
        let result = get_display_name(&item);

        // Then it returns the directory name.
        assert_eq!(result, "dir");
    }

    #[test]
    fn filter_items_returns_all_when_no_filter() {
        // Given a list of items with no filter.
        let dir = TempDir::new().unwrap();
        let a = create_test_file(&dir, "a.mp4");
        let b = create_test_file(&dir, "b.mp4");
        let c = create_test_file(&dir, "c.mp4");
        let items = vec![item(&a), item(&b), item(&c)];

        // When filtering with empty input and no applied filter.
        let result = filter_items(&items, "", None);

        // Then all items are returned with original indices.
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].0, 0);
        assert_eq!(result[1].0, 1);
        assert_eq!(result[2].0, 2);
    }

    #[test]
    fn filter_items_filters_by_pattern() {
        // Given a list of items.
        let dir = TempDir::new().unwrap();
        let apple = create_test_file(&dir, "apple.mp4");
        let banana = create_test_file(&dir, "banana.mp4");
        let cherry = create_test_file(&dir, "cherry.mp4");
        let items = vec![item(&apple), item(&banana), item(&cherry)];

        // When filtering with pattern "an".
        let result = filter_items(&items, "an", None);

        // Then only matching items are returned.
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1.path, ItemPath::File(banana));
    }

    #[test]
    fn filter_items_uses_applied_filter_when_input_empty() {
        // Given a list of items and an applied filter.
        let dir = TempDir::new().unwrap();
        let test = create_test_file(&dir, "test.mp4");
        let other = create_test_file(&dir, "other.mp4");
        let items = vec![item(&test), item(&other)];
        let applied = String::from("test");

        // When filtering with empty input but an applied filter.
        let result = filter_items(&items, "", Some(&applied));

        // Then the applied filter is used.
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1.path, ItemPath::File(test));
    }

    #[test]
    fn filter_items_prefers_input_over_applied() {
        // Given a list of items with both input and applied filter.
        let dir = TempDir::new().unwrap();
        let apple = create_test_file(&dir, "apple.mp4");
        let banana = create_test_file(&dir, "banana.mp4");
        let items = vec![item(&apple), item(&banana)];
        let applied = String::from("apple");

        // When filtering with input that differs from applied filter.
        let result = filter_items(&items, "banana", Some(&applied));

        // Then the input filter takes precedence.
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1.path, ItemPath::File(banana));
    }

    #[test]
    fn filter_items_returns_empty_when_no_match() {
        // Given a list of items.
        let dir = TempDir::new().unwrap();
        let apple = create_test_file(&dir, "apple.mp4");
        let banana = create_test_file(&dir, "banana.mp4");
        let items = vec![item(&apple), item(&banana)];

        // When filtering with a pattern that matches nothing.
        let result = filter_items(&items, "xyz", None);

        // Then an empty list is returned.
        assert!(result.is_empty());
    }

    #[test]
    fn filter_items_searches_alias() {
        // Given items with aliases.
        let dir = TempDir::new().unwrap();
        let a = create_test_file(&dir, "a.mp4");
        let b = create_test_file(&dir, "b.mp4");
        let items = vec![
            item_with_alias(&a, "First Video"),
            item_with_alias(&b, "Second Clip"),
        ];

        // When filtering by alias content.
        let result = filter_items(&items, "Second", None);

        // Then items matching the alias are returned.
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1.alias, Some("Second Clip".to_string()));
    }

    #[test]
    fn filter_items_preserves_original_indices() {
        // Given a list of items.
        let dir = TempDir::new().unwrap();
        let a = create_test_file(&dir, "a.mp4");
        let b = create_test_file(&dir, "b.mp4");
        let c = create_test_file(&dir, "c.mp4");
        let items = vec![item(&a), item(&b), item(&c)];

        // When filtering for the last item.
        let result = filter_items(&items, "c", None);

        // Then the original index is preserved.
        assert_eq!(result[0].0, 2);
    }

    #[test]
    fn format_mime_type_returns_full_mime_for_video() {
        // Given a video mime type.
        let mime = "video/mp4";

        // When formatting the mime type.
        let result = format_mime_type(Some(mime));

        // Then it returns the full mime type.
        assert_eq!(result, "video/mp4");
    }

    #[test]
    fn format_mime_type_returns_full_mime_for_audio() {
        // Given an audio mime type.
        let mime = "audio/mpeg";

        // When formatting the mime type.
        let result = format_mime_type(Some(mime));

        // Then it returns the full mime type.
        assert_eq!(result, "audio/mpeg");
    }

    #[test]
    fn format_mime_type_returns_subtype_for_application() {
        // Given an application mime type.
        let mime = "application/pdf";

        // When formatting the mime type.
        let result = format_mime_type(Some(mime));

        // Then it returns only the subtype.
        assert_eq!(result, "pdf");
    }

    #[test]
    fn format_mime_type_returns_unknown_for_none() {
        // Given no mime type.

        // When formatting None.
        let result = format_mime_type(None);

        // Then it returns "unknown".
        assert_eq!(result, "unknown");
    }

    #[test]
    fn format_item_line_formats_with_duration_and_alias() {
        // Given an item with duration, alias, and mime type.
        let dir = TempDir::new().unwrap();
        let path = create_test_file(&dir, "video.mp4");
        let item = PlaylistItem {
            path: ItemPath::File(path),
            duration: Some(Duration::from_secs(65)),
            alias: Some("My Video".to_string()),
            mime_type: Some("video/mp4".to_string()),
            is_virtual: false,
            playlist_count: 0,
            has_sources: true,
        };

        // When formatting the item line.
        let result = format_item_line(&item, ItemDisplayMode::Path, 0, 0, 2);

        // Then it includes mime type, duration, file stem, and alias.
        assert_eq!(result, "[video/mp4] [00:01:05] video / My Video");
    }

    #[test]
    fn format_item_line_formats_with_duration_no_alias() {
        // Given an item with duration and mime type but no alias.
        let dir = TempDir::new().unwrap();
        let path = create_test_file(&dir, "video.mp4");
        let item = PlaylistItem {
            path: ItemPath::File(path),
            duration: Some(Duration::from_secs(65)),
            alias: None,
            mime_type: Some("video/mp4".to_string()),
            is_virtual: false,
            playlist_count: 0,
            has_sources: true,
        };

        // When formatting the item line.
        let result = format_item_line(&item, ItemDisplayMode::Path, 0, 0, 2);

        // Then it includes mime type, duration, and file stem without alias.
        assert_eq!(result, "[video/mp4] [00:01:05] video");
    }

    #[test]
    fn format_item_line_formats_without_duration_with_alias() {
        // Given an item with alias and mime type but no duration.
        let dir = TempDir::new().unwrap();
        let path = create_test_file(&dir, "doc.pdf");
        let item = PlaylistItem {
            path: ItemPath::File(path),
            duration: None,
            alias: Some("My Doc".to_string()),
            mime_type: Some("application/pdf".to_string()),
            is_virtual: false,
            playlist_count: 0,
            has_sources: true,
        };

        // When formatting the item line.
        let result = format_item_line(&item, ItemDisplayMode::Path, 0, 0, 2);

        // Then it shows placeholder duration and includes alias.
        assert_eq!(result, "[pdf] [--:--:--] doc / My Doc");
    }

    #[test]
    fn format_item_line_formats_without_duration_or_alias() {
        // Given an item with mime type but no duration or alias.
        let dir = TempDir::new().unwrap();
        let path = create_test_file(&dir, "doc.pdf");
        let item = PlaylistItem {
            path: ItemPath::File(path),
            duration: None,
            alias: None,
            mime_type: Some("application/pdf".to_string()),
            is_virtual: false,
            playlist_count: 0,
            has_sources: true,
        };

        // When formatting the item line.
        let result = format_item_line(&item, ItemDisplayMode::Path, 0, 0, 2);

        // Then it shows placeholder duration and file stem only.
        assert_eq!(result, "[pdf] [--:--:--] doc");
    }

    #[test]
    fn format_item_line_uses_unknown_when_no_mime() {
        // Given an item without mime type.
        let dir = TempDir::new().unwrap();
        let path = create_test_file(&dir, "file.xyz");
        let item = PlaylistItem {
            path: ItemPath::File(path),
            duration: None,
            alias: None,
            mime_type: None,
            is_virtual: false,
            playlist_count: 0,
            has_sources: true,
        };

        // When formatting the item line.
        let result = format_item_line(&item, ItemDisplayMode::Path, 0, 0, 2);

        // Then it shows "unknown" for mime type.
        assert_eq!(result, "[unknown] [--:--:--] file");
    }

    #[test]
    fn format_item_line_shows_full_url_for_virtual_items() {
        // Given a virtual item with a URL path.
        let item = PlaylistItem {
            path: ItemPath::Url("https://youtube.com/watch?v=abc123".to_string()),
            duration: None,
            alias: None,
            mime_type: Some("url".to_string()),
            is_virtual: true,
            playlist_count: 0,
            has_sources: true,
        };

        // When formatting the item line.
        let result = format_item_line(&item, ItemDisplayMode::Path, 0, 0, 2);

        // Then it shows the full URL.
        assert_eq!(
            result,
            "[url] [--:--:--] https://youtube.com/watch?v=abc123"
        );
    }

    #[test]
    fn format_item_line_shows_full_url_with_alias() {
        // Given a virtual item with URL and alias.
        let item = PlaylistItem {
            path: ItemPath::Url("https://youtube.com/watch?v=abc123".to_string()),
            duration: None,
            alias: Some("My Video".to_string()),
            mime_type: Some("url".to_string()),
            is_virtual: true,
            playlist_count: 0,
            has_sources: true,
        };

        // When formatting the item line.
        let result = format_item_line(&item, ItemDisplayMode::Path, 0, 0, 2);

        // Then it shows the full URL with alias.
        assert_eq!(
            result,
            "[url] [--:--:--] https://youtube.com/watch?v=abc123 / My Video"
        );
    }

    #[test]
    fn format_item_line_shows_file_stem_for_non_virtual_items() {
        // Given a non-virtual file item.
        let dir = TempDir::new().unwrap();
        let path = create_test_file(&dir, "video.mp4");
        let item = PlaylistItem {
            path: ItemPath::File(path),
            duration: None,
            alias: None,
            mime_type: Some("video/mp4".to_string()),
            is_virtual: false,
            playlist_count: 0,
            has_sources: true,
        };

        // When formatting the item line.
        let result = format_item_line(&item, ItemDisplayMode::Path, 0, 0, 2);

        // Then it shows the file stem.
        assert_eq!(result, "[video/mp4] [--:--:--] video");
    }

    #[test]
    fn format_mime_type_shows_url_without_extra_brackets() {
        // Given a "url" mime type.
        let mime = "url";

        // When formatting the mime type.
        let result = format_mime_type(Some(mime));

        // Then it returns "url" without extra formatting.
        assert_eq!(result, "url");
    }

    #[test]
    fn format_mime_type_shows_deleted_without_extra_brackets() {
        // Given a "deleted" mime type.
        let mime = "deleted";

        // When formatting the mime type.
        let result = format_mime_type(Some(mime));

        // Then it returns "deleted" without extra formatting.
        assert_eq!(result, "deleted");
    }

    #[test]
    fn format_item_line_no_count_shown_when_count_is_zero() {
        // Given an item with playlist_count of 0.
        let dir = TempDir::new().unwrap();
        let path = create_test_file(&dir, "video.mp4");
        let item = PlaylistItem {
            path: ItemPath::File(path),
            duration: Some(Duration::from_secs(65)),
            alias: Some("My Video".to_string()),
            mime_type: Some("video/mp4".to_string()),
            is_virtual: false,
            playlist_count: 0,
            has_sources: true,
        };

        // When formatting the item line with count 0.
        let result = format_item_line(&item, ItemDisplayMode::Alias, 80, 0, 2);

        // Then no count is shown.
        assert_eq!(result, "[video/mp4] [00:01:05] My Video");
    }

    #[test]
    fn format_item_line_no_count_shown_when_count_is_one() {
        // Given an item with playlist_count of 1.
        let dir = TempDir::new().unwrap();
        let path = create_test_file(&dir, "video.mp4");
        let item = PlaylistItem {
            path: ItemPath::File(path),
            duration: Some(Duration::from_secs(65)),
            alias: Some("My Video".to_string()),
            mime_type: Some("video/mp4".to_string()),
            is_virtual: false,
            playlist_count: 1,
            has_sources: true,
        };

        // When formatting the item line with count 1 and min_count 2.
        let result = format_item_line(&item, ItemDisplayMode::Alias, 80, 1, 2);

        // Then no count is shown (below threshold).
        assert_eq!(result, "[video/mp4] [00:01:05] My Video");
    }

    #[test]
    fn format_item_line_shows_count_when_count_is_two() {
        // Given an item with playlist_count of 2.
        let dir = TempDir::new().unwrap();
        let path = create_test_file(&dir, "video.mp4");
        let item = PlaylistItem {
            path: ItemPath::File(path),
            duration: Some(Duration::from_secs(65)),
            alias: Some("My Video".to_string()),
            mime_type: Some("video/mp4".to_string()),
            is_virtual: false,
            playlist_count: 2,
            has_sources: true,
        };

        // When formatting the item line with count 2 and min_count 2.
        let result = format_item_line(&item, ItemDisplayMode::Alias, 50, 2, 2);

        // Then the count is shown.
        assert_eq!(result, "[video/mp4] [00:01:05] My Video                (2)");
    }

    #[test]
    fn format_item_line_shows_count_when_count_is_three() {
        // Given an item with playlist_count of 3.
        let dir = TempDir::new().unwrap();
        let path = create_test_file(&dir, "video.mp4");
        let item = PlaylistItem {
            path: ItemPath::File(path),
            duration: Some(Duration::from_secs(65)),
            alias: Some("My Video".to_string()),
            mime_type: Some("video/mp4".to_string()),
            is_virtual: false,
            playlist_count: 3,
            has_sources: true,
        };

        // When formatting the item line with count 3 and min_count 2.
        let result = format_item_line(&item, ItemDisplayMode::Alias, 50, 3, 2);

        // Then the count is shown.
        assert_eq!(result, "[video/mp4] [00:01:05] My Video                (3)");
    }

    #[test]
    fn format_item_line_no_count_when_pane_width_is_zero() {
        // Given an item with playlist_count of 5.
        let dir = TempDir::new().unwrap();
        let path = create_test_file(&dir, "video.mp4");
        let item = PlaylistItem {
            path: ItemPath::File(path),
            duration: Some(Duration::from_secs(65)),
            alias: Some("My Video".to_string()),
            mime_type: Some("video/mp4".to_string()),
            is_virtual: false,
            playlist_count: 5,
            has_sources: true,
        };

        // When formatting with pane width 0.
        let result = format_item_line(&item, ItemDisplayMode::Alias, 0, 5, 2);

        // Then no count is shown.
        assert_eq!(result, "[video/mp4] [00:01:05] My Video");
    }

    #[test]
    fn format_item_line_no_count_when_line_too_long() {
        // Given an item with playlist_count of 2.
        let dir = TempDir::new().unwrap();
        let path = create_test_file(&dir, "video.mp4");
        let item = PlaylistItem {
            path: ItemPath::File(path),
            duration: Some(Duration::from_secs(65)),
            alias: Some("My Video".to_string()),
            mime_type: Some("video/mp4".to_string()),
            is_virtual: false,
            playlist_count: 2,
            has_sources: true,
        };

        // When formatting with a narrow pane width.
        let result = format_item_line(&item, ItemDisplayMode::Alias, 20, 2, 2);

        // Then no count is shown (not enough room).
        assert_eq!(result, "[video/mp4] [00:01:05] My Video");
    }

    #[test]
    fn format_item_line_shows_count_when_min_count_is_one_and_count_is_one() {
        // Given an item with playlist_count of 1.
        let dir = TempDir::new().unwrap();
        let path = create_test_file(&dir, "video.mp4");
        let item = PlaylistItem {
            path: ItemPath::File(path),
            duration: Some(Duration::from_secs(65)),
            alias: Some("My Video".to_string()),
            mime_type: Some("video/mp4".to_string()),
            is_virtual: false,
            playlist_count: 1,
            has_sources: true,
        };

        // When formatting with min_count 1.
        let result = format_item_line(&item, ItemDisplayMode::Alias, 50, 1, 1);

        // Then the count is shown (meets threshold).
        assert_eq!(result, "[video/mp4] [00:01:05] My Video                (1)");
    }

    #[test]
    fn split_pane_layout_splits_into_list_and_footer() {
        // Given a pane area of 10 rows by 40 columns.
        let area = Rect::new(0, 0, 40, 10);

        // When splitting the layout.
        let (list_area, footer_area) = split_pane_layout(area);

        // Then list gets min 1 row and footer gets 1 row.
        assert_eq!(list_area.height, 9);
        assert_eq!(footer_area.height, 1);
        assert_eq!(list_area.width, 40);
        assert_eq!(footer_area.width, 40);
    }

    #[test]
    fn item_style_selected_missing_file_returns_red_on_yellow_bold() {
        // Given selected item with missing file.
        let style = item_style(true, true, true);

        // Then style is red on yellow with bold.
        assert_eq!(style.fg, Some(Color::Red));
        assert_eq!(style.bg, Some(Color::Yellow));
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn item_style_selected_normal_file_returns_black_on_yellow_bold() {
        // Given selected item with file present.
        let style = item_style(true, false, true);

        // Then style is black on yellow with bold.
        assert_eq!(style.fg, Some(Color::Black));
        assert_eq!(style.bg, Some(Color::Yellow));
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn item_style_unselected_no_sources_returns_yellow() {
        // Given unselected item without sources.
        let style = item_style(false, false, false);

        // Then style is yellow foreground.
        assert_eq!(style.fg, Some(Color::Yellow));
        assert_eq!(style.bg, None);
    }

    #[test]
    fn item_style_unselected_normal_returns_default() {
        // Given unselected item with sources.
        let style = item_style(false, false, true);

        // Then style is default.
        assert_eq!(style.fg, None);
        assert_eq!(style.bg, None);
    }

    #[test]
    fn focused_border_style_focused_returns_cyan() {
        // Given a focused pane.
        let style = focused_border_style(true);

        // Then style is cyan.
        assert_eq!(style.fg, Some(Color::Cyan));
    }

    #[test]
    fn focused_border_style_unfocused_returns_default() {
        // Given an unfocused pane.
        let style = focused_border_style(false);

        // Then style is default.
        assert_eq!(style.fg, None);
    }

    #[test]
    fn focused_text_style_focused_returns_cyan() {
        // Given a focused pane.
        let style = focused_text_style(true);

        // Then style is cyan.
        assert_eq!(style.fg, Some(Color::Cyan));
    }

    #[test]
    fn focused_text_style_unfocused_returns_default() {
        // Given an unfocused pane.
        let style = focused_text_style(false);

        // Then style is default.
        assert_eq!(style.fg, None);
    }

    #[test]
    fn pane_title_no_filter_no_focus_returns_base() {
        // Given base title with no filter and not focused.
        let title = pane_title("Playlist", false, false);

        // Then title is just the base with spaces.
        assert_eq!(title, " Playlist ");
    }

    #[test]
    fn pane_title_no_filter_with_focus_returns_base_with_asterisk() {
        // Given base title with no filter but focused.
        let title = pane_title("Playlist", false, true);

        // Then title includes focus indicator.
        assert_eq!(title, " Playlist [*] ");
    }

    #[test]
    fn pane_title_with_filter_no_focus_returns_base_with_filtered() {
        // Given base title with filter but not focused.
        let title = pane_title("Playlist", true, false);

        // Then title includes filter indicator.
        assert_eq!(title, " Playlist [filtered] ");
    }

    #[test]
    fn pane_title_with_filter_and_focus_returns_base_with_both_indicators() {
        // Given base title with filter and focused.
        let title = pane_title("Playlist", true, true);

        // Then title includes both indicators.
        assert_eq!(title, " Playlist [filtered] [*] ");
    }

    #[test]
    fn total_duration_sums_item_durations() {
        // Given items with durations.
        let dir = TempDir::new().unwrap();
        let a = create_test_file(&dir, "a.mp4");
        let b = create_test_file(&dir, "b.mp4");
        let items = [
            PlaylistItem {
                path: ItemPath::File(a),
                duration: Some(Duration::from_secs(60)),
                alias: None,
                mime_type: None,
                is_virtual: false,
                playlist_count: 0,
                has_sources: true,
            },
            PlaylistItem {
                path: ItemPath::File(b),
                duration: Some(Duration::from_secs(120)),
                alias: None,
                mime_type: None,
                is_virtual: false,
                playlist_count: 0,
                has_sources: true,
            },
        ];

        // When calculating total duration.
        let total = total_duration(items.iter());

        // Then it sums to 180 seconds.
        assert_eq!(total, Duration::from_secs(180));
    }

    #[test]
    fn total_duration_ignores_items_without_duration() {
        // Given items with mixed durations.
        let dir = TempDir::new().unwrap();
        let a = create_test_file(&dir, "a.mp4");
        let b = create_test_file(&dir, "b.mp4");
        let c = create_test_file(&dir, "c.mp4");
        let items = [
            PlaylistItem {
                path: ItemPath::File(a),
                duration: Some(Duration::from_secs(60)),
                alias: None,
                mime_type: None,
                is_virtual: false,
                playlist_count: 0,
                has_sources: true,
            },
            PlaylistItem {
                path: ItemPath::File(b),
                duration: None,
                alias: None,
                mime_type: None,
                is_virtual: false,
                playlist_count: 0,
                has_sources: true,
            },
            PlaylistItem {
                path: ItemPath::File(c),
                duration: Some(Duration::from_secs(30)),
                alias: None,
                mime_type: None,
                is_virtual: false,
                playlist_count: 0,
                has_sources: true,
            },
        ];

        // When calculating total duration.
        let total = total_duration(items.iter());

        // Then it only sums items with durations (90 seconds).
        assert_eq!(total, Duration::from_secs(90));
    }
}
