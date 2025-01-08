use packet_forge::{ChunkRequest, Index};
use wg_internal::network::SourceRoutingHeader;

use super::{utils::send_packet, Client};

impl Client {
    pub(crate) fn request_video(&self, video_name: &str) {
        let msg = ChunkRequest::new(video_name.to_string(), Index::All);
        let hops = vec![20, 1, 30];
        let dest = hops[1];
        let srh = SourceRoutingHeader::new(hops, 1);

        let (packets, sender, client_id) = {
            let mut state_guard = self.state.write().unwrap();

            // Disassemble the message into packets
            let Ok(packets) = state_guard.packet_forge.disassemble(msg, srh) else {
                eprintln!("Client {}, error: disassembling failed", state_guard.id);
                return;
            };

            // Get sender
            let sender = if let Some(s) = state_guard.senders.get(&dest) {
                s.clone()
            } else {
                eprintln!(
                    "Client {}, error: sender {} not found",
                    state_guard.id, dest
                );
                return;
            };

            (packets, sender, state_guard.id)
        };

        for packet in packets {
            let mut state_guard = self.state.write().unwrap();
            send_packet(&mut state_guard, &sender, packet, client_id);
        }
    }
}
