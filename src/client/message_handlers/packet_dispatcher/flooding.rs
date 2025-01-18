use wg_internal::{
    controller::DroneEvent,
    network::NodeId,
    packet::{FloodRequest, FloodResponse, Packet},
};

use crate::client::utils::send_packet;

use super::{Client, StateGuardT};

impl Client {
    pub(crate) fn handle_flood_res(state_guard: &mut StateGuardT, flood: FloodResponse) {
        state_guard.routing_handler.update_graph(flood);
    }

    pub(crate) fn get_flood_id(state_guard: &mut StateGuardT) -> u64 {
        state_guard.flood_id += 1;
        state_guard.flood_id
    }

    // pub(crate) fn init_flood_request(&mut self) {
    //     let flood_req = FloodRequest {
    //         flood_id: self.get_flood_id(),
    //         initiator_id: self.id,
    //         path_trace: vec![(self.id, NodeType::Server)],
    //     };
    //     for (id, sender) in &self.packet_send {
    //         let packet = Packet::new_flood_request(
    //             SourceRoutingHeader::new(vec![], 0),
    //             self.packet_forge.get_session_id(),
    //             flood_req.clone(),
    //         );
    //         if let Err(err) = send_packet(sender, &packet) {
    //             self.logger.log_error(&format!(
    //                 "[SERVER-{}][FLOODING] Sending to [DRONE-{}]: {}",
    //                 self.id, id, err
    //             ));
    //         }
    //         let packet_str = get_packet_type(&packet.pack_type);
    //         self.event_dispatcher(&packet, &packet_str);
    //     }
    // }

    fn build_flood_response(flood_req: &FloodRequest) -> (NodeId, Packet) {
        let mut packet = flood_req.generate_response(1); // Note: returns with hop_index = 0;
        packet.routing_header.increase_hop_index();
        let dest = packet.routing_header.current_hop();

        if dest.is_none() {
            return (0, packet);
        }

        (dest.unwrap(), packet)
    }

    fn send_flood_response(
        state_guard: &mut StateGuardT,
        dest: NodeId,
        packet: &Packet,
    ) -> Result<(), String> {
        // Get sender
        let sender = if let Some(s) = state_guard.senders.get(&dest) {
            s.clone()
        } else {
            return Err(format!(
                "Client {}, error: sender {} not found",
                state_guard.id, dest
            ));
        };

        if let Err(err) = send_packet(state_guard, &sender, packet) {
            state_guard.logger.log_warn(&format!("[SERVER-{}][FLOOD RESPONSE] - Failed to forward packet to [DRONE-{}]. \n Error: {} \n Trying to use SC shortcut...", state_guard.id, packet.routing_header.current_hop().unwrap(), err));
            // Send to SC
            let res = state_guard
                .controller_send
                .send(DroneEvent::ControllerShortcut(packet.clone()));

            if res.is_err() {
                return Err(format!(
                    "[SERVER-{}][FLOOD RESPONSE] - Unable to forward packet to neither next hop nor SC. \n Packet: {}",
                    state_guard.id, packet
                ));
            }

            state_guard.logger.log_debug(&format!("[SERVER-{}][FLOOD RESPONSE] - Successfully sent flood response through SC. Packet: {}", state_guard.id, packet));
        }
        Ok(())
    }

    pub(crate) fn handle_flood_req(state_guard: &mut StateGuardT, message: &FloodRequest) {
        let (dest, packet) = Self::build_flood_response(message);
        let res = Self::send_flood_response(state_guard, dest, &packet);

        if let Err(msg) = res {
            state_guard.logger.log_error(msg.as_str());
        }
    }
}
