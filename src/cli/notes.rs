use std::{
    fmt::Write,
    io::Write as IoWrite,
    path::PathBuf,
    process::{Command, Stdio},
};

use clap::Subcommand;
use error_stack::{Report, ResultExt};

use crate::feat::{ExternalEditor, NoteDb, PathResolver};
use crate::services::Services;

use super::{utils::create_symlink_with_suffix, RunError};

#[derive(Subcommand)]
pub enum NotesCommands {
    /// Add notes to files
    Add { paths: Vec<PathBuf> },

    /// Search for words in notes
    Search {
        /// Search query. Uses AND matching.
        query: String,
        /// Create symlinks to located results in current directory.
        #[arg(long)]
        symlink: bool,
    },

    /// Fuzzy search through all notes
    Fuzzy {
        /// Create symlinks to located results in current directory.
        #[arg(long)]
        symlink: bool,
    },
}

/// Runs a notes command.
///
/// # Errors
///
/// Returns an error if:
/// - The database cannot be accessed
/// - Path resolution fails
/// - The editor fails to open
/// - The fuzzy search process fails to spawn or communicate
pub fn run_notes_command(cmd: NotesCommands, db_path: &std::path::Path) -> Result<(), Report<RunError>> {
    let rt = tokio::runtime::Runtime::new().change_context(RunError)?;
    rt.block_on(async { run_notes_command_async(cmd, db_path).await })
}

#[allow(clippy::too_many_lines)]
async fn run_notes_command_async(
    cmd: NotesCommands,
    db_path: &std::path::Path,
) -> Result<(), Report<RunError>> {
    let services = Services::new(&db_path.to_string_lossy())
        .await
        .change_context(RunError)?;

    match cmd {
        NotesCommands::Add { paths } => {
            if paths.is_empty() {
                return Err(Report::new(RunError));
            }

            let mut resolved_paths = Vec::with_capacity(paths.len());
            for path in paths {
                let resolved = services
                    .path_resolver
                    .resolve(&path)
                    .await
                    .change_context(RunError)?;
                resolved_paths.push(resolved);
            }

            if resolved_paths.len() == 1 {
                let resolved_path = &resolved_paths[0];
                let path_str = resolved_path.to_string_lossy();
                let file_path_id = services
                    .db
                    .get_or_create_file_path(&path_str)
                    .await
                    .change_context(RunError)?;

                let existing_note = services
                    .db
                    .get_note(file_path_id)
                    .await
                    .change_context(RunError)?;

                let initial_content = existing_note.unwrap_or_default();
                if let Some(new_content) = services
                    .editor
                    .open(&initial_content)
                    .await
                    .change_context(RunError)?
                {
                    services
                        .db
                        .upsert_note(file_path_id, &new_content)
                        .await
                        .change_context(RunError)?;
                }
            } else if let Some(new_content) = services
                .editor
                .open("")
                .await
                .change_context(RunError)?
            {
                for resolved_path in resolved_paths {
                    let path_str = resolved_path.to_string_lossy();
                    let file_path_id = services
                        .db
                        .get_or_create_file_path(&path_str)
                        .await
                        .change_context(RunError)?;

                    let existing_note = services
                        .db
                        .get_note(file_path_id)
                        .await
                        .change_context(RunError)?;

                    let final_content = match existing_note {
                        Some(existing) => format!("{existing}\n\n{new_content}"),
                        None => new_content.clone(),
                    };

                    services
                        .db
                        .upsert_note(file_path_id, &final_content)
                        .await
                        .change_context(RunError)?;
                }
            }
        }
        NotesCommands::Search { query, symlink } => {
            let results = services
                .db
                .search_notes(&query)
                .await
                .change_context(RunError)?;

            let cwd = std::env::current_dir().change_context(RunError)?;

            for path in &results {
                println!("{path}");
            }

            if symlink {
                for path in &results {
                    let src = PathBuf::from(path);
                    match create_symlink_with_suffix(&src, &cwd) {
                        Ok(dest) => eprintln!("Created symlink: {}", dest.display()),
                        Err(e) => eprintln!("Failed to create symlink for {path}: {e:?}"),
                    }
                }
            }
        }
        NotesCommands::Fuzzy { symlink } => {
            let notes = services
                .db
                .get_all_notes_with_paths()
                .await
                .change_context(RunError)?;

            if notes.is_empty() {
                return Ok(());
            }

            let input: String = notes.iter().fold(String::new(), |mut output, (path, content)| {
                let cleaned: String = content
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .collect::<Vec<_>>()
                    .join(". ");
                let _ = writeln!(output, "{path}\t{cleaned}");
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
                .change_context(RunError)?;

            if let Some(mut stdin) = child.stdin.take() {
                stdin
                    .write_all(input.as_bytes())
                    .change_context(RunError)?;
            }

            let output = child
                .wait_with_output()
                .change_context(RunError)?;

            let selected = String::from_utf8_lossy(&output.stdout);
            let selected_paths: Vec<&str> = selected
                .lines()
                .filter_map(|line| line.split('\t').next())
                .collect();

            for path in &selected_paths {
                println!("{path}");
            }

            if symlink {
                let cwd = std::env::current_dir().change_context(RunError)?;
                for path in &selected_paths {
                    let src = PathBuf::from(path);
                    match create_symlink_with_suffix(&src, &cwd) {
                        Ok(dest) => eprintln!("Created symlink: {}", dest.display()),
                        Err(e) => eprintln!("Failed to create symlink for {path}: {e:?}"),
                    }
                }
            }
        }
    }

    Ok(())
}
