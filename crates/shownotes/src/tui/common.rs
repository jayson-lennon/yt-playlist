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
        let duration = Duration::from_secs(65);
        let result = format_duration(Some(duration));
        assert_eq!(result, "[00:01:05]");
    }

    #[test]
    fn format_duration_formats_hours() {
        let duration = Duration::from_secs(3661);
        let result = format_duration(Some(duration));
        assert_eq!(result, "[01:01:01]");
    }

    #[test]
    fn format_duration_handles_zero() {
        let duration = Duration::from_secs(0);
        let result = format_duration(Some(duration));
        assert_eq!(result, "[00:00:00]");
    }

    #[test]
    fn format_duration_handles_none() {
        let result = format_duration(None);
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
        let item = item_with_alias("/path/to/video.mp4", "My Video");
        let result = get_display_name(&item);
        assert_eq!(result, "My Video");
    }

    #[test]
    fn get_display_name_returns_filename_when_no_alias() {
        let item = item("/path/to/video.mp4");
        let result = get_display_name(&item);
        assert_eq!(result, "video");
    }

    #[test]
    fn get_display_name_handles_path_without_filename() {
        let item = item("/path/to/dir/");
        let result = get_display_name(&item);
        assert_eq!(result, "dir");
    }

    #[test]
    fn filter_items_returns_all_when_no_filter() {
        let items = vec![item("a.mp4"), item("b.mp4"), item("c.mp4")];
        let result = filter_items(&items, "", None);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].0, 0);
        assert_eq!(result[1].0, 1);
        assert_eq!(result[2].0, 2);
    }

    #[test]
    fn filter_items_filters_by_pattern() {
        let items = vec![item("apple.mp4"), item("banana.mp4"), item("cherry.mp4")];
        let result = filter_items(&items, "an", None);
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].1.path,
            ItemPath::File(CanonicalPath::new(PathBuf::from("banana.mp4")))
        );
    }

    #[test]
    fn filter_items_uses_applied_filter_when_input_empty() {
        let items = vec![item("test.mp4"), item("other.mp4")];
        let applied = String::from("test");
        let result = filter_items(&items, "", Some(&applied));
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].1.path,
            ItemPath::File(CanonicalPath::new(PathBuf::from("test.mp4")))
        );
    }

    #[test]
    fn filter_items_prefers_input_over_applied() {
        let items = vec![item("apple.mp4"), item("banana.mp4")];
        let applied = String::from("apple");
        let result = filter_items(&items, "banana", Some(&applied));
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].1.path,
            ItemPath::File(CanonicalPath::new(PathBuf::from("banana.mp4")))
        );
    }

    #[test]
    fn filter_items_returns_empty_when_no_match() {
        let items = vec![item("apple.mp4"), item("banana.mp4")];
        let result = filter_items(&items, "xyz", None);
        assert!(result.is_empty());
    }

    #[test]
    fn filter_items_searches_alias() {
        let items = vec![
            item_with_alias("a.mp4", "First Video"),
            item_with_alias("b.mp4", "Second Clip"),
        ];
        let result = filter_items(&items, "Second", None);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1.alias, Some("Second Clip".to_string()));
    }

    #[test]
    fn filter_items_preserves_original_indices() {
        let items = vec![item("a.mp4"), item("b.mp4"), item("c.mp4")];
        let result = filter_items(&items, "c", None);
        assert_eq!(result[0].0, 2);
    }

    #[test]
    fn format_mime_type_returns_full_mime_for_video() {
        let mime = "video/mp4";
        let result = format_mime_type(Some(mime));
        assert_eq!(result, "video/mp4");
    }

    #[test]
    fn format_mime_type_returns_full_mime_for_audio() {
        let mime = "audio/mpeg";
        let result = format_mime_type(Some(mime));
        assert_eq!(result, "audio/mpeg");
    }

    #[test]
    fn format_mime_type_returns_subtype_for_application() {
        let mime = "application/pdf";
        let result = format_mime_type(Some(mime));
        assert_eq!(result, "pdf");
    }

    #[test]
    fn format_mime_type_returns_unknown_for_none() {
        let result = format_mime_type(None);
        assert_eq!(result, "unknown");
    }

    #[test]
    fn format_item_line_formats_with_duration_and_alias() {
        let item = PlaylistItem {
            path: ItemPath::File(CanonicalPath::new(PathBuf::from("/path/to/video.mp4"))),
            duration: Some(Duration::from_secs(65)),
            alias: Some("My Video".to_string()),
            mime_type: Some("video/mp4".to_string()),
            is_virtual: false,
            playlist_count: 0,
            has_sources: true,
        };
        let result = format_item_line(&item, ItemDisplayMode::Path, 0, 0, 2);
        assert_eq!(result, "[video/mp4] [00:01:05] video / My Video");
    }

    #[test]
    fn format_item_line_formats_with_duration_no_alias() {
        let item = PlaylistItem {
            path: ItemPath::File(CanonicalPath::new(PathBuf::from("/path/to/video.mp4"))),
            duration: Some(Duration::from_secs(65)),
            alias: None,
            mime_type: Some("video/mp4".to_string()),
            is_virtual: false,
            playlist_count: 0,
            has_sources: true,
        };
        let result = format_item_line(&item, ItemDisplayMode::Path, 0, 0, 2);
        assert_eq!(result, "[video/mp4] [00:01:05] video");
    }

    #[test]
    fn format_item_line_formats_without_duration_with_alias() {
        let item = PlaylistItem {
            path: ItemPath::File(CanonicalPath::new(PathBuf::from("/path/to/doc.pdf"))),
            duration: None,
            alias: Some("My Doc".to_string()),
            mime_type: Some("application/pdf".to_string()),
            is_virtual: false,
            playlist_count: 0,
            has_sources: true,
        };
        let result = format_item_line(&item, ItemDisplayMode::Path, 0, 0, 2);
        assert_eq!(result, "[pdf] [--:--:--] doc / My Doc");
    }

    #[test]
    fn format_item_line_formats_without_duration_or_alias() {
        let item = PlaylistItem {
            path: ItemPath::File(CanonicalPath::new(PathBuf::from("/path/to/doc.pdf"))),
            duration: None,
            alias: None,
            mime_type: Some("application/pdf".to_string()),
            is_virtual: false,
            playlist_count: 0,
            has_sources: true,
        };
        let result = format_item_line(&item, ItemDisplayMode::Path, 0, 0, 2);
        assert_eq!(result, "[pdf] [--:--:--] doc");
    }

    #[test]
    fn format_item_line_uses_unknown_when_no_mime() {
        let item = PlaylistItem {
            path: ItemPath::File(CanonicalPath::new(PathBuf::from("/path/to/file.xyz"))),
            duration: None,
            alias: None,
            mime_type: None,
            is_virtual: false,
            playlist_count: 0,
            has_sources: true,
        };
        let result = format_item_line(&item, ItemDisplayMode::Path, 0, 0, 2);
        assert_eq!(result, "[unknown] [--:--:--] file");
    }

    #[test]
    fn format_item_line_shows_full_url_for_virtual_items() {
        let item = PlaylistItem {
            path: ItemPath::Url("https://youtube.com/watch?v=abc123".to_string()),
            duration: None,
            alias: None,
            mime_type: Some("url".to_string()),
            is_virtual: true,
            playlist_count: 0,
            has_sources: true,
        };
        let result = format_item_line(&item, ItemDisplayMode::Path, 0, 0, 2);
        assert_eq!(
            result,
            "[url] [--:--:--] https://youtube.com/watch?v=abc123"
        );
    }

    #[test]
    fn format_item_line_shows_full_url_with_alias() {
        let item = PlaylistItem {
            path: ItemPath::Url("https://youtube.com/watch?v=abc123".to_string()),
            duration: None,
            alias: Some("My Video".to_string()),
            mime_type: Some("url".to_string()),
            is_virtual: true,
            playlist_count: 0,
            has_sources: true,
        };
        let result = format_item_line(&item, ItemDisplayMode::Path, 0, 0, 2);
        assert_eq!(
            result,
            "[url] [--:--:--] https://youtube.com/watch?v=abc123 / My Video"
        );
    }

    #[test]
    fn format_item_line_shows_file_stem_for_non_virtual_items() {
        let item = PlaylistItem {
            path: ItemPath::File(CanonicalPath::new(PathBuf::from("/path/to/video.mp4"))),
            duration: None,
            alias: None,
            mime_type: Some("video/mp4".to_string()),
            is_virtual: false,
            playlist_count: 0,
            has_sources: true,
        };
        let result = format_item_line(&item, ItemDisplayMode::Path, 0, 0, 2);
        assert_eq!(result, "[video/mp4] [--:--:--] video");
    }

    #[test]
    fn format_mime_type_shows_url_without_extra_brackets() {
        let mime = "url";
        let result = format_mime_type(Some(mime));
        assert_eq!(result, "url");
    }

    #[test]
    fn format_mime_type_shows_deleted_without_extra_brackets() {
        let mime = "deleted";
        let result = format_mime_type(Some(mime));
        assert_eq!(result, "deleted");
    }

    #[test]
    fn format_item_line_no_count_shown_when_count_is_zero() {
        let item = PlaylistItem {
            path: ItemPath::File(CanonicalPath::new(PathBuf::from("/path/to/video.mp4"))),
            duration: Some(Duration::from_secs(65)),
            alias: Some("My Video".to_string()),
            mime_type: Some("video/mp4".to_string()),
            is_virtual: false,
            playlist_count: 0,
            has_sources: true,
        };
        let result = format_item_line(&item, ItemDisplayMode::Alias, 80, 0, 2);
        assert_eq!(result, "[video/mp4] [00:01:05] My Video");
    }

    #[test]
    fn format_item_line_no_count_shown_when_count_is_one() {
        let item = PlaylistItem {
            path: ItemPath::File(CanonicalPath::new(PathBuf::from("/path/to/video.mp4"))),
            duration: Some(Duration::from_secs(65)),
            alias: Some("My Video".to_string()),
            mime_type: Some("video/mp4".to_string()),
            is_virtual: false,
            playlist_count: 1,
            has_sources: true,
        };
        let result = format_item_line(&item, ItemDisplayMode::Alias, 80, 1, 2);
        assert_eq!(result, "[video/mp4] [00:01:05] My Video");
    }

    #[test]
    fn format_item_line_shows_count_when_count_is_two() {
        let item = PlaylistItem {
            path: ItemPath::File(CanonicalPath::new(PathBuf::from("/path/to/video.mp4"))),
            duration: Some(Duration::from_secs(65)),
            alias: Some("My Video".to_string()),
            mime_type: Some("video/mp4".to_string()),
            is_virtual: false,
            playlist_count: 2,
            has_sources: true,
        };
        let result = format_item_line(&item, ItemDisplayMode::Alias, 50, 2, 2);
        assert_eq!(result, "[video/mp4] [00:01:05] My Video                (2)");
    }

    #[test]
    fn format_item_line_shows_count_when_count_is_three() {
        let item = PlaylistItem {
            path: ItemPath::File(CanonicalPath::new(PathBuf::from("/path/to/video.mp4"))),
            duration: Some(Duration::from_secs(65)),
            alias: Some("My Video".to_string()),
            mime_type: Some("video/mp4".to_string()),
            is_virtual: false,
            playlist_count: 3,
            has_sources: true,
        };
        let result = format_item_line(&item, ItemDisplayMode::Alias, 50, 3, 2);
        assert_eq!(result, "[video/mp4] [00:01:05] My Video                (3)");
    }

    #[test]
    fn format_item_line_no_count_when_pane_width_is_zero() {
        let item = PlaylistItem {
            path: ItemPath::File(CanonicalPath::new(PathBuf::from("/path/to/video.mp4"))),
            duration: Some(Duration::from_secs(65)),
            alias: Some("My Video".to_string()),
            mime_type: Some("video/mp4".to_string()),
            is_virtual: false,
            playlist_count: 5,
            has_sources: true,
        };
        let result = format_item_line(&item, ItemDisplayMode::Alias, 0, 5, 2);
        assert_eq!(result, "[video/mp4] [00:01:05] My Video");
    }

    #[test]
    fn format_item_line_no_count_when_line_too_long() {
        let item = PlaylistItem {
            path: ItemPath::File(CanonicalPath::new(PathBuf::from("/path/to/video.mp4"))),
            duration: Some(Duration::from_secs(65)),
            alias: Some("My Video".to_string()),
            mime_type: Some("video/mp4".to_string()),
            is_virtual: false,
            playlist_count: 2,
            has_sources: true,
        };
        let result = format_item_line(&item, ItemDisplayMode::Alias, 20, 2, 2);
        assert_eq!(result, "[video/mp4] [00:01:05] My Video");
    }

    #[test]
    fn format_item_line_shows_count_when_min_count_is_one_and_count_is_one() {
        let item = PlaylistItem {
            path: ItemPath::File(CanonicalPath::new(PathBuf::from("/path/to/video.mp4"))),
            duration: Some(Duration::from_secs(65)),
            alias: Some("My Video".to_string()),
            mime_type: Some("video/mp4".to_string()),
            is_virtual: false,
            playlist_count: 1,
            has_sources: true,
        };
        let result = format_item_line(&item, ItemDisplayMode::Alias, 50, 1, 1);
        assert_eq!(result, "[video/mp4] [00:01:05] My Video                (1)");
    }
}
