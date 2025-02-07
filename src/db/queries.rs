use packet_forge::{FileHash, VideoMetaData};

use super::structures::VideoDb;

impl VideoDb {
    pub(crate) fn get_video_list(&self) -> Vec<VideoMetaData> {
        self.metadata_tree
            .iter()
            .filter_map(|entry| {
                if let Ok((_, data)) = entry {
                    // Attempt to deserialize the data
                    return bincode::deserialize::<VideoMetaData>(&data).ok();
                }

                None
            })
            .collect()
    }

    /// Retrieves video payload from the database by ID.
    pub(crate) fn get_video_content(&self, id: FileHash) -> Result<Vec<u8>, String> {
        self.content_tree
            .get(id.to_be_bytes())
            .map_err(|e| format!("Error accessing database: {e}"))?
            .map(|data| data.to_vec())
            .ok_or_else(|| "Video payload not found".to_string())
    }
}
