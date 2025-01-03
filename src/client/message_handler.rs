use crossbeam::channel::TryRecvError;
use packet_forge::{MessageType, SessionIdT};
use std::thread;
use wg_internal::controller::DroneCommand;
use wg_internal::packet::{Ack, FloodRequest, FloodResponse, Fragment, Nack, Packet, PacketType};

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

    fn handle_messages(&self, state: &mut StateGuardT, message: MessageType) {
        match message {
            MessageType::SubscribeClient(content) => {
                println!(
                    "Client {} received a SubscribeClient message: {:?}",
                    state.id, content
                );
            }
            MessageType::ChunkResponse(content) => {
                // Send data to event stream
                if let Some(sender) = &state.video_sender {
                    let _ = sender.send(content.chunk_data);
                }
            }
            _ => {
                println!("Client {} received an unimplemented message", state.id);
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

    fn handle_ack(&self, state_guard: &mut StateGuardT, ack: Ack) {
        unimplemented!("Ack")
    }

    fn handle_nack(&self, state_guard: &mut StateGuardT, nack: Nack) {
        unimplemented!("Nack")
    }

    fn handle_flood_req(&self, state_guard: &mut StateGuardT, req: FloodRequest) {
        unimplemented!("FloodRequest")
    }

    fn handle_flood_res(&self, state_guard: &mut StateGuardT, res: FloodResponse) {
        unimplemented!("FloodResponse")
    }

    fn handle_packets(&self, state_guard: &mut StateGuardT, packet: Packet) {
        let session_id = packet.session_id;
        match packet.pack_type {
            PacketType::MsgFragment(frag) => self.handle_fragment(state_guard, frag, session_id),
            PacketType::Ack(ack) => self.handle_ack(state_guard, ack),
            PacketType::Nack(nack) => self.handle_nack(state_guard, nack),
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
                    Ok(command) => {
                        self.command_dispatcher(&mut state_guard, &command);
                    }
                    Err(TryRecvError::Empty) => {}
                    Err(e) => {
                        eprintln!(
                            "Error receiving command for server {}: {:?}",
                            state_guard.id, e
                        );
                    }
                }

                match state_guard.packet_recv.try_recv() {
                    Ok(packet) => {
                        if state_guard.id == 20 {
                            state_guard.id = 69;
                        } else {
                            state_guard.id = 20;
                        }
                        self.handle_packets(&mut state_guard, packet);
                    }
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
