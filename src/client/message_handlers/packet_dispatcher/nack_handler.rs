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
            state_guard.logger.log_error(&format!(
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
        let sender = if let Some(s) = state_guard.senders.get(&next_hop) {
            s.clone()
        } else {
            state_guard.logger.log_error(&format!(
                "[{}, {}] sender {} not found",
                file!(),
                line!(),
                next_hop
            ));
            return;
        };

        let res = send_packet(state_guard, &sender, &packet);

        if let Err(err) = res {
            state_guard.logger.log_error(err.as_str());
            state_guard.logger.log_error(&format!(
                "[{}, {}] failed to send packet | err: {}",
                file!(),
                line!(),
                err
            ));
        }
    }

    pub(crate) fn handle_nack(
        state_guard: &mut StateGuardWriteT,
        nack: &Nack,
        session_id: SessionIdT,
    ) {
        // Retrieve the packet that generated the nack
        let Some(packet) = state_guard
            .packets_history
            .get(&(nack.fragment_index, session_id))
            .cloned()
        else {
            state_guard.logger.log_error(&format!(
                "[{}, {}] failed to retrieve packet_history with id ({}, {})",
                file!(),
                line!(),
                nack.fragment_index,
                session_id
            ));
            return;
        };

        match nack.nack_type {
            NackType::Dropped => Self::retransmit_packet(state_guard, packet),
            NackType::DestinationIsDrone => {
                state_guard.logger.log_error(&format!(
                    "[{}, {}] received a Nack with DestinationIsDrone",
                    file!(),
                    line!()
                ));
            }
            NackType::ErrorInRouting(id) => {
                state_guard.logger.log_error(&format!(
                    "[{}, {}] received a Nack with ErrorInRouting: {}",
                    file!(),
                    line!(),
                    id
                ));
            }
            NackType::UnexpectedRecipient(id) => {
                state_guard.logger.log_error(&format!(
                    "[{}, {}] received a Nack with UnexpectedRecipient: {}",
                    file!(),
                    line!(),
                    id
                ));
            }
        }
    }
}
