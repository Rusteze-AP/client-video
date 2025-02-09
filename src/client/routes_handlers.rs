use packet_forge::FileHash;

use super::{video_chunker::get_video_chunks, ClientVideo};

impl ClientVideo {
    fn get_video_from_db(&self, video_id: FileHash) -> Option<()> {
        // Search for the video in the database
        let video_content = self.db.get_video_content(video_id);
        let state_guard = self.state.read();

        match video_content {
            Ok(video_content) => {
                if video_content.is_empty() {
                    state_guard.logger.log_warn(&format!(
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
                state_guard.logger.log_warn(&format!(
                    "[{}, {}] failed to get video content from db: {err}",
                    file!(),
                    line!()
                ));
            }
        }

        None
    }

    pub(crate) fn request_video(&self, video_id: FileHash) {
        // Search for the video in the database
        if self.get_video_from_db(video_id).is_some() {
            return;
        }

        // Clear the chunk buffer and reset the next expected index
        self.chunk_buffer.write().clear();
        *self.next_expected_index.write() = 0;

        // If the video is not found in the database, request it from the network
        self.send_req_peer_list(video_id);
    }
}
