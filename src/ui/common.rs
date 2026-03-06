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
