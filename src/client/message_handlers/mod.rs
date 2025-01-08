mod command_dispatcher;
mod packet_dispatcher;

use crossbeam::channel::TryRecvError;
use std::thread;

use super::{Client, StateGuardT};

impl Client {
    #[must_use]
    pub(crate) fn start_message_processing(self) -> thread::JoinHandle<()> {
        let state = self.state.clone();

        thread::spawn(move || {
            loop {
                // Get mutable access to state
                let mut state_guard = state.write().unwrap();

                if state_guard.terminated {
                    break;
                }

                match state_guard.controller_recv.try_recv() {
                    Ok(command) => self.command_dispatcher(&mut state_guard, &command),
                    Err(TryRecvError::Empty) => {}
                    Err(e) => {
                        eprintln!(
                            "Error receiving command for server {}: {:?}",
                            state_guard.id, e
                        );
                    }
                }

                match state_guard.packet_recv.try_recv() {
                    Ok(packet) => self.packet_dispatcher(&mut state_guard, packet),
                    Err(TryRecvError::Empty) => {}
                    Err(e) => {
                        eprintln!(
                            "Error receiving message for server {}: {:?}",
                            state_guard.id, e
                        );
                    }
                }

                // RwLock is automatically released here when state_guard goes out of scope
            }
        })
    }
}
