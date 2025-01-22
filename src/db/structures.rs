use packet_forge::VideoMetaData;
use serde::{Deserialize, Serialize};

// #[derive(Debug, Serialize, Deserialize)]
// pub struct Video {
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub id: Option<Thing>,
//     pub title: String,
//     pub description: String,
//     pub duration: f32,
//     pub content: Vec<u8>, // Binary content of the video
//     pub mime_type: String,
//     pub created_at: String,
// }

#[derive(Debug, Serialize, Deserialize)]
pub struct Video {
    pub metadata: VideoMetaData,
    pub content: Vec<u8>,
}

// #[derive(Debug, Serialize, Deserialize)]
// pub struct VideoMetaData {
//     pub title: String,
//     pub description: String,
//     pub duration: f32,
//     pub mime_type: String,
//     pub created_at: String,
// }
