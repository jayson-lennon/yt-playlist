use std::{path::PathBuf, time::Duration};

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

/// Display mode for showing item names in the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ItemDisplayMode {
    /// Show filename as primary (current behavior).
    Path,
    /// Show alias as primary when available.
    #[default]
    Alias,
}

/// Which pane is currently focused in the TUI.
///
/// The focused pane receives keyboard input for navigation and actions.
/// Some keybindings are context-sensitive and only apply to specific panes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Playlist,
    Library,
}

/// An item in the playlist or library.
///
/// Represents a media file or virtual URL with associated metadata including
/// duration, optional alias for display, MIME type, and whether it's a virtual
/// item (URL) that doesn't exist on the local filesystem.
#[derive(Debug, Clone)]
pub struct PlaylistItem {
    pub path: PathBuf,
    pub duration: Option<Duration>,
    pub alias: Option<String>,
    pub mime_type: Option<String>,
    pub is_virtual: bool,
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
            |n| n.to_string_lossy().into_owned(),
        )
    })
}

pub fn get_mime_type(path: &PathBuf) -> Option<String> {
    infer::get_from_path(path)
        .ok()
        .flatten()
        .map(|t| t.mime_type().to_string())
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

pub fn format_item_line(item: &PlaylistItem, display_mode: ItemDisplayMode) -> String {
    let mime_str = format_mime_type(item.mime_type.as_deref());
    let duration_str = format_duration(item.duration);

    let filename = if item.is_virtual {
        item.path.to_string_lossy().into_owned()
    } else {
        item.path.file_stem().map_or_else(
            || item.path.to_string_lossy().into_owned(),
            |n| n.to_string_lossy().into_owned(),
        )
    };

    let primary = match display_mode {
        ItemDisplayMode::Alias => item.alias.clone().unwrap_or(filename),
        ItemDisplayMode::Path => filename,
    };

    match display_mode {
        ItemDisplayMode::Alias => format!("[{mime_str}] {duration_str} {primary}"),
        ItemDisplayMode::Path => match &item.alias {
            Some(alias) => format!("[{mime_str}] {duration_str} {primary} / {alias}"),
            None => format!("[{mime_str}] {duration_str} {primary}"),
        },
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn item(path: &str) -> PlaylistItem {
        PlaylistItem {
            path: PathBuf::from(path),
            duration: None,
            alias: None,
            mime_type: None,
            is_virtual: false,
        }
    }

    fn item_with_alias(path: &str, alias: &str) -> PlaylistItem {
        PlaylistItem {
            path: PathBuf::from(path),
            duration: None,
            alias: Some(alias.to_string()),
            mime_type: None,
            is_virtual: false,
        }
    }

    #[allow(dead_code)]
    fn item_with_duration(path: &str, secs: u64) -> PlaylistItem {
        PlaylistItem {
            path: PathBuf::from(path),
            duration: Some(Duration::from_secs(secs)),
            alias: None,
            mime_type: None,
            is_virtual: false,
        }
    }

    #[test]
    fn format_duration_formats_seconds() {
        // Given a duration of 65 seconds.
        let duration = Duration::from_secs(65);

        // When formatting.
        let result = format_duration(Some(duration));

        // Then it shows hours:minutes:seconds.
        assert_eq!(result, "[00:01:05]");
    }

    #[test]
    fn format_duration_formats_hours() {
        // Given a duration of 3661 seconds (1 hour, 1 minute, 1 second).
        let duration = Duration::from_secs(3661);

        // When formatting.
        let result = format_duration(Some(duration));

        // Then it shows hours:minutes:seconds.
        assert_eq!(result, "[01:01:01]");
    }

    #[test]
    fn format_duration_handles_zero() {
        // Given a zero duration.
        let duration = Duration::from_secs(0);

        // When formatting.
        let result = format_duration(Some(duration));

        // Then it shows zeros.
        assert_eq!(result, "[00:00:00]");
    }

    #[test]
    fn format_duration_handles_none() {
        // Given no duration.
        // When formatting.
        let result = format_duration(None);

        // Then it shows placeholders.
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
        assert_eq!(format_duration(Some(Duration::from_secs(secs))), expected);
    }

    #[test]
    fn get_display_name_returns_alias_when_set() {
        // Given an item with an alias.
        let item = item_with_alias("/path/to/video.mp4", "My Video");

        // When getting display name.
        let result = get_display_name(&item);

        // Then alias is returned.
        assert_eq!(result, "My Video");
    }

    #[test]
    fn get_display_name_returns_filename_when_no_alias() {
        // Given an item without alias.
        let item = item("/path/to/video.mp4");

        // When getting display name.
        let result = get_display_name(&item);

        // Then filename without extension is returned.
        assert_eq!(result, "video");
    }

    #[test]
    fn get_display_name_handles_path_without_filename() {
        // Given an item with a directory path.
        let item = item("/path/to/dir/");

        // When getting display name.
        let result = get_display_name(&item);

        // Then the last component is returned.
        assert_eq!(result, "dir");
    }

    #[test]
    fn filter_items_returns_all_when_no_filter() {
        // Given items without filter.
        let items = vec![item("a.mp4"), item("b.mp4"), item("c.mp4")];

        // When filtering with no pattern.
        let result = filter_items(&items, "", None);

        // Then all items are returned in order.
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].0, 0);
        assert_eq!(result[1].0, 1);
        assert_eq!(result[2].0, 2);
    }

    #[test]
    fn filter_items_filters_by_pattern() {
        // Given items with various names.
        let items = vec![item("apple.mp4"), item("banana.mp4"), item("cherry.mp4")];

        // When filtering for "an".
        let result = filter_items(&items, "an", None);

        // Then only matching items are returned.
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1.path, PathBuf::from("banana.mp4"));
    }

    #[test]
    fn filter_items_uses_applied_filter_when_input_empty() {
        // Given items and an applied filter.
        let items = vec![item("test.mp4"), item("other.mp4")];
        let applied = String::from("test");

        // When filtering with empty input but applied filter.
        let result = filter_items(&items, "", Some(&applied));

        // Then applied filter is used.
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1.path, PathBuf::from("test.mp4"));
    }

    #[test]
    fn filter_items_prefers_input_over_applied() {
        // Given items with both input and applied filter.
        let items = vec![item("apple.mp4"), item("banana.mp4")];
        let applied = String::from("apple");

        // When filtering with input (overrides applied).
        let result = filter_items(&items, "banana", Some(&applied));

        // Then input filter is used.
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1.path, PathBuf::from("banana.mp4"));
    }

    #[test]
    fn filter_items_returns_empty_when_no_match() {
        // Given items.
        let items = vec![item("apple.mp4"), item("banana.mp4")];

        // When filtering with non-matching pattern.
        let result = filter_items(&items, "xyz", None);

        // Then empty result.
        assert!(result.is_empty());
    }

    #[test]
    fn filter_items_searches_alias() {
        // Given items with aliases.
        let items = vec![
            item_with_alias("a.mp4", "First Video"),
            item_with_alias("b.mp4", "Second Clip"),
        ];

        // When filtering by alias content.
        let result = filter_items(&items, "Second", None);

        // Then matching item is found.
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1.alias, Some("Second Clip".to_string()));
    }

    #[test]
    fn filter_items_preserves_original_indices() {
        // Given items.
        let items = vec![item("a.mp4"), item("b.mp4"), item("c.mp4")];

        // When filtering for "c".
        let result = filter_items(&items, "c", None);

        // Then original index is preserved.
        assert_eq!(result[0].0, 2);
    }

    #[test]
    fn format_mime_type_returns_full_mime_for_video() {
        // Given a video mime type.
        let mime = "video/mp4";

        // When formatting.
        let result = format_mime_type(Some(mime));

        // Then full mime type is returned.
        assert_eq!(result, "video/mp4");
    }

    #[test]
    fn format_mime_type_returns_full_mime_for_audio() {
        // Given an audio mime type.
        let mime = "audio/mpeg";

        // When formatting.
        let result = format_mime_type(Some(mime));

        // Then full mime type is returned.
        assert_eq!(result, "audio/mpeg");
    }

    #[test]
    fn format_mime_type_returns_subtype_for_application() {
        // Given an application mime type.
        let mime = "application/pdf";

        // When formatting.
        let result = format_mime_type(Some(mime));

        // Then only subtype is returned.
        assert_eq!(result, "pdf");
    }

    #[test]
    fn format_mime_type_returns_unknown_for_none() {
        // Given no mime type.
        // When formatting.
        let result = format_mime_type(None);

        // Then unknown is returned.
        assert_eq!(result, "unknown");
    }

    #[test]
    fn format_item_line_formats_with_duration_and_alias() {
        // Given an item with duration and alias.
        let item = PlaylistItem {
            path: PathBuf::from("/path/to/video.mp4"),
            duration: Some(Duration::from_secs(65)),
            alias: Some("My Video".to_string()),
            mime_type: Some("video/mp4".to_string()),
            is_virtual: false,
        };

        // When formatting in Path mode.
        let result = format_item_line(&item, ItemDisplayMode::Path);

        // Then line is formatted correctly.
        assert_eq!(result, "[video/mp4] [00:01:05] video / My Video");
    }

    #[test]
    fn format_item_line_formats_with_duration_no_alias() {
        // Given an item with duration but no alias.
        let item = PlaylistItem {
            path: PathBuf::from("/path/to/video.mp4"),
            duration: Some(Duration::from_secs(65)),
            alias: None,
            mime_type: Some("video/mp4".to_string()),
            is_virtual: false,
        };

        // When formatting in Path mode.
        let result = format_item_line(&item, ItemDisplayMode::Path);

        // Then line is formatted correctly.
        assert_eq!(result, "[video/mp4] [00:01:05] video");
    }

    #[test]
    fn format_item_line_formats_without_duration_with_alias() {
        // Given an item without duration but with alias.
        let item = PlaylistItem {
            path: PathBuf::from("/path/to/doc.pdf"),
            duration: None,
            alias: Some("My Doc".to_string()),
            mime_type: Some("application/pdf".to_string()),
            is_virtual: false,
        };

        // When formatting in Path mode.
        let result = format_item_line(&item, ItemDisplayMode::Path);

        // Then line is formatted correctly.
        assert_eq!(result, "[pdf] [--:--:--] doc / My Doc");
    }

    #[test]
    fn format_item_line_formats_without_duration_or_alias() {
        // Given an item without duration or alias.
        let item = PlaylistItem {
            path: PathBuf::from("/path/to/doc.pdf"),
            duration: None,
            alias: None,
            mime_type: Some("application/pdf".to_string()),
            is_virtual: false,
        };

        // When formatting in Path mode.
        let result = format_item_line(&item, ItemDisplayMode::Path);

        // Then line is formatted correctly.
        assert_eq!(result, "[pdf] [--:--:--] doc");
    }

    #[test]
    fn format_item_line_uses_unknown_when_no_mime() {
        // Given an item without mime type.
        let item = PlaylistItem {
            path: PathBuf::from("/path/to/file.xyz"),
            duration: None,
            alias: None,
            mime_type: None,
            is_virtual: false,
        };

        // When formatting in Path mode.
        let result = format_item_line(&item, ItemDisplayMode::Path);

        // Then unknown is used.
        assert_eq!(result, "[unknown] [--:--:--] file");
    }

    #[test]
    fn format_item_line_shows_full_url_for_virtual_items() {
        // Given a virtual URL item.
        let item = PlaylistItem {
            path: PathBuf::from("https://youtube.com/watch?v=abc123"),
            duration: None,
            alias: None,
            mime_type: Some("url".to_string()),
            is_virtual: true,
        };

        // When formatting in Path mode.
        let result = format_item_line(&item, ItemDisplayMode::Path);

        // Then full URL is shown (not just filename stem).
        assert_eq!(
            result,
            "[url] [--:--:--] https://youtube.com/watch?v=abc123"
        );
    }

    #[test]
    fn format_item_line_shows_full_url_with_alias() {
        // Given a virtual URL item with alias.
        let item = PlaylistItem {
            path: PathBuf::from("https://youtube.com/watch?v=abc123"),
            duration: None,
            alias: Some("My Video".to_string()),
            mime_type: Some("url".to_string()),
            is_virtual: true,
        };

        // When formatting in Path mode.
        let result = format_item_line(&item, ItemDisplayMode::Path);

        // Then full URL and alias are shown.
        assert_eq!(
            result,
            "[url] [--:--:--] https://youtube.com/watch?v=abc123 / My Video"
        );
    }

    #[test]
    fn format_item_line_shows_file_stem_for_non_virtual_items() {
        // Given a non-virtual file item.
        let item = PlaylistItem {
            path: PathBuf::from("/path/to/video.mp4"),
            duration: None,
            alias: None,
            mime_type: Some("video/mp4".to_string()),
            is_virtual: false,
        };

        // When formatting in Path mode.
        let result = format_item_line(&item, ItemDisplayMode::Path);

        // Then only filename stem is shown (not full path or extension).
        assert_eq!(result, "[video/mp4] [--:--:--] video");
    }

    #[test]
    fn format_mime_type_shows_url_without_extra_brackets() {
        // Given a URL mime type (without brackets).
        let mime = "url";

        // When formatting.
        let result = format_mime_type(Some(mime));

        // Then url is returned as-is.
        assert_eq!(result, "url");
    }

    #[test]
    fn format_mime_type_shows_deleted_without_extra_brackets() {
        // Given a deleted mime type (without brackets).
        let mime = "deleted";

        // When formatting.
        let result = format_mime_type(Some(mime));

        // Then deleted is returned as-is.
        assert_eq!(result, "deleted");
    }
}
