use packet_forge::SessionIdT;
use wg_internal::packet::{Ack, Packet};

use crate::client::Client;

impl Client {
    pub(crate) fn handle_ack(&self, packet: &Packet, ack: &Ack, session_id: SessionIdT) {
        self.state
            .write()
            .routing_handler
            .nodes_ack(packet.routing_header.clone());

        // Remove packet from history
        let res = self
            .state
            .write()
            .packets_history
            .remove(&(ack.fragment_index, session_id));

        if res.is_none() {
            self.state.read().logger.log_error(&format!(
                "[{}, {}] failed to remove packet_history with id ({}, {})",
                file!(),
                line!(),
                ack.fragment_index,
                session_id
            ));
        }
    }
}
