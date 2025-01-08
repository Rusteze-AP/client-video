use wg_internal::controller::DroneCommand;

use super::{Client, StateGuardT};

impl Client {
    pub(crate) fn command_dispatcher(&self, state: &mut StateGuardT, command: &DroneCommand) {
        match command {
            DroneCommand::Crash => {
                state.terminated = true;
            }
            DroneCommand::SetPacketDropRate(_) => {
                eprintln!(
                    "Client {}, error: received a SetPacketDropRate command",
                    state.id
                );
            }
            _ => {
                eprintln!(
                    "Client {}, error: received an unknown command: {:?}",
                    state.id, command
                );
            }
        }
    }
}
