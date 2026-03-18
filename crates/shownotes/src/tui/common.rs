use std::time::Duration;

pub use crate::common::domain::{get_mime_type, ItemPath, PlaylistItem};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

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

#[cfg(test)]
mod tests {
    use super::*;
    use marked_path::CanonicalPath;
    use std::path::PathBuf;

    fn item(path: &str) -> PlaylistItem {
        let item_path = if path.starts_with("http://") || path.starts_with("https://") {
            ItemPath::Url(path.to_string())
        } else {
            ItemPath::File(CanonicalPath::new(PathBuf::from(path)))
        };
        PlaylistItem {
            path: item_path,
            duration: None,
            alias: None,
            mime_type: None,
            is_virtual: false,
            playlist_count: 0,
            has_sources: true,
        }
    }

    fn item_with_alias(path: &str, alias: &str) -> PlaylistItem {
        let item_path = if path.starts_with("http://") || path.starts_with("https://") {
            ItemPath::Url(path.to_string())
        } else {
            ItemPath::File(CanonicalPath::new(PathBuf::from(path)))
        };
        PlaylistItem {
            path: item_path,
            duration: None,
            alias: Some(alias.to_string()),
            mime_type: None,
            is_virtual: false,
            playlist_count: 0,
            has_sources: true,
        }
    }

    #[allow(dead_code)]
    fn item_with_duration(path: &str, secs: u64) -> PlaylistItem {
        let item_path = if path.starts_with("http://") || path.starts_with("https://") {
            ItemPath::Url(path.to_string())
        } else {
            ItemPath::File(CanonicalPath::new(PathBuf::from(path)))
        };
        PlaylistItem {
            path: item_path,
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
        let item = item_with_alias("/path/to/video.mp4", "My Video");

        // When getting the display name.
        let result = get_display_name(&item);

        // Then it returns the alias.
        assert_eq!(result, "My Video");
    }

    #[test]
    fn get_display_name_returns_filename_when_no_alias() {
        // Given an item without an alias.
        let item = item("/path/to/video.mp4");

        // When getting the display name.
        let result = get_display_name(&item);

        // Then it returns the file stem.
        assert_eq!(result, "video");
    }

    #[test]
    fn get_display_name_handles_path_without_filename() {
        // Given an item with a directory path.
        let item = item("/path/to/dir/");

        // When getting the display name.
        let result = get_display_name(&item);

        // Then it returns the directory name.
        assert_eq!(result, "dir");
    }

    #[test]
    fn filter_items_returns_all_when_no_filter() {
        // Given a list of items with no filter.
        let items = vec![item("a.mp4"), item("b.mp4"), item("c.mp4")];

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
        let items = vec![item("apple.mp4"), item("banana.mp4"), item("cherry.mp4")];

        // When filtering with pattern "an".
        let result = filter_items(&items, "an", None);

        // Then only matching items are returned.
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].1.path,
            ItemPath::File(CanonicalPath::new(PathBuf::from("banana.mp4")))
        );
    }

    #[test]
    fn filter_items_uses_applied_filter_when_input_empty() {
        // Given a list of items and an applied filter.
        let items = vec![item("test.mp4"), item("other.mp4")];
        let applied = String::from("test");

        // When filtering with empty input but an applied filter.
        let result = filter_items(&items, "", Some(&applied));

        // Then the applied filter is used.
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].1.path,
            ItemPath::File(CanonicalPath::new(PathBuf::from("test.mp4")))
        );
    }

    #[test]
    fn filter_items_prefers_input_over_applied() {
        // Given a list of items with both input and applied filter.
        let items = vec![item("apple.mp4"), item("banana.mp4")];
        let applied = String::from("apple");

        // When filtering with input that differs from applied filter.
        let result = filter_items(&items, "banana", Some(&applied));

        // Then the input filter takes precedence.
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].1.path,
            ItemPath::File(CanonicalPath::new(PathBuf::from("banana.mp4")))
        );
    }

    #[test]
    fn filter_items_returns_empty_when_no_match() {
        // Given a list of items.
        let items = vec![item("apple.mp4"), item("banana.mp4")];

        // When filtering with a pattern that matches nothing.
        let result = filter_items(&items, "xyz", None);

        // Then an empty list is returned.
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

        // Then items matching the alias are returned.
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1.alias, Some("Second Clip".to_string()));
    }

    #[test]
    fn filter_items_preserves_original_indices() {
        // Given a list of items.
        let items = vec![item("a.mp4"), item("b.mp4"), item("c.mp4")];

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
        let item = PlaylistItem {
            path: ItemPath::File(CanonicalPath::new(PathBuf::from("/path/to/video.mp4"))),
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
        let item = PlaylistItem {
            path: ItemPath::File(CanonicalPath::new(PathBuf::from("/path/to/video.mp4"))),
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
        let item = PlaylistItem {
            path: ItemPath::File(CanonicalPath::new(PathBuf::from("/path/to/doc.pdf"))),
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
        let item = PlaylistItem {
            path: ItemPath::File(CanonicalPath::new(PathBuf::from("/path/to/doc.pdf"))),
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
        let item = PlaylistItem {
            path: ItemPath::File(CanonicalPath::new(PathBuf::from("/path/to/file.xyz"))),
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
        let item = PlaylistItem {
            path: ItemPath::File(CanonicalPath::new(PathBuf::from("/path/to/video.mp4"))),
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
        let item = PlaylistItem {
            path: ItemPath::File(CanonicalPath::new(PathBuf::from("/path/to/video.mp4"))),
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
        let item = PlaylistItem {
            path: ItemPath::File(CanonicalPath::new(PathBuf::from("/path/to/video.mp4"))),
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
        let item = PlaylistItem {
            path: ItemPath::File(CanonicalPath::new(PathBuf::from("/path/to/video.mp4"))),
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
        let item = PlaylistItem {
            path: ItemPath::File(CanonicalPath::new(PathBuf::from("/path/to/video.mp4"))),
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
        let item = PlaylistItem {
            path: ItemPath::File(CanonicalPath::new(PathBuf::from("/path/to/video.mp4"))),
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
        let item = PlaylistItem {
            path: ItemPath::File(CanonicalPath::new(PathBuf::from("/path/to/video.mp4"))),
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
        let item = PlaylistItem {
            path: ItemPath::File(CanonicalPath::new(PathBuf::from("/path/to/video.mp4"))),
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
}
