mod ack_handler;
mod flooding;
mod fragment_handler;
mod nack_handler;

use wg_internal::packet::{Packet, PacketType};

use crate::client::Client;

impl Client {
    pub(crate) fn packet_dispatcher(&self, packet: &Packet) {
        let session_id = packet.session_id;
        match packet.pack_type.clone() {
            PacketType::MsgFragment(frag) => self.handle_fragment(packet, frag, session_id),
            PacketType::Ack(ack) => self.handle_ack(&ack, session_id),
            PacketType::Nack(nack) => self.handle_nack(&nack, session_id),
            PacketType::FloodRequest(flood) => self.handle_flood_req(&flood),
            PacketType::FloodResponse(flood) => self.handle_flood_res(&flood),
        }
    }
}
