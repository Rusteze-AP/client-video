use packet_forge::{ChunkRequest, Index};
use wg_internal::network::SourceRoutingHeader;

use super::{
    utils::send_packet::{send_packet, send_sc_packet},
    Client,
};

impl Client {
    pub(crate) fn request_video(&self, video_name: &str) {
        let msg = ChunkRequest::new(self.get_id(), video_name.to_string(), Index::All);
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
}
