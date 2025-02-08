use packet_forge::{ChunkRequest, FileHash, Index, MessageType, ResponsePeerList};
use wg_internal::network::NodeId;

use crate::{client::utils::sends::send_msg, ClientVideo};

impl ClientVideo {
    fn request_video_from_network(&self, video_id: FileHash, dest_id: NodeId) {
        // Create ChunkRequest
        let msg = MessageType::ChunkRequest(ChunkRequest::new(self.get_id(), video_id, Index::All));

        // Send message
        let res = send_msg(&self.state, dest_id, msg);
        if let Err(err) = res {
            self.state.read().logger.log_error(&err);
        }
    }

    pub(crate) fn handle_peer_list_res(&self, content: &ResponsePeerList) {
        if content.peers.is_empty() {
            self.state.read().logger.log_warn(&format!(
                "[{}, {}] peer list is empty",
                file!(),
                line!()
            ));
            return;
        }

        self.request_video_from_network(content.file_hash, content.peers[0].client_id);
    }
}
