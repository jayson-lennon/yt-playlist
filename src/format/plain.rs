use crate::format::{ShowNotesEntry, ShowNotesFormat};

pub struct PlainFormat;

impl ShowNotesFormat for PlainFormat {
    fn format(&self, entries: &[ShowNotesEntry]) -> String {
        entries
            .iter()
            .filter(|e| !e.sources.is_empty())
            .flat_map(|entry| {
                entry
                    .sources
                    .iter()
                    .map(move |url| format!("{}: {}", entry.display_name(), url))
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn name(&self) -> &'static str {
        "plain"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, alias: Option<&str>, sources: Vec<&str>) -> ShowNotesEntry {
        ShowNotesEntry {
            path: format!("/path/{}", name),
            filename: name.to_string(),
            alias: alias.map(|s| s.to_string()),
            sources: sources.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn format_creates_plain_list() {
        let entries = vec![entry("video.mp4", None, vec!["https://example.com"])];
        let output = PlainFormat.format(&entries);
        assert_eq!(output, "video.mp4: https://example.com");
    }

    #[test]
    fn format_uses_alias() {
        let entries = vec![entry(
            "video.mp4",
            Some("My Video"),
            vec!["https://example.com"],
        )];
        let output = PlainFormat.format(&entries);
        assert_eq!(output, "My Video: https://example.com");
    }

    #[test]
    fn format_multiple_sources() {
        let entries = vec![entry(
            "video.mp4",
            None,
            vec!["https://a.com", "https://b.com"],
        )];
        let output = PlainFormat.format(&entries);
        assert_eq!(output, "video.mp4: https://a.com\nvideo.mp4: https://b.com");
    }

    #[test]
    fn format_skips_entries_without_sources() {
        let entries = vec![
            entry("video.mp4", None, vec!["https://example.com"]),
            entry("no-source.mp4", None, vec![]),
        ];
        let output = PlainFormat.format(&entries);
        assert_eq!(output, "video.mp4: https://example.com");
    }

    #[test]
    fn format_empty_entries() {
        let entries: Vec<ShowNotesEntry> = vec![];
        let output = PlainFormat.format(&entries);
        assert!(output.is_empty());
    }
}
