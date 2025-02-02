use packet_forge::{ChunkRequest, FileHash, Index};
use wg_internal::{controller::DroneEvent, network::SourceRoutingHeader};

use crate::db::queries::get_video_content;

use super::{
    utils::sends::{send_packet, send_sc_packet},
    video_chunker::get_video_chunks,
    Client,
};

impl Client {
    async fn get_video_from_db(&self, video_id: FileHash) -> Option<()> {
        // Search for the video in the database
        let video_content = get_video_content(self.db.clone(), video_id).await;
        let state_guard = self.state.read();

        match video_content {
            Ok(video_content) => {
                if video_content.is_empty() {
                    state_guard.logger.log_error(&format!(
                        "[{}, {}] video content is empty",
                        file!(),
                        line!()
                    ));
                    return None;
                }

                // Send video chunks to frontend
                if let Some(sender) = self.video_sender.read().clone() {
                    let video_chunks = get_video_chunks(video_content);
                    for chunk in video_chunks {
                        let _ = sender.send(chunk);
                    }
                    return Some(());
                }

                state_guard.logger.log_error(&format!(
                    "[{}, {}] frontend sender not found",
                    file!(),
                    line!()
                ));
            }
            Err(err) => {
                state_guard.logger.log_error(&format!(
                    "[{}, {}] failed to get video content: {err}",
                    file!(),
                    line!()
                ));
            }
        }

        None
    }

    pub(crate) fn request_video_from_network(&self, video_id: FileHash) {
        let msg = ChunkRequest::new(self.get_id(), video_id, Index::All);
        let hops = vec![20, 1, 30];
        let dest = hops[1];
        let srh = SourceRoutingHeader::new(hops, 1);

        let (packets, sender) = {
            let mut state_guard = self.state.write();

            // Disassemble the message into packets
            let Ok(packets) = state_guard.packet_forge.disassemble(msg, &srh) else {
                state_guard.logger.log_error(&format!(
                    "[{}, {}] disassemble failed",
                    file!(),
                    line!()
                ));
                return;
            };

            // Get sender
            let sender = if let Some(s) = state_guard.senders.get(&dest) {
                s.clone()
            } else {
                state_guard.logger.log_error(&format!(
                    "[{}, {}] Sender {dest} not found",
                    file!(),
                    line!()
                ));
                return;
            };

            (packets, sender)
        };

        for packet in packets {
            // Send to node
            let res = send_packet(&self.state, &sender, &packet);
            if let Err(err) = res {
                self.state.read().logger.log_error(&format!(
                    "[{}, {}] failed send packet: {:?}",
                    file!(),
                    line!(),
                    err.as_str()
                ));
            }

            // Send to SC
            let res = send_sc_packet(&self.state, &DroneEvent::PacketSent(packet));
            if let Err(err) = res {
                self.state.read().logger.log_error(&format!(
                    "[{}, {}] failed send sc packet: {:?}",
                    file!(),
                    line!(),
                    err.as_str()
                ));
            }
        }
    }

    pub(crate) async fn request_video(&self, video_id: FileHash) {
        // Search for the video in the database
        if self.get_video_from_db(video_id).await.is_some() {
            return;
        }

        // If the video is not found in the database, request it from the network
        self.request_video_from_network(video_id);
    }
}
