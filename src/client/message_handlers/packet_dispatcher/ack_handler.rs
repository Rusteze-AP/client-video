use packet_forge::SessionIdT;
use wg_internal::packet::Ack;

use super::{Client, StateGuardWriteT};

impl Client {
    pub(crate) fn handle_ack(
        state_guard: &mut StateGuardWriteT,
        ack: &Ack,
        session_id: SessionIdT,
    ) {
        // Remove packet from history
        let res = state_guard
            .packets_history
            .remove(&(ack.fragment_index, session_id));
        if res.is_none() {
            state_guard.logger.log_error(&format!(
                "[{}, {}] failed to remove packet_history with id ({}, {})",
                file!(),
                line!(),
                ack.fragment_index,
                session_id
            ));
        }
    }
}
