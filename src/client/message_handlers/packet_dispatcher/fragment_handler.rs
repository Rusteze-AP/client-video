mod chunk_req_handler;
mod chunk_res_handler;
mod res_file_list_handler;
mod res_peer_list_handler;

use packet_forge::{MessageType, SessionIdT};
use wg_internal::packet::{Fragment, Packet};

use crate::client::{utils::sends::send_ack, ClientVideo};

impl ClientVideo {
    fn handle_messages(&self, message: MessageType) {
        match message {
            MessageType::ResponseFileList(content) => self.handle_response_file_list(&content),
            MessageType::ChunkResponse(content) => self.handle_chunk_res(content),
            MessageType::ChunkRequest(content) => self.handle_chunk_req(&content),
            MessageType::ResponsePeerList(content) => self.handle_peer_list_res(&content),
            _ => {
                self.state.read().logger.log_error(&format!(
                    "[{}, {}] message not handled: {:?}",
                    file!(),
                    line!(),
                    message
                ));
            }
        }
    }

    pub(crate) fn handle_fragment(&self, packet: &Packet, frag: Fragment, session_id: SessionIdT) {
        let state = &self.state;

        // Add fragment to packets_map
        state
            .write()
            .packets_map
            .entry(session_id)
            .or_default()
            .push(frag);

        // Send an ack to the sender
        let res = send_ack(state, packet);
        if let Err(err) = res {
            state.read().logger.log_error(&format!(
                "[{}, {}] failed to send ack: {:?}",
                file!(),
                line!(),
                err
            ));
        }

        let should_assemble = {
            let state_guard = state.read();
            if let Some(fragments) = state_guard.packets_map.get(&session_id) {
                let total_expected = fragments[0].total_n_fragments;
                fragments.len() as u64 == total_expected
            } else {
                false
            }
        };

        // If all fragments are received, assemble the message
        if should_assemble {
            // Clone fragments
            let fragments_clone = state.read().packets_map.get(&session_id).cloned();

            if let Some(mut fragments) = fragments_clone {
                // Assemble message using cloned fragments
                let assembled = match state.read().packet_forge.assemble_dynamic(&mut fragments) {
                    Ok(message) => message,
                    Err(e) => {
                        state.read().logger.log_error(&format!(
                            "[{}, {}] failed to assemble message: {:?}",
                            file!(),
                            line!(),
                            e
                        ));
                        return;
                    }
                };

                // Remove assembled fragments and handle the message
                state.write().packets_map.remove(&session_id);
                self.handle_messages(assembled);
            }
        }
    }
}
