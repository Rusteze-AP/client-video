use crossbeam::channel::TryRecvError;
use packet_forge::MessageType;
use std::sync::RwLockWriteGuard;
use std::thread;
use wg_internal::controller::DroneCommand;
use wg_internal::packet::{Packet, PacketType};

use super::{Client, ClientState};

impl Client {
    fn command_dispatcher(
        &self,
        state: &mut RwLockWriteGuard<'_, ClientState>,
        command: &DroneCommand,
    ) {
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

    fn handle_messages(&self, state: &mut RwLockWriteGuard<'_, ClientState>, message: MessageType) {
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

    fn handle_packets(&self, state: &mut RwLockWriteGuard<'_, ClientState>, packet: Packet) {
        let session_id = packet.session_id;
        match packet.pack_type {
            PacketType::MsgFragment(frag) => {
                // Add fragment to packets_map
                state.packets_map.entry(session_id).or_default().push(frag);
                let fragments = state.packets_map.get(&session_id).unwrap();
                let total_fragments = fragments[0].total_n_fragments;

                // If all fragments are received, assemble the message
                if fragments.len() as u64 == total_fragments {
                    let assembled = match state.packet_forge.assemble_dynamic(fragments.clone()) {
                        Ok(message) => message,
                        Err(e) => panic!("Error assembling: {e}"),
                    };
                    state.packets_map.remove(&session_id);
                    self.handle_messages(state, assembled);
                }
            }
            _ => {
                println!(
                    "Client {} received an unimplemented packet: {:?}",
                    state.id, packet
                );
            }
        }
    }

    #[must_use]
    pub(crate) fn start_message_processing(self) -> thread::JoinHandle<()> {
        let state = self.state.clone();

        thread::spawn(move || {
            loop {
                // thread::sleep(Duration::from_secs(1));

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
