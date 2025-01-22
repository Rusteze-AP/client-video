use std::sync::Arc;

use serde::Deserialize;
use surrealdb::{engine::local::Db, Surreal};

use super::structures::VideoMetaData;

pub async fn get_video_list(db: Arc<Surreal<Db>>) -> surrealdb::Result<Vec<VideoMetaData>> {
    let query = r"
        SELECT title, description, duration, mime_type, created_at 
        FROM video
    ";
    db.query(query).await?.take(0)
}

pub async fn get_video_content(db: Arc<Surreal<Db>>, title: &str) -> surrealdb::Result<Vec<u8>> {
    // SurrealDB returns Vec<i64> for binary data
    let result: Option<Vec<i64>> = db
        .query("SELECT content FROM video WHERE title = $title")
        .bind(("title", title.to_string()))
        .await?
        .take(0)?;

    // Convert Vec<i64> to Vec<u8>
    Ok(result
        .map(|vc| vc.into_iter().map(|n| n as u8).collect())
        .unwrap_or_default())
}
