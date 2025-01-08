use crossbeam::channel::TryRecvError;
use packet_forge::{MessageType, SessionIdT};
use std::thread;
use wg_internal::controller::DroneCommand;
use wg_internal::packet::{
    Ack, FloodRequest, FloodResponse, Fragment, Nack, NackType, Packet, PacketType,
};

use super::utils::send_packet;
use super::{Client, StateGuardT};

impl Client {
    fn command_dispatcher(&self, state: &mut StateGuardT, command: &DroneCommand) {
        match command {
            DroneCommand::Crash => {
                state.terminated = true;
            }
            DroneCommand::SetPacketDropRate(_) => {
                eprintln!(
                    "Client {}, error: received a SetPacketDropRate command",
                    state.id
                );
            }
            _ => {
                eprintln!(
                    "Client {}, error: received an unknown command: {:?}",
                    state.id, command
                );
            }
        }
    }

    fn handle_messages(&self, state_guard: &mut StateGuardT, message: MessageType) {
        match message {
            MessageType::SubscribeClient(content) => {
                println!(
                    "Client {} received a SubscribeClient message: {:?}",
                    state_guard.id, content
                );
            }
            MessageType::ChunkResponse(content) => {
                // Send data to event stream
                if let Some(sender) = &state_guard.video_sender {
                    let _ = sender.send(content.chunk_data);
                }
            }
            _ => {
                println!(
                    "Client {} received an unimplemented message",
                    state_guard.id
                );
            }
        }
    }

    fn handle_fragment(
        &self,
        state_guard: &mut StateGuardT,
        frag: Fragment,
        session_id: SessionIdT,
    ) {
        // Add fragment to packets_map
        state_guard
            .packets_map
            .entry(session_id)
            .or_default()
            .push(frag);
        let fragments = state_guard.packets_map.get(&session_id).unwrap();
        let total_fragments = fragments[0].total_n_fragments;

        // If all fragments are received, assemble the message
        if fragments.len() as u64 == total_fragments {
            let assembled = match state_guard.packet_forge.assemble_dynamic(fragments.clone()) {
                Ok(message) => message,
                Err(e) => panic!("Error assembling: {e}"),
            };
            state_guard.packets_map.remove(&session_id);
            self.handle_messages(state_guard, assembled);
        }
    }

    fn retransmit_packet(&self, state_guard: &mut StateGuardT, mut packet: Packet) {
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

        send_packet(state_guard, &sender, packet, state_guard.id);
    }

    fn handle_ack(&self, state_guard: &mut StateGuardT, ack: Ack, session_id: SessionIdT) {
        // Remove packet from history
        let res = state_guard
            .packets_history
            .remove(&(ack.fragment_index, session_id));
        if res.is_none() {
            eprintln!(
                "Client {}, failed to remove packet_history with id ({}, {})",
                state_guard.id, ack.fragment_index, session_id
            );
        }
    }

    fn handle_nack(&self, state_guard: &mut StateGuardT, nack: Nack, session_id: SessionIdT) {
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
            NackType::Dropped => self.retransmit_packet(state_guard, packet),
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

    fn handle_flood_req(&self, state_guard: &mut StateGuardT, req: FloodRequest) {
        unimplemented!("FloodRequest")
    }

    fn handle_flood_res(&self, state_guard: &mut StateGuardT, flood: FloodResponse) {
        state_guard.routing_handler.update_graph(flood);
    }

    fn packet_dispatcher(&self, state_guard: &mut StateGuardT, packet: Packet) {
        let session_id = packet.session_id;
        match packet.pack_type {
            PacketType::MsgFragment(frag) => self.handle_fragment(state_guard, frag, session_id),
            PacketType::Ack(ack) => self.handle_ack(state_guard, ack, session_id),
            PacketType::Nack(nack) => self.handle_nack(state_guard, nack, session_id),
            PacketType::FloodRequest(flood) => self.handle_flood_req(state_guard, flood),
            PacketType::FloodResponse(flood) => self.handle_flood_res(state_guard, flood),
        }
    }

    #[must_use]
    pub(crate) fn start_message_processing(self) -> thread::JoinHandle<()> {
        let state = self.state.clone();

        thread::spawn(move || {
            loop {
                // Get mutable access to state
                let mut state_guard = state.write().unwrap();

                if state_guard.terminated {
                    break;
                }

                match state_guard.controller_recv.try_recv() {
                    Ok(command) => self.command_dispatcher(&mut state_guard, &command),
                    Err(TryRecvError::Empty) => {}
                    Err(e) => {
                        eprintln!(
                            "Error receiving command for server {}: {:?}",
                            state_guard.id, e
                        );
                    }
                }

                match state_guard.packet_recv.try_recv() {
                    Ok(packet) => self.packet_dispatcher(&mut state_guard, packet),
                    Err(TryRecvError::Empty) => {}
                    Err(e) => {
                        eprintln!(
                            "Error receiving message for server {}: {:?}",
                            state_guard.id, e
                        );
                    }
                }

                // RwLock is automatically released here when state_guard goes out of scope
            }
        })
    }
}
