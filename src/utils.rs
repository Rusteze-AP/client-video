use std::sync::Arc;

use surrealdb::{engine::local::Db, Surreal};
use tokio::try_join;

use crate::db::structures::Video;

async fn insert_gandalf_sax(db: Arc<Surreal<Db>>, path_to_video: &str) -> surrealdb::Result<()> {
    let video_content = std::fs::read(path_to_video).expect("Failed to load gandalf video");

    let video = Video {
        id: None,
        title: "gandalf_sax".to_string(),
        description: "gandalf playing sax".to_string(),
        duration: 0.0,
        content: video_content,
        mime_type: "video/mp4".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    let _created: Option<Video> = db
        .create(("video", video.title.clone()))
        .content(video)
        .await?;

    Ok(())
}

async fn insert_dancing_pirate(db: Arc<Surreal<Db>>, path_to_video: &str) -> surrealdb::Result<()> {
    let video_content = std::fs::read(path_to_video).expect("Failed to load gandalf video");

    let video = Video {
        id: None,
        title: "dancing_pirate".to_string(),
        description: "a pirate dancing".to_string(),
        duration: 0.0,
        content: video_content,
        mime_type: "video/mp4".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    let _created: Option<Video> = db
        .create(("video", video.title.clone()))
        .content(video)
        .await?;

    Ok(())
}

pub async fn pupulate_db(db: &Arc<Surreal<Db>>) -> surrealdb::Result<()> {
    try_join!(
        insert_gandalf_sax(
            db.clone(),
            "../client/frontend/public/videos/gandalf_sax.mp4",
        ),
        insert_dancing_pirate(
            db.clone(),
            "../client/frontend/public/videos/dancing_pirate.mp4",
        )
    )?;

    Ok(())
}
