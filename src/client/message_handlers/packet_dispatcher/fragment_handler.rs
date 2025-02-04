use packet_forge::{FileMetadata, MessageType, SessionIdT, VideoMetaData};
use wg_internal::packet::{Fragment, Packet};

use crate::client::{utils::sends::send_ack, Client, FsmStatus};

impl Client {
    fn handle_messages(&self, message: MessageType) {
        match message {
            MessageType::AckSubscribeClient(content) => {
                if content.client_id != self.state.read().id {
                    self.state.read().logger.log_error(&format!(
                        "[{}, {}] client id mismatch",
                        file!(),
                        line!()
                    ));
                    return;
                }

                self.state.read().logger.log_info(&format!(
                    "[{}, {}] received ack subscribe client",
                    file!(),
                    line!()
                ));

                // Once the client is subscribed, set the FSM to running
                self.state.write().fsm = FsmStatus::Running;
            }
            MessageType::ResponseFileList(content) => {
                let fsm_state = self.state.read().fsm.clone();
                if fsm_state == FsmStatus::Idle {
                    self.state.write().fsm = FsmStatus::Running;
                }

                // Convert FileMetadata to VideoMetaData
                let video_list: Vec<VideoMetaData> = content
                    .file_list
                    .iter()
                    .filter_map(|metadata| {
                        if let FileMetadata::Video(video) = metadata {
                            Some(video.clone())
                        } else {
                            None
                        }
                    })
                    .collect();

                // Send video metadata to event stream
                if let Some(sender) = &self.file_list_sender.read().clone() {
                    let _ = sender.send(video_list);
                } else {
                    self.state.read().logger.log_error(&format!(
                        "[{}, {}] frontend file list sender not found",
                        file!(),
                        line!()
                    ));
                }
            }
            MessageType::ChunkResponse(content) => {
                // Send data to event stream
                if let Some(sender) = &self.video_sender.read().clone() {
                    let _ = sender.send(content.chunk_data);
                } else {
                    self.state.read().logger.log_error(&format!(
                        "[{}, {}] frontend video sender not found",
                        file!(),
                        line!()
                    ));
                }
            }
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

            if let Some(fragments) = fragments_clone {
                // Assemble message using cloned fragments
                let assembled = match state.read().packet_forge.assemble_dynamic(fragments) {
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
