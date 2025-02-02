mod command_dispatcher;
mod packet_dispatcher;

use crossbeam::channel::TryRecvError;
use std::thread;

use super::{utils::start_flooding::init_flood_request, Client};

impl Client {
    fn start_flooding(&self) {
        let state = self.state.clone();
        thread::spawn(move || {
            init_flood_request(&state);
        });
    }

    // async fn send_subscribe_client(state: StateT<'_>, db: Arc<Surreal<Db>>) {
    //     let videos_info = get_video_list(db).await.unwrap_or_default();
    //     let mut available_videos = Vec::new();
    //     for video in videos_info {
    //         available_videos.push(FileMetadata::Video(video));
    //     }

    //     let mut state_guard = state.write();
    //     let msg = SubscribeClient::new(
    //         state_guard.id,
    //         state_guard.client_type.clone(),
    //         available_videos,
    //     );
    //     let hops = vec![20, 1, 30];
    //     let dest = hops[1];
    //     let srh = SourceRoutingHeader::new(hops, 1);

    //     // Disassemble the message into packets
    //     let Ok(packets) = state_guard.packet_forge.disassemble(msg, &srh) else {
    //         state_guard
    //             .logger
    //             .log_error(&format!("[{}, {}] disassemble failed", file!(), line!()));
    //         return;
    //     };

    //     // Get sender
    //     let sender = if let Some(s) = state_guard.senders.get(&dest) {
    //         s.clone()
    //     } else {
    //         state_guard.logger.log_error(&format!(
    //             "[{}, {}] Sender {dest} not found",
    //             file!(),
    //             line!()
    //         ));
    //         return;
    //     };
    //     drop(state_guard);

    //     loop {
    //         let mut state_guard = state.write();
    //         for packet in packets.clone() {
    //             let res = send_packet(&mut state_guard, &sender, &packet);
    //             if let Err(err) = res {
    //                 state_guard.logger.log_error(&format!(
    //                     "[{}, {}] failed send packet: {:?}",
    //                     file!(),
    //                     line!(),
    //                     err.as_str()
    //                 ));
    //             }
    //         }

    //         println!("Sent subscribe client message");

    //         thread::sleep(std::time::Duration::from_secs(1));
    //     }
    // }

    #[must_use]
    pub(crate) fn start_message_processing(self) -> thread::JoinHandle<()> {
        let state = self.state.clone();

        self.start_flooding();
        // tokio::spawn(Self::send_subscribe_client(state.clone(), self.db.clone()));

        thread::spawn(move || {
            loop {
                // Get receivers without holding the lock
                let (controller_recv, packet_recv) = (
                    state.read().controller_recv.clone(),
                    state.read().packet_recv.clone(),
                );

                // If the client is terminated, break the loop
                if state.read().terminated {
                    break;
                }

                match controller_recv.try_recv() {
                    Ok(command) => Self::command_dispatcher(&state, &command),
                    Err(TryRecvError::Empty) => {}
                    Err(e) => {
                        state.read().logger.log_error(&format!(
                            "[{}, {}], error receiving command: {e:?}",
                            file!(),
                            line!()
                        ));
                    }
                }

                match packet_recv.try_recv() {
                    Ok(packet) => self.packet_dispatcher(&packet),
                    Err(TryRecvError::Empty) => {}
                    Err(e) => {
                        state.read().logger.log_error(&format!(
                            "[{}, {}], error receiving packet: {e:?}, ",
                            file!(),
                            line!()
                        ));
                    }
                }
            }
        })
    }
}
