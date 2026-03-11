#![allow(clippy::missing_panics_doc)]
use cucumber::{World, given, then, when};

use acceptance::ShownotesWorld;
use shownotes::NoteDb;
use shownotes::PathResolver;
use shownotes::command::notes::add_alias_as_note;

#[derive(Debug, World)]
#[world(init = Self::new_world)]
pub struct AliasNotesWorld {
    inner: ShownotesWorld,
}

impl AliasNotesWorld {
    async fn new_world() -> Self {
        Self {
            inner: ShownotesWorld::new().await,
        }
    }

    async fn get_note_for_path(&self, path: &str) -> Option<String> {
        let full_path = self.inner.resolve_path(path);
        let resolved = self
            .inner
            .services
            .path_resolver
            .resolve(&full_path)
            .await
            .expect("failed to resolve path");
        let path_str = resolved.to_string_lossy().to_string();
        let file_path_id = self
            .inner
            .services
            .db
            .get_or_create_file_path(&path_str)
            .await
            .expect("failed to get or create file path");
        self.inner
            .services
            .db
            .get_note(file_path_id)
            .await
            .expect("failed to get note")
    }

    async fn get_file_path_id(&self, path: &str) -> i64 {
        let full_path = self.inner.resolve_path(path);
        let resolved = self
            .inner
            .services
            .path_resolver
            .resolve(&full_path)
            .await
            .expect("failed to resolve path");
        let path_str = resolved.to_string_lossy().to_string();
        self.inner
            .services
            .db
            .get_or_create_file_path(&path_str)
            .await
            .expect("failed to get or create file path")
    }
}

#[given(expr = r#"a real file at {string}"#)]
fn given_real_file(world: &mut AliasNotesWorld, filename: String) {
    world.inner.create_file(&filename);
}

#[given(expr = r#"a symlink to {string} at {string}"#)]
fn given_symlink(world: &mut AliasNotesWorld, target: String, link: String) {
    world.inner.create_symlink(&target, &link);
}

#[given(expr = r#"the file {string} has note {string}"#)]
async fn given_file_has_note(world: &mut AliasNotesWorld, path: String, note: String) {
    let file_path_id = world.get_file_path_id(&path).await;

    let existing = world.get_note_for_path(&path).await;

    let new_content = match existing {
        Some(notes) if !notes.is_empty() => format!("{notes}\n{note}"),
        _ => note,
    };

    world
        .inner
        .services
        .db
        .upsert_note(file_path_id, &new_content)
        .await
        .expect("failed to upsert note");
}

#[when(expr = r#"I add alias {string} to {string}"#)]
async fn when_add_alias(world: &mut AliasNotesWorld, alias: String, path: String) {
    let full_path = world.inner.resolve_path(&path);
    add_alias_as_note(&world.inner.services, &full_path, &alias)
        .await
        .expect("add_alias_as_note failed");
}

#[then(expr = r#"the file {string} has note {string}"#)]
async fn then_file_has_note(world: &mut AliasNotesWorld, path: String, expected: String) {
    let note = world.get_note_for_path(&path).await;

    match note {
        Some(content) => {
            assert!(
                content.contains(&expected),
                "expected note to contain '{expected}', but got: '{content}'"
            );
        }
        None => {
            panic!("expected file '{path}' to have note '{expected}', but no note exists");
        }
    }
}

#[then(expr = r#"the file {string} has no notes"#)]
async fn then_file_has_no_notes(world: &mut AliasNotesWorld, path: String) {
    let note = world.get_note_for_path(&path).await;

    assert!(
        note.is_none() || note.as_ref().is_none_or(String::is_empty),
        "expected file '{path}' to have no notes, but found: '{note:?}'"
    );
}

#[then(expr = r#"the file {string} has exactly {int} note line"#)]
async fn then_file_has_exactly_n_note_lines(
    world: &mut AliasNotesWorld,
    path: String,
    count: usize,
) {
    let note = world.get_note_for_path(&path).await;

    let line_count = match note {
        Some(content) if !content.is_empty() => content.lines().count(),
        _ => 0,
    };

    assert_eq!(
        line_count, count,
        "expected file '{path}' to have exactly {count} note lines, but found {line_count}"
    );
}

#[tokio::main]
async fn main() {
    AliasNotesWorld::run("tests/features/alias_to_notes.feature").await;
}
