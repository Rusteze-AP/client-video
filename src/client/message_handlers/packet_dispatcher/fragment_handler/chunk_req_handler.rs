use packet_forge::{ChunkRequest, ChunkResponse, MessageType};

use crate::{
    client::{utils::sends::send_msg, video_chunker::get_video_chunks, RT},
    db::queries::get_video_content,
    Client,
};

impl Client {
    pub(crate) fn handle_chunk_req(&self, content: &ChunkRequest) {
        // Get video from db
        let res = RT.block_on(get_video_content(&self.db, content.file_hash));
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
