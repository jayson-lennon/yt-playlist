use super::{ShowNotesEntry, ShowNotesFormat};

pub struct HtmlFormat;

impl ShowNotesFormat for HtmlFormat {
    fn format(&self, entries: &[ShowNotesEntry]) -> String {
        entries
            .iter()
            .filter(|e| !e.sources.is_empty())
            .filter_map(|entry| {
                entry
                    .sources
                    .first()
                    .map(|url| format!("<a href=\"{}\">{}</a>", url, entry.display_name()))
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn name(&self) -> &'static str {
        "html"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, alias: Option<&str>, sources: Vec<&str>) -> ShowNotesEntry {
        ShowNotesEntry {
            path: format!("/path/{name}"),
            filename: name.to_string(),
            alias: alias.map(ToString::to_string),
            sources: sources.iter().map(ToString::to_string).collect(),
        }
    }

    #[test]
    fn format_creates_html_links() {
        let entries = vec![entry("video.mp4", None, vec!["https://example.com"])];
        let output = HtmlFormat.format(&entries);
        assert_eq!(output, "<a href=\"https://example.com\">video.mp4</a>");
    }

    #[test]
    fn format_uses_alias() {
        let entries = vec![entry(
            "video.mp4",
            Some("My Video"),
            vec!["https://example.com"],
        )];
        let output = HtmlFormat.format(&entries);
        assert_eq!(output, "<a href=\"https://example.com\">My Video</a>");
    }

    #[test]
    fn format_multiple_sources() {
        let entries = vec![entry(
            "video.mp4",
            None,
            vec!["https://a.com", "https://b.com"],
        )];
        let output = HtmlFormat.format(&entries);
        assert_eq!(output, "<a href=\"https://a.com\">video.mp4</a>");
    }

    #[test]
    fn format_uses_only_first_source() {
        let entries = vec![entry(
            "video.mp4",
            None,
            vec![
                "https://first.com",
                "https://second.com",
                "https://third.com",
            ],
        )];
        let output = HtmlFormat.format(&entries);
        assert_eq!(output, "<a href=\"https://first.com\">video.mp4</a>");
        assert!(!output.contains("second.com"));
        assert!(!output.contains("third.com"));
        assert_eq!(output.matches("video.mp4").count(), 1);
    }

    #[test]
    fn format_skips_entries_without_sources() {
        let entries = vec![
            entry("video.mp4", None, vec!["https://example.com"]),
            entry("no-source.mp4", None, vec![]),
        ];
        let output = HtmlFormat.format(&entries);
        assert_eq!(output, "<a href=\"https://example.com\">video.mp4</a>");
    }

    #[test]
    fn format_empty_entries() {
        let entries: Vec<ShowNotesEntry> = vec![];
        let output = HtmlFormat.format(&entries);
        assert!(output.is_empty());
    }
}
