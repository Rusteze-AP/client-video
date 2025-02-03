use wg_internal::{
    controller::DroneEvent,
    network::NodeId,
    packet::{FloodRequest, FloodResponse, NodeType, Packet},
};

use crate::client::{
    utils::sends::{send_packet, send_sc_packet},
    Client,
};

impl Client {
    pub(crate) fn handle_flood_res(&self, flood_res: &FloodResponse) {
        self.state
            .write()
            .routing_handler
            .update_graph(flood_res.clone());
        for (id, node_type) in &flood_res.path_trace {
            if *node_type == NodeType::Server && !self.state.read().servers_id.contains(id) {
                self.state.write().servers_id.push(*id);
                self.state.read().logger.log_info(&format!(
                    "[{}, {}] added server id: {}",
                    file!(),
                    line!(),
                    id
                ));
            }
        }
    }

    fn build_flood_response(flood_req: &FloodRequest) -> (NodeId, Packet) {
        let mut packet = flood_req.generate_response(1); // Note: returns with hop_index = 0;
        let dest = packet.routing_header.current_hop();
        packet.routing_header.increase_hop_index();

        if dest.is_none() {
            return (0, packet);
        }

        (dest.unwrap(), packet)
    }

    fn send_flood_response(&self, dest: NodeId, packet: &Packet) -> Result<(), String> {
        let state = &self.state;

        // Get sender
        let sender = if let Some(s) = state.read().senders.get(&dest) {
            s.clone()
        } else {
            return Err(format!("sender {dest} not found"));
        };

        if let Err(err) = send_packet(state, &sender, packet) {
            state.read().logger.log_warn(&format!(
                "[{}, {}] failed to forward packet to [DRONE-{}] | err: {}",
                file!(),
                line!(),
                packet.routing_header.current_hop().unwrap(),
                err
            ));

            // Send to SC
            send_sc_packet(state, &DroneEvent::ControllerShortcut(packet.clone()))?;

            state.read().logger.log_debug(&format!(
                "[{}, {}], successfully sent flood response through SC. Packet: {}",
                file!(),
                line!(),
                packet
            ));
        }
        Ok(())
    }

    pub(crate) fn handle_flood_req(&self, message: &FloodRequest) {
        let (dest, packet) = Self::build_flood_response(message);
        let res = self.send_flood_response(dest, &packet);

        if let Err(err) = res {
            self.state.read().logger.log_error(&format!(
                "[{}, {}] failed to send flood response, err: {}",
                file!(),
                line!(),
                err
            ));
        }
    }
}
