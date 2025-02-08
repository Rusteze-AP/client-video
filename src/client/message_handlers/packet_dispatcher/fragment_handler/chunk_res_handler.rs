use packet_forge::ChunkResponse;

use crate::ClientVideo;

impl ClientVideo {
    pub(crate) fn handle_chunk_res(&self, content: ChunkResponse) {
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
}
