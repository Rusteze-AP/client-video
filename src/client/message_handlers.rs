mod command_dispatcher;
mod node_messages;
mod packet_dispatcher;

use crossbeam::channel::TryRecvError;
use std::{thread, time::Duration};

use super::{utils::start_flooding::init_flood_request, Client, FsmStatus, FLOODING_TIMER};

impl Client {
    /// Sends a `flood_req` every 60 seconds in a separate thread
    fn start_flooding(&self) {
        let state = self.state.clone();
        thread::spawn(move || loop {
            init_flood_request(&state);
            thread::sleep(Duration::from_secs(FLOODING_TIMER));
        });
    }

    #[must_use]
    pub(crate) fn start_message_processing(self) -> thread::JoinHandle<()> {
        let state = self.state.clone();

        self.start_flooding();

        thread::spawn(move || {
            loop {
                // If the client is terminated, break the loop
                if state.read().fsm == FsmStatus::Terminated {
                    break;
                }

                // If the client is not subscribed to any server, send a subscribe client message
                if state.read().fsm == FsmStatus::ServerNotFound
                    && !state.read().servers_id.is_empty()
                {
                    state.read().logger.log_info(&format!(
                        "[{}, {}] sending subscribe client message",
                        file!(),
                        line!()
                    ));

                    self.send_subscribe_client();
                    state.write().fsm = FsmStatus::NotSubscribedToServer;
                }

                let controller_recv = state.read().controller_recv.clone();
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

                let packet_recv = state.read().packet_recv.clone();
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
