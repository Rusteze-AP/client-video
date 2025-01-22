use std::sync::Arc;

use packet_forge::{FileHash, VideoMetaData};
use surrealdb::{engine::local::Db, Surreal};

use super::structures::Video;

pub async fn get_video_list(db: Arc<Surreal<Db>>) -> surrealdb::Result<Vec<VideoMetaData>> {
    db.query("SELECT VALUE metadata FROM video").await?.take(0)
}

// pub async fn get_video_content(db: Arc<Surreal<Db>>, title: u16) -> surrealdb::Result<Vec<u8>> {
//     // SurrealDB returns Vec<i64> for binary data
//     let result: Option<Vec<i64>> = db
//         .query("SELECT VALUE content FROM video WHERE title = $title")
//         .bind(("title", title.to_string()))
//         .await?
//         .take(0)?;

//     // Convert Vec<i64> to Vec<u8>
//     Ok(result
//         .map(|vc| vc.into_iter().map(|n| n as u8).collect())
//         .unwrap_or_default())
// }

pub async fn get_video_content(
    db: Arc<Surreal<Db>>,
    video_id: FileHash,
) -> surrealdb::Result<Vec<u8>> {
    let result: Option<Video> = db.select(("video", video_id.to_string())).await?;

    // keep only content and convert to Vec<u8>
    Ok(result
        .map(|r| r.content)
        .map(|vc| vc.into_iter().map(|n| n as u8).collect())
        .unwrap_or_default())
}
