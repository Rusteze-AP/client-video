use packet_forge::{
    FileHash, FileMetadata, MessageType, RequestFileList, RequestPeerList, SubscribeClient,
};

use crate::{
    client::{utils::sends::send_msg, Client, DbT},
    db::queries::get_video_list,
};

impl Client {
    pub(crate) async fn send_subscribe_client(&self, db: &DbT) {
        // Get available videos from db
        let videos_info = get_video_list(db).await.unwrap_or_default();

        // Create a vec of FileMetadata::Video
        let mut available_videos = Vec::new();
        for video in videos_info {
            available_videos.push(FileMetadata::Video(video));
        }

        // Create a SubscribeClient message
        let msg = MessageType::SubscribeClient(SubscribeClient::new(
            self.state.read().id,
            self.state.read().client_type.clone(),
            available_videos,
        ));

        // Get source and destination id
        let dest_id = self.state.read().servers_id[0];

        // Send message
        let res = send_msg(&self.state, dest_id, msg);
        if let Err(err) = res {
            self.state.read().logger.log_error(&err);
        }
    }

    pub(crate) fn send_req_file_list(&self) {
        // Create a RequestFileList message
        let msg = MessageType::RequestFileList(RequestFileList::new(self.get_id()));

        if self.state.read().servers_id.is_empty() {
            self.state.read().logger.log_error(&format!(
                "[{}, {}] No servers available",
                file!(),
                line!()
            ));
            return;
        }
        let dest_id = self.state.read().servers_id[0];

        // Send message
        let res = send_msg(&self.state, dest_id, msg);
        if let Err(err) = res {
            self.state.read().logger.log_error(&err);
        }
    }

    pub(crate) fn send_req_peer_list(&self, video_id: FileHash) {
        // Create RequestPeerList
        let msg = MessageType::RequestPeerList(RequestPeerList::new(self.get_id(), video_id));
        let dest_id = self.state.read().servers_id[0];

        // Send message
        let res = send_msg(&self.state, dest_id, msg);
        if let Err(err) = res {
            self.state.read().logger.log_error(&err);
        }
    }
}
