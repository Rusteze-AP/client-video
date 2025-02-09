use wg_internal::{
    controller::DroneEvent,
    network::NodeId,
    packet::{FloodRequest, FloodResponse, NodeType, Packet},
};

use crate::client::{
    utils::sends::{send_packet, send_sc_packet},
    ClientVideo, FsmStatus,
};

impl ClientVideo {
    pub(crate) fn handle_flood_res(&self, flood_res: &FloodResponse) {
        self.state
            .write()
            .routing_handler
            .update_graph(flood_res.clone());

        for (id, node_type) in &flood_res.path_trace {
            // If node is Server and not in the list, add it
            if *node_type == NodeType::Server && !self.state.read().servers.contains_key(id) {
                self.state.write().servers.insert(*id, Vec::new());
                self.send_subscribe_client(*id);

                let fsm = self.state.read().fsm.clone();
                if fsm == FsmStatus::ServerNotFound {
                    self.state.write().fsm = FsmStatus::NotSubscribedToServer;
                }

                self.state.read().logger.log_info(&format!(
                    "[{}, {}] added server id: {}",
                    file!(),
                    line!(),
                    id
                ));
            }
        }
    }

    fn build_flood_response(flood_req: &FloodRequest, client_id: NodeId) -> (NodeId, Packet) {
        let mut flood_req = flood_req.clone();
        flood_req.path_trace.push((client_id, NodeType::Client));

        let mut packet = flood_req.generate_response(1); // Note: returns with hop_index = 0;
        let dest = packet.routing_header.next_hop();
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
        let id = self.get_id();
        let (dest, packet) = Self::build_flood_response(message, id);
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
