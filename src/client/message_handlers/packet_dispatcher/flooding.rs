use wg_internal::{
    controller::DroneEvent,
    network::NodeId,
    packet::{FloodRequest, FloodResponse, Packet},
};

use crate::client::utils::send_packet::send_packet;

use super::{Client, StateGuardWriteT};

impl Client {
    pub(crate) fn handle_flood_res(state_guard: &mut StateGuardWriteT, flood: FloodResponse) {
        state_guard.routing_handler.update_graph(flood);
    }

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
        state_guard: &mut StateGuardWriteT,
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
            state_guard.logger.log_warn(&format!(
                "[{}, {}] failed to forward packet to [DRONE-{}] | err: {}",
                file!(),
                line!(),
                packet.routing_header.current_hop().unwrap(),
                err
            ));
            // Send to SC
            let res = state_guard
                .controller_send
                .send(DroneEvent::ControllerShortcut(packet.clone()));

            if res.is_err() {
                return Err(format!(
                    "[{}, {}] Unable to forward packet to neither next hop nor SC. \n Packet: {}",
                    file!(),
                    line!(),
                    packet
                ));
            }

            state_guard.logger.log_debug(&format!(
                "[{}, {}], successfully sent flood response through SC. Packet: {}",
                file!(),
                line!(),
                packet
            ));
        }
        Ok(())
    }

    pub(crate) fn handle_flood_req(state_guard: &mut StateGuardWriteT, message: &FloodRequest) {
        let (dest, packet) = Self::build_flood_response(message);
        let res = Self::send_flood_response(state_guard, dest, &packet);

        if let Err(err) = res {
            state_guard.logger.log_error(&format!(
                "[{}, {}] failed to send flood response, err: {}",
                file!(),
                line!(),
                err
            ));
        }
    }
}
