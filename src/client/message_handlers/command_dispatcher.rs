use wg_internal::controller::DroneCommand;

use super::{Client, StateGuardWriteT};

impl Client {
    pub(crate) fn command_dispatcher(state_guard: &mut StateGuardWriteT, command: &DroneCommand) {
        match command {
            DroneCommand::Crash => {
                state_guard.terminated = true;
            }
            DroneCommand::AddSender(node_id, sender) => {
                state_guard.senders.insert(*node_id, sender.clone());
            }
            DroneCommand::RemoveSender(node_id) => {
                let res = state_guard.senders.remove(node_id);
                if res.is_none() {
                    state_guard.logger.log_error(&format!(
                        "[CLIENT {}] on remove sender: sender {} not found",
                        state_guard.id, node_id
                    ));
                }
            }
            DroneCommand::SetPacketDropRate(_) => {
                state_guard.logger.log_warn(&format!(
                    "[CLIENT {}] received a SetPacketDropRate command",
                    state_guard.id
                ));
            }
        }
    }
}
