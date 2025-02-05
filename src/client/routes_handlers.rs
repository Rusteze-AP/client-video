use packet_forge::FileHash;

use crate::db::queries::get_video_content;

use super::{video_chunker::get_video_chunks, Client};

impl Client {
    async fn get_video_from_db(&self, video_id: FileHash) -> Option<()> {
        // Search for the video in the database
        let video_content = get_video_content(&self.db, video_id).await;
        let state_guard = self.state.read();

        match video_content {
            Ok(video_content) => {
                if video_content.is_empty() {
                    state_guard.logger.log_error(&format!(
                        "[{}, {}] video content is empty",
                        file!(),
                        line!()
                    ));
                    return None;
                }

                // Send video chunks to frontend
                if let Some(sender) = self.video_sender.read().clone() {
                    let video_chunks = get_video_chunks(video_content);
                    for chunk in video_chunks {
                        let _ = sender.send(chunk);
                    }
                    return Some(());
                }

                state_guard.logger.log_error(&format!(
                    "[{}, {}] frontend sender not found",
                    file!(),
                    line!()
                ));
            }
            Err(err) => {
                state_guard.logger.log_error(&format!(
                    "[{}, {}] failed to get video content: {err}",
                    file!(),
                    line!()
                ));
            }
        }

        None
    }

    pub(crate) async fn request_video(&self, video_id: FileHash) {
        // Search for the video in the database
        if self.get_video_from_db(video_id).await.is_some() {
            return;
        }

        // If the video is not found in the database, request it from the network
        self.send_req_peer_list(video_id);
    }
}
