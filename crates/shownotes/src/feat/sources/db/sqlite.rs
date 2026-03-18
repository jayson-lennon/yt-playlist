use std::collections::HashMap;

use async_trait::async_trait;
use error_stack::{Report, ResultExt};
use sqlx::SqlitePool;
use wherror::Error;

use crate::feat::sources::{Source, SourceDb, SourceDbError};

#[derive(Debug, Error)]
pub enum SqliteSourceDbError {
    #[error("failed to connect to database")]
    Connect,
    #[error("failed to run migrations")]
    Migrate,
    #[error("database operation failed")]
    Query,
}

#[derive(Debug, Clone)]
pub struct SqliteSourceDb {
    pool: SqlitePool,
}

impl SqliteSourceDb {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SourceDb for SqliteSourceDb {
    async fn get_sources(&self, file_path_id: i64) -> Result<Vec<Source>, Report<SourceDbError>> {
        let results = sqlx::query_as::<_, (i64, String, Option<String>)>(
            "SELECT id, source_url, label FROM sources WHERE file_path_id = ? ORDER BY id",
        )
        .bind(file_path_id)
        .fetch_all(&self.pool)
        .await
        .change_context(SourceDbError)?;

        Ok(results
            .into_iter()
            .map(|(id, source_url, label)| Source {
                id,
                source_url,
                label,
            })
            .collect())
    }

    async fn set_sources(
        &self,
        file_path_id: i64,
        urls: &[String],
    ) -> Result<(), Report<SourceDbError>> {
        let mut tx = self
            .pool
            .begin()
            .await
            .change_context(SourceDbError)?;

        sqlx::query("DELETE FROM sources WHERE file_path_id = ?")
            .bind(file_path_id)
            .execute(&mut *tx)
            .await
            .change_context(SourceDbError)?;

        for url in urls {
            if url.trim().is_empty() {
                continue;
            }
            sqlx::query(
                "INSERT INTO sources (file_path_id, source_url, updated_at) VALUES (?, ?, datetime('now'))",
            )
            .bind(file_path_id)
            .bind(url.trim())
            .execute(&mut *tx)
            .await
            .change_context(SourceDbError)?;
        }

        tx.commit().await.change_context(SourceDbError)?;

        Ok(())
    }

    async fn get_sources_for_paths(
        &self,
        paths: &[String],
    ) -> Result<HashMap<String, Vec<Source>>, Report<SourceDbError>> {
        if paths.is_empty() {
            return Ok(HashMap::new());
        }

        let placeholders: Vec<String> = paths.iter().map(|_| "?".to_string()).collect();
        let sql = format!(
            "SELECT fp.path, s.id, s.source_url, s.label \
             FROM file_paths fp \
             JOIN sources s ON fp.id = s.file_path_id \
             WHERE fp.path IN ({})",
            placeholders.join(", ")
        );

        let mut query = sqlx::query_as::<_, (String, i64, String, Option<String>)>(&sql);
        for path in paths {
            query = query.bind(path);
        }

        let results = query
            .fetch_all(&self.pool)
            .await
            .change_context(SourceDbError)?;

        let mut map: HashMap<String, Vec<Source>> = HashMap::new();
        for (path, id, source_url, label) in results {
            map.entry(path).or_default().push(Source {
                id,
                source_url,
                label,
            });
        }

        Ok(map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feat::NoteDb;
    use crate::feat::note_db::SqliteNoteDb;
    use crate::test_utils::create_temp_file;

    async fn create_test_db() -> (SqliteSourceDb, SqliteNoteDb) {
        let db = SqliteNoteDb::new("sqlite::memory:").await.unwrap();
        let source_db = SqliteSourceDb::new(db.pool().clone());
        (source_db, db)
    }

    #[tokio::test]
    async fn get_sources_returns_empty_for_new_file() {
        // Given a new database and file.
        let (source_db, note_db) = create_test_db().await;
        let temp = create_temp_file();
        let path = temp.path().to_str().unwrap();
        let file_path_id = note_db.get_or_create_file_path(path).await.unwrap();

        // When getting sources.
        let sources = source_db.get_sources(file_path_id).await.unwrap();

        // Then the result is empty.
        assert!(sources.is_empty());
    }

    #[tokio::test]
    async fn set_sources_inserts_sources() {
        // Given a database with a file.
        let (source_db, note_db) = create_test_db().await;
        let temp = create_temp_file();
        let path = temp.path().to_str().unwrap();
        let file_path_id = note_db.get_or_create_file_path(path).await.unwrap();

        // When setting sources.
        let urls = vec![
            "https://example.com/1".to_string(),
            "https://example.com/2".to_string(),
        ];
        source_db.set_sources(file_path_id, &urls).await.unwrap();

        // Then the sources are stored.
        let sources = source_db.get_sources(file_path_id).await.unwrap();
        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0].source_url, "https://example.com/1");
        assert_eq!(sources[1].source_url, "https://example.com/2");
    }

