use packet_forge::{FileHash, Metadata, VideoMetaData};

pub(crate) struct VideoDb {
    db: sled::Db,
    pub metadata_tree: sled::Tree,
    pub content_tree: sled::Tree,
}

impl VideoDb {
    /// Creates or opens a database at the specified path
    pub fn new(database: &str) -> Self {
        let db = sled::open(database).unwrap_or_else(|e| {
            eprintln!("Error opening database: {e}");
            std::process::exit(1);
        });

        // Helper function to open trees and exit on error
        let open_tree = |tree_name: &str| {
            db.open_tree(tree_name).unwrap_or_else(|e| {
                eprintln!("Error opening {tree_name} tree: {e}");
                std::process::exit(1);
            })
        };

        let metadata_tree = open_tree("metadata");
        let content_tree = open_tree("content");

        Self {
            db,
            metadata_tree,
            content_tree,
        }
    }

    // Clear all entries in the database
    fn clear_database(&self) -> Result<(), String> {
        let trees = [&self.db, &self.metadata_tree, &self.content_tree];

        for tree in trees {
            tree.clear()
                .map_err(|e| format!("Error clearing database: {e}"))?;
            tree.flush()
                .map_err(|e| format!("Error flushing database: {e}"))?;
        }
        Ok(())
    }

    fn load_json_metadata(json_file_path: &str) -> Result<Vec<VideoMetaData>, String> {
        let json_array = "videos";

        let file_content = std::fs::read_to_string(json_file_path)
            .map_err(|e| format!("Error reading file {json_file_path}: {e}"))?;

        let json_data: serde_json::Value =
            serde_json::from_str(&file_content).map_err(|e| format!("Error parsing JSON: {e}"))?;

        let videos_array = json_data[json_array]
            .as_array()
            .ok_or_else(|| format!("Invalid JSON: '{json_array}' is not an array"))?;

        videos_array
            .iter()
            .map(|song| {
                serde_json::from_value(song.clone()).map_err(|e| format!("Invalid song data: {e}"))
            })
            .collect()
    }

    /// Initializes the database:
    /// - clears existing entries
    /// - checks for data from local files
    /// ### Arguments
    /// - `local_path`: folder containing the JSON file
    /// - `file_video_name`: file name with video metadata (.json). If `None`, the database will be empty.
    pub fn init(&self, local_path: &str, file_video_name: Option<&str>) -> Result<(), String> {
        self.clear_database()?;

        if let Some(file_name) = file_video_name {
            let videos_metadata_path = format!("{local_path}/{file_name}");
            let mut videos_array = Self::load_json_metadata(&videos_metadata_path)?;
            self.insert_videos_from_vec(local_path, &mut videos_array)?;
        }

        Ok(())
    }

    /// Insert `VideoMetaData` into `metadata_tree`
    fn insert_video_metadata(
        &self,
        mut file_hash: FileHash,
        file_metadata: &mut VideoMetaData,
    ) -> Result<FileHash, String> {
        // Generate file id
        if file_hash == 0 {
            file_hash = file_metadata.compact_hash_u16();
            file_metadata.id = file_hash;
        }

        let serialized_entry =
            bincode::serialize(&file_metadata).map_err(|e| format!("Serialization error: {e}"))?;
        self.metadata_tree
            .insert(file_hash.to_be_bytes(), serialized_entry)
            .map(|_| file_hash)
            .map_err(|e| format!("Error inserting song metadata: {e}"))
    }

    /// Inserts video content inside `content_tree`
    fn insert_video_content(&self, video_id: FileHash, payload: Vec<u8>) -> Result<(), String> {
        match self.content_tree.insert(video_id.to_be_bytes(), payload) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Error inserting song payload: {e}")),
        }
    }

    /// Insert a vector of `VideoMetaData` inside `metadata_tree`
    fn insert_videos_from_vec(
        &self,
        local_path: &str,
        videos: &mut [VideoMetaData],
    ) -> Result<(), String> {
        for video_metadata in videos.iter_mut() {
            let video_id = self.insert_video_metadata(video_metadata.id, video_metadata)?;

            let video_title_parsed = video_metadata.title.replace(' ', "").to_lowercase();
            let video_file_path = format!("{local_path}/videos/{video_title_parsed}.mp4");

            let video_content = std::fs::read(&video_file_path)
                .map_err(|e| format!("Error reading video file {video_file_path}: {e}"))?;

            self.insert_video_content(video_id, video_content)?;
        }
        Ok(())
    }
}
