use std::collections::HashSet;
use std::path::Path;
use std::str::FromStr;

use async_trait::async_trait;
use error_stack::{Report, ResultExt};
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;
use wherror::Error;

use super::super::{NoteDb, NoteDbError};

#[derive(Debug, Error)]
pub enum SqliteNoteDbError {
    #[error("failed to connect to database")]
    Connect,
    #[error("failed to run migrations")]
    Migrate,
    #[error("database operation failed")]
    Query,
}

#[derive(Debug, Clone)]
pub struct SqliteNoteDb {
    pool: SqlitePool,
}

/// # Errors
/// Returns an error if the connection fails or migrations cannot be run.
pub async fn connect_and_migrate(
    database_url: &str,
) -> Result<SqlitePool, Report<SqliteNoteDbError>> {
    let options = SqliteConnectOptions::from_str(database_url)
        .change_context(SqliteNoteDbError::Connect)?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .busy_timeout(std::time::Duration::from_secs(30))
        .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
        .foreign_keys(true);

    let pool = SqlitePool::connect_with(options)
        .await
        .change_context(SqliteNoteDbError::Connect)?;

    sqlx::migrate!()
        .run(&pool)
        .await
        .change_context(SqliteNoteDbError::Migrate)?;

    Ok(pool)
}

impl SqliteNoteDb {
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// # Errors
    /// Returns an error if the connection fails or migrations cannot be run.
    pub async fn new(database_url: &str) -> Result<Self, Report<SqliteNoteDbError>> {
        let pool = connect_and_migrate(database_url).await?;
        Ok(Self { pool })
    }

    pub async fn close(&self) {
        self.pool.close().await;
    }
}

#[async_trait]
impl NoteDb for SqliteNoteDb {
    async fn get_or_create_file_path(&self, path: &str) -> Result<i64, Report<NoteDbError>> {
        let result = sqlx::query_scalar::<_, i64>("SELECT id FROM file_paths WHERE path = ?")
            .bind(path)
            .fetch_optional(&self.pool)
            .await
            .change_context(NoteDbError)?;

        if let Some(id) = result {
            return Ok(id);
        }

        let id =
            sqlx::query_scalar::<_, i64>("INSERT INTO file_paths (path) VALUES (?) RETURNING id")
                .bind(path)
                .fetch_one(&self.pool)
                .await
                .change_context(NoteDbError)?;

        Ok(id)
    }

    async fn get_note(&self, file_path_id: i64) -> Result<Option<String>, Report<NoteDbError>> {
        let result =
            sqlx::query_scalar::<_, String>("SELECT content FROM notes WHERE file_path_id = ?")
                .bind(file_path_id)
                .fetch_optional(&self.pool)
                .await
                .change_context(NoteDbError)?;

        Ok(result)
    }

    async fn upsert_note(
        &self,
        file_path_id: i64,
        content: &str,
    ) -> Result<(), Report<NoteDbError>> {
        sqlx::query(
            r#"
            INSERT INTO notes (file_path_id, content, updated_at)
            VALUES (?, ?, datetime('now'))
            ON CONFLICT(file_path_id) DO UPDATE SET
                content = excluded.content,
                updated_at = datetime('now')
            "#,
        )
        .bind(file_path_id)
        .bind(content)
        .execute(&self.pool)
        .await
        .change_context(NoteDbError)?;

        Ok(())
    }

    async fn search_notes(&self, query: &str) -> Result<HashSet<String>, Report<NoteDbError>> {
        let terms: Vec<&str> = query.split_whitespace().collect();

        if terms.is_empty() {
            return Ok(HashSet::new());
        }

        let mut conditions: Vec<String> = Vec::new();
        let mut params: Vec<String> = Vec::new();

        for term in &terms {
            conditions.push("LOWER(n.content) LIKE LOWER(?)".to_string());
            params.push(format!("%{term}%"));
        }

        let where_clause = conditions.join(" AND ");
        let sql = format!(
            "SELECT DISTINCT fp.path FROM file_paths fp JOIN notes n ON fp.id = n.file_path_id WHERE {where_clause}"
        );

        let mut query_builder = sqlx::query_as::<_, (String,)>(&sql);
        for param in &params {
            query_builder = query_builder.bind(param);
        }

        let results = query_builder
            .fetch_all(&self.pool)
            .await
            .change_context(NoteDbError)?;

        Ok(results.into_iter().map(|(path,)| path).collect())
    }

    async fn get_all_notes_with_paths(&self) -> Result<Vec<(String, String)>, Report<NoteDbError>> {
        let results = sqlx::query_as::<_, (String, String)>(
            "SELECT fp.path, n.content FROM file_paths fp JOIN notes n ON fp.id = n.file_path_id",
        )
        .fetch_all(&self.pool)
        .await
        .change_context(NoteDbError)?;

        Ok(results)
    }

