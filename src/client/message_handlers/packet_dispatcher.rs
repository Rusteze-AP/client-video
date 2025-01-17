mod ack_handler;
mod flooding;
mod fragment_handler;
mod nack_handler;

use wg_internal::packet::{Packet, PacketType};

use super::{Client, StateGuardT};

impl Client {
    pub(crate) fn packet_dispatcher(&self, state_guard: &mut StateGuardT, packet: Packet) {
        let session_id = packet.session_id;
        match packet.pack_type {
            PacketType::MsgFragment(frag) => self.handle_fragment(state_guard, frag, session_id),
            PacketType::Ack(ack) => self.handle_ack(state_guard, &ack, session_id),
            PacketType::Nack(nack) => self.handle_nack(state_guard, &nack, session_id),
            PacketType::FloodRequest(flood) => self.handle_flood_req(state_guard, &flood),
            PacketType::FloodResponse(flood) => self.handle_flood_res(state_guard, flood),
        }
    }
}
