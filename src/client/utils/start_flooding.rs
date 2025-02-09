use wg_internal::{
    controller::DroneEvent,
    network::SourceRoutingHeader,
    packet::{FloodRequest, NodeType, Packet},
};

use crate::client::StateT;

use super::sends::{send_packet, send_sc_packet};

/// Returns the next `flood_id` and increments the current one
fn get_flood_id(state: &StateT) -> u64 {
    state.write().flood_id += 1;
    state.read().flood_id
}

pub(crate) fn init_flood_request(state: &StateT) {
    state.read().logger.log_info(&format!(
        "[{}, {}] starting flood request",
        file!(),
        line!()
    ));

    // Get flood rquest data
    let flood_id = get_flood_id(state);
    let id = state.read().id;
    let senders = state.read().senders.clone();
    let session_id = state.write().packet_forge.get_session_id();

    // Create flood request
    let flood_req = FloodRequest {
        flood_id,
        initiator_id: id,
        path_trace: vec![(id, NodeType::Client)],
    };

    // Prepare the packet and send to all senders
    for (target_id, sender) in &senders {
        let packet = Packet::new_flood_request(
            SourceRoutingHeader::new(vec![], 0),
            session_id,
            flood_req.clone(),
        );

        // Send to node
        if let Err(err) = send_packet(state, sender, &packet) {
            state.read().logger.log_error(&format!(
                "[{}, {}] sending flood_req to [DRONE-{}] | err: {}",
                file!(),
                line!(),
                target_id,
                err
            ));
        }

        // Send to SC
        if let Err(err) = send_sc_packet(state, &DroneEvent::PacketSent(packet)) {
            state.read().logger.log_error(&format!(
                "[{}, {}] failed to send flood_req to SC | err: {}",
                file!(),
                line!(),
                err
            ));
        }
    }
}