    async fn get_all_paths_for_fuzzy_search(&self) -> Result<Vec<(String, String)>, Report<NoteDbError>> {
        let results = sqlx::query_as::<_, (String, Option<String>)>(
            "SELECT fp.path, n.content FROM file_paths fp LEFT JOIN notes n ON fp.id = n.file_path_id",
        )
        .fetch_all(&self.pool)
        .await
        .change_context(NoteDbError)?;

        let processed = results
            .into_iter()
            .map(|(path, content)| {
                let search_content = content.unwrap_or_else(|| {
                    Path::new(&path)
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or(&path)
                        .to_string()
                });
                (path, search_content)
            })
            .collect();

        Ok(processed)
    }

    async fn close(&self) {
        self.pool.close().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::create_temp_file;

    async fn create_test_db() -> SqliteNoteDb {
        SqliteNoteDb::new("sqlite::memory:").await.unwrap()
    }

    #[tokio::test]
    async fn get_or_create_file_path_creates_new_path() {
        // Given a new database and temp file.
        let db = create_test_db().await;
        let temp = create_temp_file();
        let path = temp.path().to_str().unwrap();

        // When creating a file path.
        let id = db.get_or_create_file_path(path).await.unwrap();

        // Then a valid ID is returned.
        assert!(id > 0);
    }

    #[tokio::test]
    async fn get_or_create_file_path_returns_same_id_for_same_path() {
        // Given a database and a file path.
        let db = create_test_db().await;
        let temp = create_temp_file();
        let path = temp.path().to_str().unwrap();

        // When getting or creating the same path twice.
        let id1 = db.get_or_create_file_path(path).await.unwrap();
        let id2 = db.get_or_create_file_path(path).await.unwrap();

        // Then both calls return the same ID.
        assert_eq!(id1, id2);
    }

    #[tokio::test]
    async fn get_or_create_file_path_returns_different_ids_for_different_paths() {
        // Given a database and two different file paths.
        let db = create_test_db().await;
        let temp1 = create_temp_file();
        let temp2 = create_temp_file();
        let path1 = temp1.path().to_str().unwrap();
        let path2 = temp2.path().to_str().unwrap();

        // When creating file paths for each.
        let id1 = db.get_or_create_file_path(path1).await.unwrap();
        let id2 = db.get_or_create_file_path(path2).await.unwrap();

        // Then different IDs are returned.
        assert_ne!(id1, id2);
    }

    #[tokio::test]
    async fn get_note_returns_none_when_no_note_exists() {
        // Given a database with a file path but no note.
        let db = create_test_db().await;
        let temp = create_temp_file();
        let path = temp.path().to_str().unwrap();
        let file_path_id = db.get_or_create_file_path(path).await.unwrap();

        // When getting the note.
        let result = db.get_note(file_path_id).await.unwrap();

        // Then no note is found.
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn get_note_returns_content_when_note_exists() {
        // Given a database with a file path and a note.
        let db = create_test_db().await;
        let temp = create_temp_file();
        let path = temp.path().to_str().unwrap();
        let file_path_id = db.get_or_create_file_path(path).await.unwrap();
        db.upsert_note(file_path_id, "test content").await.unwrap();

        // When getting the note.
        let result = db.get_note(file_path_id).await.unwrap();

        // Then the note content is returned.
        assert_eq!(result, Some("test content".to_string()));
    }

    #[tokio::test]
    async fn upsert_note_inserts_new_note() {
        // Given a database with a file path but no note.
        let db = create_test_db().await;
        let temp = create_temp_file();
        let path = temp.path().to_str().unwrap();
        let file_path_id = db.get_or_create_file_path(path).await.unwrap();

        // When upserting a new note.
        db.upsert_note(file_path_id, "new note").await.unwrap();
        let result = db.get_note(file_path_id).await.unwrap();

        // Then the note is stored.
        assert_eq!(result, Some("new note".to_string()));
    }

    #[tokio::test]
    async fn upsert_note_updates_existing_note() {
        // Given a database with a file path and an existing note.
        let db = create_test_db().await;
        let temp = create_temp_file();
        let path = temp.path().to_str().unwrap();
        let file_path_id = db.get_or_create_file_path(path).await.unwrap();
        db.upsert_note(file_path_id, "original content")
            .await
            .unwrap();

        // When upserting updated content.
        db.upsert_note(file_path_id, "updated content")
            .await
            .unwrap();
        let result = db.get_note(file_path_id).await.unwrap();

        // Then the note is updated.
        assert_eq!(result, Some("updated content".to_string()));
    }

    #[rstest::rstest]
    #[case::empty("")]
    #[case::single_char("a")]
    #[case::multiline("line1\nline2\nline3")]
    #[case::unicode("Hello 世界 🌍")]
    #[tokio::test]
    async fn upsert_note_handles_various_content(#[case] content: &str) {
        // Given a database with a file path.
        let db = create_test_db().await;
        let temp = create_temp_file();
        let path = temp.path().to_str().unwrap();
        let file_path_id = db.get_or_create_file_path(path).await.unwrap();

        // When upserting the note.
        db.upsert_note(file_path_id, content).await.unwrap();
        let result = db.get_note(file_path_id).await.unwrap();

        // Then the content is stored correctly.
        assert_eq!(result, Some(content.to_string()));
    }

    #[tokio::test]
    async fn full_workflow_create_path_and_note() {
        // Given a new database.
        let db = create_test_db().await;
        let temp = create_temp_file();
        let path = temp.path().to_str().unwrap();

        // When creating a file path, adding a note, and retrieving it.
        let file_path_id = db.get_or_create_file_path(path).await.unwrap();
        db.upsert_note(file_path_id, "my note content")
            .await
            .unwrap();
        let note = db.get_note(file_path_id).await.unwrap();

        // Then the note is stored and retrieved correctly.
        assert_eq!(note, Some("my note content".to_string()));
    }

    #[tokio::test]
    async fn multiple_file_paths_with_separate_notes() {
        // Given a database with two file paths.
        let db = create_test_db().await;
        let temp1 = create_temp_file();
        let temp2 = create_temp_file();
        let path1 = temp1.path().to_str().unwrap();
        let path2 = temp2.path().to_str().unwrap();

        // When creating file paths and adding notes to each.
        let id1 = db.get_or_create_file_path(path1).await.unwrap();
        let id2 = db.get_or_create_file_path(path2).await.unwrap();
        db.upsert_note(id1, "note for file 1").await.unwrap();
        db.upsert_note(id2, "note for file 2").await.unwrap();

        // Then each file has its own note.
        let note1 = db.get_note(id1).await.unwrap();
        let note2 = db.get_note(id2).await.unwrap();
        assert_eq!(note1, Some("note for file 1".to_string()));
        assert_eq!(note2, Some("note for file 2".to_string()));
    }

    #[tokio::test]
    async fn get_all_paths_for_fuzzy_search_returns_paths_with_notes() {
        // Given a database with a file path that has a note.
        let db = create_test_db().await;
        let temp = create_temp_file();
        let path = temp.path().to_str().unwrap();
        let file_path_id = db.get_or_create_file_path(path).await.unwrap();
        db.upsert_note(file_path_id, "my note content")
            .await
            .unwrap();

        // When getting all paths for fuzzy search.
        let results = db.get_all_paths_for_fuzzy_search().await.unwrap();

        // Then the path is returned with its note content.
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, path);
        assert_eq!(results[0].1, "my note content");
    }

    #[tokio::test]
    async fn get_all_paths_for_fuzzy_search_returns_filename_when_no_note() {
        // Given a database with a file path that has no note.
        let db = create_test_db().await;
        let temp = create_temp_file();
        let path = temp.path().to_str().unwrap();
        db.get_or_create_file_path(path).await.unwrap();

        // When getting all paths for fuzzy search.
        let results = db.get_all_paths_for_fuzzy_search().await.unwrap();

        // Then the path is returned with just the filename.
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, path);
        // The second element should be just the filename, not the full path
        let expected_filename = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap();
        assert_eq!(results[0].1, expected_filename);
        assert!(!results[0].1.contains('/')); // filename shouldn't have path separators
    }

