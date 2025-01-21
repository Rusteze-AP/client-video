use packet_forge::{ChunkRequest, Index};
use wg_internal::network::SourceRoutingHeader;

use crate::db::queries::get_video_content;

use super::{
    utils::send_packet::{send_packet, send_sc_packet},
    video_chunker::get_video_chunks,
    Client,
};

impl Client {
    async fn get_video_from_db(&self, video_name: &str) -> Option<()> {
        // Search for the video in the database
        let video_content = get_video_content(self.db.clone(), video_name).await;
        let state_guard = self.state.read().unwrap();

        match video_content {
            Ok(video_content) => {
                if let Some(sender) = &state_guard.video_sender {
                    let mut video_chunks = get_video_chunks(video_content);
                    while let Some(chunk) = video_chunks.next() {
                        let _ = sender.send(chunk);
                    }
                    return Some(());
                } else {
                    state_guard.logger.log_error(&format!(
                        "[CLIENT {}][req_video] frontend sender not found",
                        state_guard.id
                    ));
                }
            }
            Err(err) => {
                state_guard.logger.log_error(&format!(
                    "[CLIENT {}][req_video] failed to get video content: {}",
                    state_guard.id, err
                ));
            }
        }

        None
    }

    pub(crate) fn request_video_from_network(&self, video_name: &str) {
        let msg = ChunkRequest::new(self.get_id(), video_name.to_string() + ".mp4", Index::All);
        let hops = vec![20, 1, 30];
        let dest = hops[1];
        let srh = SourceRoutingHeader::new(hops, 1);

        let (packets, sender) = {
            let mut state_guard = self.state.write().unwrap();

            // Disassemble the message into packets
            let Ok(packets) = state_guard.packet_forge.disassemble(msg, srh) else {
                state_guard.logger.log_error(&format!(
                    "[CLIENT {}][disasembling req_video] failed",
                    state_guard.id
                ));
                return;
            };

            // Get sender
            let sender = if let Some(s) = state_guard.senders.get(&dest) {
                s.clone()
            } else {
                state_guard.logger.log_error(&format!(
                    "[CLIENT {}][req_video] Sender {} not found",
                    state_guard.id, dest
                ));
                return;
            };

            (packets, sender)
        };

        for packet in packets {
            // Send to node
            {
                let mut state_guard = self.state.write().unwrap();
                let res = send_packet(&mut state_guard, &sender, &packet);
                if let Err(err) = res {
                    state_guard.logger.log_error(err.as_str());
                }
            }

            // Send to SC
            {
                let state_guard = self.state.read().unwrap();
                let res = send_sc_packet(&state_guard, &packet);
                if let Err(err) = res {
                    state_guard.logger.log_error(err.as_str());
                }
            }
        }
    }

    pub(crate) async fn request_video(&self, video_name: &str) {
        // Search for the video in the database
        if let Some(_) = self.get_video_from_db(video_name).await {
            return;
        }

        // If the video is not found in the database, request it from the network
        self.request_video_from_network(video_name);
    }
}
