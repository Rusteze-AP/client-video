use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use wg_internal::network::NodeId;

pub trait Serializable: Serialize + DeserializeOwned {
    fn stringify(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
    fn from_string(raw: String) -> Result<Self, String> {
        serde_json::from_str(raw.as_str()).map_err(|e| e.to_string())
    }
}

pub struct Message<M: Serializable> {
    pub source_id: NodeId,
    pub session_id: u64,
    pub content: M,
}
