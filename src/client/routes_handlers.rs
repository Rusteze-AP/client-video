use packet_forge::{ChunkRequest, Index};
use wg_internal::network::SourceRoutingHeader;

use super::Client;

impl Client {
    pub(crate) fn request_video(&self, video_name: &str) {
        let mut state = self.state.write().unwrap();

        let msg = ChunkRequest::new(video_name.to_string(), Index::All);
        let hops = vec![20, 1, 30];
        let dest = hops[1];

        let srh = SourceRoutingHeader::new(hops, 1);

        // Disassemble the message into packets
        let packets = state.packet_forge.disassemble(msg, srh);
        if packets.is_err() {
            eprintln!("Client {}, error: disassembling failed", state.id);
            return;
        }
        let packets = packets.unwrap();

        drop(state);
        let state = self.state.read().unwrap();

        // Get sender
        let sender = state.senders.get(&dest);
        if sender.is_none() {
            eprintln!("Client {}, error: sender {} not found", state.id, dest);
            return;
        }
        let sender = sender.unwrap();

        // Send packets
        for packet in packets {
            sender.send(packet).unwrap();
        }
    }
}
