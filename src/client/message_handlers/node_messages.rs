use packet_forge::{
    FileHash, FileMetadata, MessageType, RequestFileList, RequestPeerList, SubscribeClient,
};
use wg_internal::network::NodeId;

use crate::client::{utils::sends::send_msg, ClientVideo};

impl ClientVideo {
    pub(crate) fn send_subscribe_client(&self, dest_id: NodeId) {
        // Get available videos from db
        let videos_info = self.db.get_video_list();

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
        // let dest_id = self.state.read().servers_id[0];

        // Send message
        let res = send_msg(&self.state, dest_id, msg);
        if let Err(err) = res {
            self.state.read().logger.log_error(&err);
        }
    }

    pub(crate) fn send_req_file_list(&self) {
        // Create a RequestFileList message
        let msg = MessageType::RequestFileList(RequestFileList::new(self.get_id()));

        // Check if there are servers available
        if self.state.read().servers.is_empty() {
            self.state.read().logger.log_error(&format!(
                "[{}, {}] No servers available",
                file!(),
                line!()
            ));
            return;
        }

        // Send request to all servers
        let servers = self.state.read().servers.clone();
        for dest_id in servers.keys() {
            // Send message
            let res = send_msg(&self.state, *dest_id, msg.clone());
            if let Err(err) = res {
                self.state.read().logger.log_error(&err);
            }
        }
    }

    pub(crate) fn send_req_peer_list(&self, video_id: FileHash) {
        // Create RequestPeerList
        let msg = MessageType::RequestPeerList(RequestPeerList::new(self.get_id(), video_id));

        // Check if the video_id is available in any server
        let servers = self.state.read().servers.clone();
        for server in &servers {
            if server.1.contains(&video_id) {
                // Send message
                let res = send_msg(&self.state, *server.0, msg);
                if let Err(err) = res {
                    self.state.read().logger.log_error(&err);
                }
                return;
            }
        }

        // Log error if video_id is not found in any server
        self.state.read().logger.log_error(&format!(
            "[{}, {}] video_id {} not found in servers",
            file!(),
            line!(),
            video_id
        ));
    }
}
