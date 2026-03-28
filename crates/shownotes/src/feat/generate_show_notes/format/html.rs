// Copyright (C) 2026 Jayson Lennon
// 
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
// 
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
// 
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

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
        // Given entries with sources.
        let entries = vec![entry("video.mp4", None, vec!["https://example.com"])];

        // When formatting as HTML.
        let output = HtmlFormat.format(&entries);

        // Then HTML links are created.
        assert_eq!(output, "<a href=\"https://example.com\">video.mp4</a>");
    }

    #[test]
    fn format_uses_alias() {
        // Given an entry with an alias.
        let entries = vec![entry(
            "video.mp4",
            Some("My Video"),
            vec!["https://example.com"],
        )];

        // When formatting as HTML.
        let output = HtmlFormat.format(&entries);

        // Then the alias is used as the link text.
        assert_eq!(output, "<a href=\"https://example.com\">My Video</a>");
    }

    #[test]
    fn format_multiple_sources() {
        // Given an entry with multiple sources.
        let entries = vec![entry(
            "video.mp4",
            None,
            vec!["https://a.com", "https://b.com"],
        )];

        // When formatting as HTML.
        let output = HtmlFormat.format(&entries);

        // Then only the first source is used.
        assert_eq!(output, "<a href=\"https://a.com\">video.mp4</a>");
    }

    #[test]
    fn format_uses_only_first_source() {
        // Given an entry with many sources.
        let entries = vec![entry(
            "video.mp4",
            None,
            vec![
                "https://first.com",
                "https://second.com",
                "https://third.com",
            ],
        )];

        // When formatting as HTML.
        let output = HtmlFormat.format(&entries);

        // Then only the first source appears in output.
        assert_eq!(output, "<a href=\"https://first.com\">video.mp4</a>");
        assert!(!output.contains("second.com"));
        assert!(!output.contains("third.com"));
        assert_eq!(output.matches("video.mp4").count(), 1);
    }

    #[test]
    fn format_skips_entries_without_sources() {
        // Given entries where some lack sources.
        let entries = vec![
            entry("video.mp4", None, vec!["https://example.com"]),
            entry("no-source.mp4", None, vec![]),
        ];

        // When formatting as HTML.
        let output = HtmlFormat.format(&entries);

        // Then only entries with sources are included.
        assert_eq!(output, "<a href=\"https://example.com\">video.mp4</a>");
    }

    #[test]
    fn format_empty_entries() {
        // Given no entries.
        let entries: Vec<ShowNotesEntry> = vec![];

        // When formatting as HTML.
        let output = HtmlFormat.format(&entries);

        // Then the output is empty.
        assert!(output.is_empty());
    }
}
