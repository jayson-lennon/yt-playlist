use std::{path::PathBuf, time::Duration};

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Playlist,
    Directory,
}

#[derive(Debug, Clone)]
pub struct PlaylistItem {
    pub path: PathBuf,
    pub duration: Option<Duration>,
    pub alias: Option<String>,
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
        item.path.file_name().map_or_else(
            || item.path.to_string_lossy().into_owned(),
            |n| n.to_string_lossy().into_owned(),
        )
    })
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
        }
    }

    fn item_with_alias(path: &str, alias: &str) -> PlaylistItem {
        PlaylistItem {
            path: PathBuf::from(path),
            duration: None,
            alias: Some(alias.to_string()),
        }
    }

    fn item_with_duration(path: &str, secs: u64) -> PlaylistItem {
        PlaylistItem {
            path: PathBuf::from(path),
            duration: Some(Duration::from_secs(secs)),
            alias: None,
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

        // Then filename is returned.
        assert_eq!(result, "video.mp4");
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
}
