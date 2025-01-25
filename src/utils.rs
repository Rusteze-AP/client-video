use std::sync::Arc;

use packet_forge::{Metadata, VideoMetaData};
use surrealdb::{engine::local::Db, Surreal};
use tokio::try_join;

use crate::db::structures::Video;

async fn insert_gandalf_sax(db: Arc<Surreal<Db>>, path_to_video: &str) -> surrealdb::Result<()> {
    let content = std::fs::read(path_to_video).expect("Failed to load gandalf_sax video");

    let mut metadata = VideoMetaData {
        id: 0,
        title: "gandalf_sax".to_string(),
        description: "gandalf sax guy".to_string(),
        duration: 0,
        mime_type: "video/mp4".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    metadata.id = metadata.compact_hash_u16();
    let video = Video { metadata, content };

    let _created: Option<Video> = db
        .create(("video", video.metadata.id.to_string()))
        .content(video)
        .await?;

    Ok(())
}

async fn insert_dancing_pirate(db: Arc<Surreal<Db>>, path_to_video: &str) -> surrealdb::Result<()> {
    let content = std::fs::read(path_to_video).expect("Failed to load dancing_pirate video");

    let mut metadata = VideoMetaData {
        id: 0,
        title: "dancing_pirate".to_string(),
        description: "dancing pirate".to_string(),
        duration: 0,
        mime_type: "video/mp4".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    metadata.id = metadata.compact_hash_u16();
    let video = Video { metadata, content };

    let _created: Option<Video> = db
        .create(("video", video.metadata.id.to_string()))
        .content(video)
        .await?;

    Ok(())
}

pub async fn pupulate_db(db: &Arc<Surreal<Db>>) -> surrealdb::Result<()> {
    try_join!(
        insert_gandalf_sax(
            db.clone(),
            "initializations_files/client_video/gandalf_sax.mp4",
        ),
        insert_dancing_pirate(
            db.clone(),
            "initializations_files/client_video/dancing_pirate.mp4",
        )
    )?;

    Ok(())
}
