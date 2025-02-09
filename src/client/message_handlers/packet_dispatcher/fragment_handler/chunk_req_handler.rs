use packet_forge::{ChunkRequest, ChunkResponse, MessageType};

use crate::{
    client::{utils::sends::send_msg, video_chunker::get_video_chunks},
    ClientVideo,
};

impl ClientVideo {
    pub(crate) fn handle_chunk_req(&self, content: &ChunkRequest) {
        // Get video from db
        let res = self.db.get_video_content(content.file_hash);
        if let Err(err) = res {
            self.state.read().logger.log_error(&format!(
                "[{}, {}] failed to get video content: {err}",
                file!(),
                line!()
            ));
            return;
        }
        let res = res.unwrap();

        // Split the video into chunks
        let video_chunks = get_video_chunks(res);

        // Get the total number of chunks
        let total_n_chunks = u32::try_from(video_chunks.len());
        if let Err(e) = total_n_chunks {
            self.state.read().logger.log_error(&format!(
                "[{}, {}] failed to convert len to u32: {e:?}",
                file!(),
                line!()
            ));
            return;
        }
        let total_n_chunks = total_n_chunks.unwrap();

        // Send each chunk
        for (i, chunk) in video_chunks.enumerate() {
            let Ok(chunk_index) = u32::try_from(i) else {
                self.state.read().logger.log_error(&format!(
                    "[{}, {}] failed to convert index {} to u32",
                    file!(),
                    line!(),
                    i
                ));
                return;
            };

            // Create ChunkResponse
            let chunk_res = MessageType::ChunkResponse(ChunkResponse::new(
                content.file_hash,
                chunk_index,
                total_n_chunks,
                chunk.clone(),
            ));

            // Send message
            let res = send_msg(&self.state, content.client_id, chunk_res);
            if let Err(err) = res {
                self.state.read().logger.log_error(&err);
            }
        }
    }
}
