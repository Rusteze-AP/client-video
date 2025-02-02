use crossbeam::channel::Sender;
use wg_internal::{controller::DroneEvent, network::NodeId, packet::Packet};

use crate::client::StateT;

/// Send a `Packet` to a client and update the history
pub fn send_packet(state: &StateT, sender: &Sender<Packet>, packet: &Packet) -> Result<(), String> {
    if let Err(e) = sender.send(packet.clone()) {
        return Err(format!(
            "Client {}, sending packet {} failed, error: {}",
            state.read().id,
            packet.session_id,
            e
        ));
    }

    // Update history
    state.write().packets_history.insert(
        (packet.get_fragment_index(), packet.session_id),
        packet.clone(),
    );

    Ok(())
}

/// Send a `Packet` to the SC
pub fn send_sc_packet(state: &StateT, drone_event: &DroneEvent) -> Result<(), String> {
    if let Err(e) = state.read().controller_send.send(drone_event.clone()) {
        return Err(format!(
            "failed sending to SC, drone_event: {drone_event:?}, error: {e}"
        ));
    }

    state
        .read()
        .logger
        .log_info(&format!("[{}, {}] sent packet to SC", file!(), line!()));

    Ok(())
}

/// Send an `Ack` to `sender_id`
pub fn send_ack(state: &StateT, sender_id: NodeId, packet: &Packet) -> Result<(), String> {
    let sender = if let Some(s) = state.read().senders.get(&sender_id) {
        s.clone()
    } else {
        return Err(format!("sender {sender_id} not found"));
    };

    let mut srh = packet.routing_header.get_reversed();
    srh.increase_hop_index();
    let packet = Packet::new_ack(srh, packet.session_id, packet.get_fragment_index());

    if let Err(e) = sender.send(packet.clone()) {
        return Err(format!(
            "Client {}, sending packet {} failed, error: {}",
            state.read().id,
            packet.session_id,
            e
        ));
    }

    Ok(())
}
