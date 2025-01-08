use wg_internal::packet::{FloodRequest, FloodResponse};

use super::{Client, StateGuardT};

impl Client {
    pub(crate) fn handle_flood_req(&self, state_guard: &mut StateGuardT, req: FloodRequest) {
        unimplemented!("FloodRequest")
    }

    pub(crate) fn handle_flood_res(&self, state_guard: &mut StateGuardT, flood: FloodResponse) {
        state_guard.routing_handler.update_graph(flood);
    }
}
