use packet_forge::SessionIdT;
use wg_internal::packet::{Nack, NackType, Packet};

use crate::client::{
    utils::{sends::send_packet, start_flooding::init_flood_request},
    ClientVideo, StateT,
};

impl ClientVideo {
    fn retransmit_packet(state: &StateT, mut packet: Packet) {
        let dest = packet.routing_header.hops[packet.routing_header.hops.len() - 1];

        // Retrieve new best path from server to client otherwise return
        let client_id = state.read().id;
        let Some(srh) = state.write().routing_handler.best_path(client_id, dest) else {
            state.read().logger.log_error(&format!(
                "[{}, {}] best path not found from {client_id} to {dest}",
                file!(),
                line!()
            ));
            return;
        };

        let next_hop = srh.hops[srh.hop_index];
        // Assign the new SourceRoutingHeader
        packet.routing_header = srh;

        // Get sender
        let sender = if let Some(s) = state.read().senders.get(&next_hop) {
            s.clone()
        } else {
            state.read().logger.log_error(&format!(
                "[{}, {}] sender {} not found",
                file!(),
                line!(),
                next_hop
            ));
            return;
        };

        let res = send_packet(state, &sender, &packet);
        if let Err(err) = res {
            state.read().logger.log_error(&err);
        }
    }

    pub(crate) fn handle_nack(&self, nack: &Nack, session_id: SessionIdT) {
        let state = &self.state;

        // Retrieve the packet that generated the nack
        let Some(packet) = state
            .read()
            .packets_history
            .get(&(nack.fragment_index, session_id))
            .cloned()
        else {
            state.read().logger.log_error(&format!(
                "[{}, {}] failed to retrieve packet_history with id ({}, {})",
                file!(),
                line!(),
                nack.fragment_index,
                session_id
            ));
            return;
        };

        match nack.nack_type {
            NackType::Dropped => {
                // Update the routing handler
                self.state
                    .write()
                    .routing_handler
                    .node_nack(packet.routing_header.hops[0]);

                Self::retransmit_packet(state, packet);
            }
            NackType::ErrorInRouting(id) => {
                state.read().logger.log_error(&format!(
                    "[{}, {}] received a Nack with ErrorInRouting: {}",
                    file!(),
                    line!(),
                    id
                ));

                // Remove the server from the list of servers
                state.write().servers_id.retain(|&x| x != id);

                init_flood_request(&state);
            }
            NackType::DestinationIsDrone => {
                state.read().logger.log_error(&format!(
                    "[{}, {}] received a Nack with DestinationIsDrone",
                    file!(),
                    line!()
                ));
            }
            NackType::UnexpectedRecipient(id) => {
                state.read().logger.log_error(&format!(
                    "[{}, {}] received a Nack with UnexpectedRecipient: {}",
                    file!(),
                    line!(),
                    id
                ));
            }
        }
    }
}
