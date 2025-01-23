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
                        "[{}, {}] failed remove, sender {node_id} not found",
                        file!(),
                        line!()
                    ));
                }
            }
            DroneCommand::SetPacketDropRate(_) => {
                state_guard.logger.log_warn(&format!(
                    "[{}, {}] received a SetPacketDropRate command",
                    file!(),
                    line!()
                ));
            }
        }
    }
}
