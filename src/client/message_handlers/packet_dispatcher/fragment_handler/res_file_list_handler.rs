use packet_forge::{FileMetadata, ResponseFileList, VideoMetaData};

use crate::{client::FsmStatus, ClientVideo};

impl ClientVideo {
    pub(crate) fn handle_response_file_list(&self, content: &ResponseFileList) {
        let fsm_state = self.state.read().fsm.clone();
        if fsm_state == FsmStatus::NotSubscribedToServer {
            self.state.write().fsm = FsmStatus::SubscribedToServer;
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

        // Add video ids to the server id map
        let video_ids: Vec<u16> = video_list.iter().map(|video| video.id).collect();
        self.state
            .write()
            .servers
            .insert(content.server_id, video_ids);

        // Send video metadata to event stream
        if let Some(sender) = &self.file_list_sender.read().clone() {
            let _ = sender.send((content.server_id, video_list));
        } else {
            self.state.read().logger.log_warn(&format!(
                "[{}, {}] frontend file list sender not found",
                file!(),
                line!()
            ));
        }
    }
}
