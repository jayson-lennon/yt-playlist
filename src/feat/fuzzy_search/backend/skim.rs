use std::{
    fmt::Write,
    process::{Command, Stdio},
};

use error_stack::{Report, ResultExt};

use crate::feat::fuzzy_search::{FuzzySearch, FuzzySearchError, FuzzySearchResult};

pub struct SkimBackend;

impl FuzzySearch for SkimBackend {
    fn name(&self) -> &'static str {
        "skim"
    }

    fn search(
        &self,
        items: &[(String, String)],
    ) -> Result<FuzzySearchResult, Report<FuzzySearchError>> {
        let input: String = items
            .iter()
            .fold(String::new(), |mut output, (path, content)| {
                let cleaned: String = content
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .collect::<Vec<_>>()
                    .join(". ");
                let _ = writeln!(output, "{}\t{}", path, cleaned);
                output
            });

        let mut child = Command::new("sk")
            .args([
                "-m",
                "--delimiter=\\t",
                "--with-nth=2..",
                "--color=marker:51,hl+:201,hl:219",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .change_context(FuzzySearchError("Failed to spawn skim".to_string()))?;

        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write as IoWrite;
            stdin
                .write_all(input.as_bytes())
                .change_context(FuzzySearchError("Failed to write to skim".to_string()))?;
        }

        let output = child
            .wait_with_output()
            .change_context(FuzzySearchError("Failed to read skim output".to_string()))?;

        let selected = String::from_utf8_lossy(&output.stdout);
        let selected_paths: Vec<String> = selected
            .lines()
            .filter_map(|line| line.split('\t').next())
            .map(ToString::to_string)
            .collect();

        Ok(FuzzySearchResult { selected_paths })
    }
}
