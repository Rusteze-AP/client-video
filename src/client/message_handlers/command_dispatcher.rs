use wg_internal::controller::DroneCommand;

use crate::client::{Client, FsmStatus, StateT};

impl Client {
    pub(crate) fn command_dispatcher(state: &StateT, command: &DroneCommand) {
        match command {
            DroneCommand::Crash => {
                state.write().fsm = FsmStatus::Terminated;
            }
            DroneCommand::AddSender(node_id, sender) => {
                state.write().senders.insert(*node_id, sender.clone());
            }
            DroneCommand::RemoveSender(node_id) => {
                let res = state.write().senders.remove(node_id);
                if res.is_none() {
                    state.read().logger.log_error(&format!(
                        "[{}, {}] failed remove, sender {node_id} not found",
                        file!(),
                        line!()
                    ));
                }
            }
            DroneCommand::SetPacketDropRate(_) => {
                state.read().logger.log_error(&format!(
                    "[{}, {}] received a SetPacketDropRate command",
                    file!(),
                    line!()
                ));
            }
        }
    }
}
