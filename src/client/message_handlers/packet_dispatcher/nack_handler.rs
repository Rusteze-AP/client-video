use packet_forge::SessionIdT;
use wg_internal::packet::{Nack, NackType, Packet};

use crate::client::utils::send_packet::send_packet;

use super::{Client, StateGuardWriteT};

impl Client {
    fn retransmit_packet(state_guard: &mut StateGuardWriteT, mut packet: Packet) {
        let dest = packet.routing_header.hops[packet.routing_header.hops.len()];

        // Retrieve new best path from server to client otherwise return
        let client_id = state_guard.id;
        let Some(srh) = state_guard.routing_handler.best_path(client_id, dest) else {
            eprintln!(
                "Client {}, error: best path not found from {} to {}",
                state_guard.id, client_id, dest
            );
            return;
        };

        let next_hop = srh.hops[srh.hop_index];
        // Assign the new SourceRoutingHeader
        packet.routing_header = srh;

        // Get sender
        let sender = if let Some(s) = state_guard.senders.get(&next_hop) {
            s.clone()
        } else {
            eprintln!(
                "Client {}, error: sender {} not found",
                state_guard.id, next_hop
            );
            return;
        };

        let res = send_packet(state_guard, &sender, &packet);

        if let Err(err) = res {
            state_guard.logger.log_error(err.as_str());
        }
    }

    pub(crate) fn handle_nack(state_guard: &mut StateGuardWriteT, nack: &Nack, session_id: SessionIdT) {
        // Retrieve the packet that generated the nack
        let Some(packet) = state_guard
            .packets_history
            .get(&(nack.fragment_index, session_id))
            .cloned()
        else {
            eprintln!(
                "Client {}, failed to retrieve packet_history with id ({}, {})",
                state_guard.id, nack.fragment_index, session_id
            );
            return;
        };

        match nack.nack_type {
            NackType::Dropped => Self::retransmit_packet(state_guard, packet),
            NackType::DestinationIsDrone => {
                eprintln!(
                    "Client {}, received a Nack with DestinationIsDrone",
                    state_guard.id
                );
            }
            NackType::ErrorInRouting(id) => {
                eprintln!(
                    "Client {}, received a Nack with ErrorInRouting: {}",
                    state_guard.id, id
                );
            }
            NackType::UnexpectedRecipient(id) => {
                eprintln!(
                    "Client {}, received a Nack with UnexpectedRecipient: {}",
                    state_guard.id, id
                );
            }
        }
    }
}
