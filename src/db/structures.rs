use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

#[derive(Debug, Serialize, Deserialize)]
pub struct Video {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Thing>,
    pub title: String,
    pub description: String,
    pub duration: f32,
    pub content: Vec<u8>, // Binary content of the video
    pub mime_type: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VideoMetaData {
    pub title: String,
    pub description: String,
    pub duration: f32,
    pub mime_type: String,
    pub created_at: String,
}
