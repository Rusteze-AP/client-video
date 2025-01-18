use packet_forge::{MessageType, SessionIdT};
use wg_internal::packet::Fragment;

use super::{Client, StateGuardWriteT};

impl Client {
    fn handle_messages(state_guard: &mut StateGuardWriteT, message: MessageType) {
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

    pub(crate) fn handle_fragment(
        state_guard: &mut StateGuardWriteT,
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
            Self::handle_messages(state_guard, assembled);
        }
    }
}