    #[tokio::test]
    async fn get_all_paths_for_fuzzy_search_returns_both_with_and_without_notes() {
        // Given a database with two file paths, one with a note and one without.
        let db = create_test_db().await;
        let temp1 = create_temp_file();
        let temp2 = create_temp_file();
        let path1 = temp1.path().to_str().unwrap();
        let path2 = temp2.path().to_str().unwrap();

        let id1 = db.get_or_create_file_path(path1).await.unwrap();
        db.upsert_note(id1, "note for file 1").await.unwrap();
        db.get_or_create_file_path(path2).await.unwrap(); // no note

        // When getting all paths for fuzzy search.
        let results = db.get_all_paths_for_fuzzy_search().await.unwrap();

        // Then both paths are returned with appropriate content.
        assert_eq!(results.len(), 2);

        let entry1 = results.iter().find(|(p, _)| p == path1).unwrap();
        assert_eq!(entry1.1, "note for file 1");

        let entry2 = results.iter().find(|(p, _)| p == path2).unwrap();
        let expected_filename = Path::new(path2)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap();
        assert_eq!(entry2.1, expected_filename);
        assert!(!entry2.1.contains('/'));
    }

    #[tokio::test]
    async fn get_all_paths_for_fuzzy_search_returns_empty_when_no_paths() {
        // Given an empty database.
        let db = create_test_db().await;

        // When getting all paths for fuzzy search.
        let results = db.get_all_paths_for_fuzzy_search().await.unwrap();

        // Then an empty list is returned.
        assert!(results.is_empty());
    }
}
