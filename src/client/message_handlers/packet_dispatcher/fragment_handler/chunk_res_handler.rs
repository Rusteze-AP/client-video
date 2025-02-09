use bytes::Bytes;
use packet_forge::ChunkResponse;

use crate::ClientVideo;

impl ClientVideo {
    fn send_chunk(&self, data: Bytes) {
        if let Some(sender) = &self.video_sender.read().clone() {
            let _ = sender.send(data);
        } else {
            self.state.read().logger.log_error(&format!(
                "[{}, {}] frontend video sender not found",
                file!(),
                line!()
            ));
        }
    }

    pub(crate) fn handle_chunk_res(&self, content: ChunkResponse) {
        let mut buffer = self.chunk_buffer.write(); // Buffer of out-of-order chunks
        let mut next_index = self.next_expected_index.write();

        if content.chunk_index == *next_index {
            // Send the chunk directly
            self.send_chunk(content.chunk_data);

            // Update expected index and check for buffered chunks
            *next_index += 1;
            while let Some(data) = buffer.remove(&next_index) {
                self.send_chunk(data);
                *next_index += 1;
            }
        } else if content.chunk_index > *next_index {
            // Store out-of-order chunks
            buffer.insert(content.chunk_index, content.chunk_data);
        } else {
            // Duplicate chunk (or old), ignore it
        }
    }
}
