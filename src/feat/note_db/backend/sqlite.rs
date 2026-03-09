use std::collections::HashSet;
use std::str::FromStr;

use async_trait::async_trait;
use error_stack::Report;
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
        .map_err(|_| Report::new(SqliteNoteDbError::Connect))?
        .create_if_missing(true);

    let pool = SqlitePool::connect_with(options)
        .await
        .map_err(|_| Report::new(SqliteNoteDbError::Connect))?;

    sqlx::migrate!()
        .run(&pool)
        .await
        .map_err(|_| Report::new(SqliteNoteDbError::Migrate))?;

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
}

#[async_trait]
impl NoteDb for SqliteNoteDb {
    async fn get_or_create_file_path(&self, path: &str) -> Result<i64, Report<NoteDbError>> {
        let result = sqlx::query_scalar::<_, i64>("SELECT id FROM file_paths WHERE path = ?")
            .bind(path)
            .fetch_optional(&self.pool)
            .await
            .map_err(|_| Report::new(NoteDbError))?;

        if let Some(id) = result {
            return Ok(id);
        }

        let id =
            sqlx::query_scalar::<_, i64>("INSERT INTO file_paths (path) VALUES (?) RETURNING id")
                .bind(path)
                .fetch_one(&self.pool)
                .await
                .map_err(|_| Report::new(NoteDbError))?;

        Ok(id)
    }

    async fn get_note(&self, file_path_id: i64) -> Result<Option<String>, Report<NoteDbError>> {
        let result =
            sqlx::query_scalar::<_, String>("SELECT content FROM notes WHERE file_path_id = ?")
                .bind(file_path_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|_| Report::new(NoteDbError))?;

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
        .map_err(|_| Report::new(NoteDbError))?;

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
            .map_err(|_| Report::new(NoteDbError))?;

        Ok(results.into_iter().map(|(path,)| path).collect())
    }

    async fn get_all_notes_with_paths(&self) -> Result<Vec<(String, String)>, Report<NoteDbError>> {
        let results = sqlx::query_as::<_, (String, String)>(
            "SELECT fp.path, n.content FROM file_paths fp JOIN notes n ON fp.id = n.file_path_id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|_| Report::new(NoteDbError))?;

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    async fn create_test_db() -> SqliteNoteDb {
        SqliteNoteDb::new("sqlite::memory:").await.unwrap()
    }

    fn create_temp_file() -> NamedTempFile {
        NamedTempFile::new().unwrap()
    }

    #[tokio::test]
    async fn get_or_create_file_path_creates_new_path() {
        let db = create_test_db().await;
        let temp = create_temp_file();
        let path = temp.path().to_str().unwrap();

        let id = db.get_or_create_file_path(path).await.unwrap();

        assert!(id > 0);
    }

    #[tokio::test]
    async fn get_or_create_file_path_returns_same_id_for_same_path() {
        let db = create_test_db().await;
        let temp = create_temp_file();
        let path = temp.path().to_str().unwrap();

        let id1 = db.get_or_create_file_path(path).await.unwrap();
        let id2 = db.get_or_create_file_path(path).await.unwrap();

        assert_eq!(id1, id2);
    }

    #[tokio::test]
    async fn get_or_create_file_path_returns_different_ids_for_different_paths() {
        let db = create_test_db().await;
        let temp1 = create_temp_file();
        let temp2 = create_temp_file();
        let path1 = temp1.path().to_str().unwrap();
        let path2 = temp2.path().to_str().unwrap();

        let id1 = db.get_or_create_file_path(path1).await.unwrap();
        let id2 = db.get_or_create_file_path(path2).await.unwrap();

        assert_ne!(id1, id2);
    }

    #[tokio::test]
    async fn get_note_returns_none_when_no_note_exists() {
        let db = create_test_db().await;
        let temp = create_temp_file();
        let path = temp.path().to_str().unwrap();
        let file_path_id = db.get_or_create_file_path(path).await.unwrap();

        let result = db.get_note(file_path_id).await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn get_note_returns_content_when_note_exists() {
        let db = create_test_db().await;
        let temp = create_temp_file();
        let path = temp.path().to_str().unwrap();
        let file_path_id = db.get_or_create_file_path(path).await.unwrap();
        db.upsert_note(file_path_id, "test content").await.unwrap();

        let result = db.get_note(file_path_id).await.unwrap();

        assert_eq!(result, Some("test content".to_string()));
    }

    #[tokio::test]
    async fn upsert_note_inserts_new_note() {
        let db = create_test_db().await;
        let temp = create_temp_file();
        let path = temp.path().to_str().unwrap();
        let file_path_id = db.get_or_create_file_path(path).await.unwrap();

        db.upsert_note(file_path_id, "new note").await.unwrap();
        let result = db.get_note(file_path_id).await.unwrap();

        assert_eq!(result, Some("new note".to_string()));
    }

    #[tokio::test]
    async fn upsert_note_updates_existing_note() {
        let db = create_test_db().await;
        let temp = create_temp_file();
        let path = temp.path().to_str().unwrap();
        let file_path_id = db.get_or_create_file_path(path).await.unwrap();
        db.upsert_note(file_path_id, "original content")
            .await
            .unwrap();

        db.upsert_note(file_path_id, "updated content")
            .await
            .unwrap();
        let result = db.get_note(file_path_id).await.unwrap();

        assert_eq!(result, Some("updated content".to_string()));
    }

    #[rstest::rstest]
    #[case::empty("")]
    #[case::single_char("a")]
    #[case::multiline("line1\nline2\nline3")]
    #[case::unicode("Hello 世界 🌍")]
    #[tokio::test]
    async fn upsert_note_handles_various_content(#[case] content: &str) {
        let db = create_test_db().await;
        let temp = create_temp_file();
        let path = temp.path().to_str().unwrap();
        let file_path_id = db.get_or_create_file_path(path).await.unwrap();

        db.upsert_note(file_path_id, content).await.unwrap();
        let result = db.get_note(file_path_id).await.unwrap();

        assert_eq!(result, Some(content.to_string()));
    }

    #[tokio::test]
    async fn full_workflow_create_path_and_note() {
        let db = create_test_db().await;
        let temp = create_temp_file();
        let path = temp.path().to_str().unwrap();

        let file_path_id = db.get_or_create_file_path(path).await.unwrap();
        db.upsert_note(file_path_id, "my note content")
            .await
            .unwrap();
        let note = db.get_note(file_path_id).await.unwrap();

        assert_eq!(note, Some("my note content".to_string()));
    }

    #[tokio::test]
    async fn multiple_file_paths_with_separate_notes() {
        let db = create_test_db().await;
        let temp1 = create_temp_file();
        let temp2 = create_temp_file();
        let path1 = temp1.path().to_str().unwrap();
        let path2 = temp2.path().to_str().unwrap();

        let id1 = db.get_or_create_file_path(path1).await.unwrap();
        let id2 = db.get_or_create_file_path(path2).await.unwrap();
        db.upsert_note(id1, "note for file 1").await.unwrap();
        db.upsert_note(id2, "note for file 2").await.unwrap();

        let note1 = db.get_note(id1).await.unwrap();
        let note2 = db.get_note(id2).await.unwrap();
        assert_eq!(note1, Some("note for file 1".to_string()));
        assert_eq!(note2, Some("note for file 2".to_string()));
    }
}
