mod ack_handler;
mod flooding;
mod fragment_handler;
mod nack_handler;

use wg_internal::packet::{Packet, PacketType};

use crate::client::ClientVideo;

impl ClientVideo {
    pub(crate) fn packet_dispatcher(&self, packet: &Packet) {
        if let PacketType::FloodRequest(flood_req) = &packet.pack_type {
            self.handle_flood_req(flood_req);
            return;
        }

        // Update routing_handler
        self.state
            .write()
            .routing_handler
            .nodes_congestion(packet.routing_header.clone());

        let session_id = packet.session_id;
        match packet.pack_type.clone() {
            PacketType::MsgFragment(frag) => self.handle_fragment(packet, frag, session_id),
            PacketType::Ack(ack) => self.handle_ack(packet, &ack, session_id),
            PacketType::Nack(nack) => self.handle_nack(&nack, session_id),
            PacketType::FloodRequest(flood_req) => {
                // Should not get here, but just in case
                self.handle_flood_req(&flood_req);
            }
            PacketType::FloodResponse(flood_res) => self.handle_flood_res(&flood_res),
        }
    }
}
