use std::{path::Path, sync::Arc};

use packet_forge::{Metadata, VideoMetaData};
use surrealdb::{engine::local::Db, Surreal};
use tokio::try_join;

use crate::db::structures::Video;

/// Copy all files from a source directory to a destination directory and create the destination directory if it doesn't exist
pub fn copy_directory(src: &Path, dest: &Path) -> Result<(), std::io::Error> {
    // Create dest dir if it doesn't exist
    if !dest.exists() {
        std::fs::create_dir_all(dest)?;
    }

    // Iterate through src dir
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dest_path = dest.join(src_path.file_name().unwrap());

        if file_type.is_file() {
            // Copy file
            std::fs::copy(&src_path, &dest_path)?;
        } else if file_type.is_dir() {
            // Recursively copy subdirs
            copy_directory(&src_path, &dest_path)?;
        }
    }
    Ok(())
}

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

pub async fn pupulate_db(db: &Arc<Surreal<Db>>, init_db_path: &str) -> surrealdb::Result<()> {
    let gandal_sax_path = format!("{init_db_path}/videos/gandalf_sax.mp4");
    let dancing_pirate_path = format!("{init_db_path}/videos/dancing_pirate.mp4");

    try_join!(
        insert_gandalf_sax(db.clone(), &gandal_sax_path,),
        insert_dancing_pirate(db.clone(), &dancing_pirate_path,)
    )?;

    Ok(())
}
