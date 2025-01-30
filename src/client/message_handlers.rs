mod command_dispatcher;
mod packet_dispatcher;

use crossbeam::channel::TryRecvError;
use std::thread;

use super::{utils::start_flooding::init_flood_request, Client, StateGuardWriteT, StateT};

impl Client {
    fn start_flooding(state: StateT) {
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

        Self::start_flooding(state.clone());
        // tokio::spawn(Self::send_subscribe_client(state.clone(), self.db.clone()));

        thread::spawn(move || {
            loop {
                // Get mutable access to state
                let mut state_guard = state.write();

                if state_guard.terminated {
                    break;
                }

                match state_guard.controller_recv.try_recv() {
                    Ok(command) => Self::command_dispatcher(&mut state_guard, &command),
                    Err(TryRecvError::Empty) => {}
                    Err(e) => {
                        state_guard.logger.log_error(&format!(
                            "[{}, {}], error receiving command: {e:?}",
                            file!(),
                            line!()
                        ));
                    }
                }

                match state_guard.packet_recv.try_recv() {
                    Ok(packet) => Self::packet_dispatcher(&mut state_guard, packet),
                    Err(TryRecvError::Empty) => {}
                    Err(e) => {
                        state_guard.logger.log_error(&format!(
                            "[{}, {}], error receiving packet: {e:?}, ",
                            file!(),
                            line!()
                        ));
                    }
                }

                // RwLock is automatically released here when state_guard goes out of scope
            }
        })
    }
}
