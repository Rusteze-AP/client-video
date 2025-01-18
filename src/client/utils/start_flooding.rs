use wg_internal::{
    network::SourceRoutingHeader,
    packet::{FloodRequest, NodeType, Packet},
};

use crate::client::{StateGuardWriteT, StateT};

use super::send_packet::{send_packet, send_sc_packet};

/// Returns the `PacketType` formatted as a `String`
// fn get_packet_type(pt: &PacketType) -> String {
//     match pt {
//         PacketType::Ack(_) => "Ack".to_string(),
//         PacketType::Nack(_) => "Nack".to_string(),
//         PacketType::FloodRequest(_) => "Flood request".to_string(),
//         PacketType::FloodResponse(_) => "Flood response".to_string(),
//         PacketType::MsgFragment(_) => "Fragment".to_string(),
//     }
// }

/// Returns the next `flood_id` and increments the current one
fn get_flood_id(state_guard: &mut StateGuardWriteT) -> u64 {
    state_guard.flood_id += 1;
    state_guard.flood_id
}

pub(crate) fn init_flood_request(state: &StateT) {
    // Get flood rquest data
    let (id, flood_id, senders, session_id) = {
        let mut state_guard = state.write().unwrap();
        (
            state_guard.id,
            get_flood_id(&mut state_guard),
            state_guard.senders.clone(),
            state_guard.packet_forge.get_session_id(),
        )
    };

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
        {
            let mut state_guard = state.write().unwrap();
            if let Err(err) = send_packet(&mut state_guard, sender, &packet) {
                state_guard.logger.log_error(&format!(
                    "[CLIENT-{}][FLOODING] Sending to [DRONE-{}]: {}",
                    state_guard.id, target_id, err
                ));
            }
        }

        // Send to SC
        {
            let state_guard = state.read().unwrap();
            if let Err(err) = send_sc_packet(&state_guard, &packet) {
                state_guard.logger.log_error(err.as_str());
            }
        }
    }
}
