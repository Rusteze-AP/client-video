use std::sync::Arc;

use packet_forge::{FileHash, VideoMetaData};
use surrealdb::{engine::local::Db, Surreal};

use super::structures::Video;

pub async fn get_video_list(db: &Arc<Surreal<Db>>) -> surrealdb::Result<Vec<VideoMetaData>> {
    db.query("SELECT VALUE metadata FROM video").await?.take(0)
}

pub async fn get_video_content(
    db: Arc<Surreal<Db>>,
    video_id: FileHash,
) -> surrealdb::Result<Vec<u8>> {
    Ok(db
        .select(("video", video_id.to_string()))
        .await?
        .map(|r: Video| r.content)
        .unwrap_or_default())
}