    #[tokio::test]
    async fn set_sources_replaces_existing() {
        // Given a file with existing sources.
        let (source_db, note_db) = create_test_db().await;
        let temp = create_temp_file();
        let path = temp.path().to_str().unwrap();
        let file_path_id = note_db.get_or_create_file_path(path).await.unwrap();
        source_db
            .set_sources(file_path_id, &["https://old.com".to_string()])
            .await
            .unwrap();

        // When setting new sources.
        source_db
            .set_sources(file_path_id, &["https://new.com".to_string()])
            .await
            .unwrap();

        // Then the old sources are replaced.
        let sources = source_db.get_sources(file_path_id).await.unwrap();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].source_url, "https://new.com");
    }

    #[tokio::test]
    async fn set_sources_filters_empty_urls() {
        // Given a file and URLs with empty strings.
        let (source_db, note_db) = create_test_db().await;
        let temp = create_temp_file();
        let path = temp.path().to_str().unwrap();
        let file_path_id = note_db.get_or_create_file_path(path).await.unwrap();
        let urls = vec![
            "https://example.com".to_string(),
            String::new(),
            "   ".to_string(),
        ];

        // When setting sources with empty URLs.
        source_db.set_sources(file_path_id, &urls).await.unwrap();

        // Then only non-empty URLs are stored.
        let sources = source_db.get_sources(file_path_id).await.unwrap();
        assert_eq!(sources.len(), 1);
    }

    #[tokio::test]
    async fn get_sources_for_paths_returns_map() {
        // Given two files with different sources.
        let (source_db, note_db) = create_test_db().await;
        let temp1 = create_temp_file();
        let temp2 = create_temp_file();
        let path1 = temp1.path().to_str().unwrap().to_string();
        let path2 = temp2.path().to_str().unwrap().to_string();
        let id1 = note_db.get_or_create_file_path(&path1).await.unwrap();
        let id2 = note_db.get_or_create_file_path(&path2).await.unwrap();
        source_db
            .set_sources(id1, &["https://a.com".to_string()])
            .await
            .unwrap();
        source_db
            .set_sources(
                id2,
                &["https://b.com".to_string(), "https://c.com".to_string()],
            )
            .await
            .unwrap();

        // When getting sources for multiple paths.
        let map = source_db
            .get_sources_for_paths(&[path1.clone(), path2.clone()])
            .await
            .unwrap();

        // Then the map contains sources for each path.
        assert_eq!(map.len(), 2);
        assert_eq!(map.get(&path1).unwrap().len(), 1);
        assert_eq!(map.get(&path2).unwrap().len(), 2);
    }

    #[tokio::test]
    async fn get_sources_for_paths_empty_input() {
        // Given a database.
        let (source_db, _) = create_test_db().await;

        // When getting sources for an empty path list.
        let map = source_db.get_sources_for_paths(&[]).await.unwrap();

        // Then the result is empty.
        assert!(map.is_empty());
    }
}
