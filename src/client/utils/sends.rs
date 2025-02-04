use crossbeam::channel::Sender;
use packet_forge::MessageType;
use wg_internal::{controller::DroneEvent, network::NodeId, packet::Packet};

use crate::client::StateT;

/// Send a `Packet` to a client and update the history
pub fn send_packet(state: &StateT, sender: &Sender<Packet>, packet: &Packet) -> Result<(), String> {
    if let Err(e) = sender.send(packet.clone()) {
        return Err(format!(
            "[{}, {}] sending packet {} failed, error: {}",
            file!(),
            line!(),
            packet.session_id,
            e
        ));
    }

    // Update history
    state.write().packets_history.insert(
        (packet.get_fragment_index(), packet.session_id),
        packet.clone(),
    );

    send_sc_packet(state, &DroneEvent::PacketSent(packet.clone()))?;

    Ok(())
}

/// Send a `Packet` to the SC
pub fn send_sc_packet(state: &StateT, drone_event: &DroneEvent) -> Result<(), String> {
    if let Err(e) = state.read().controller_send.send(drone_event.clone()) {
        return Err(format!(
            "[{}, {}] failed sending to SC, drone_event: {:?}, error: {}",
            file!(),
            line!(),
            drone_event,
            e
        ));
    }

    state
        .read()
        .logger
        .log_debug(&format!("[{}, {}] sent packet to SC", file!(), line!()));

    Ok(())
}

/// Send a `MessageType` to `dest_id`
pub fn send_msg(state: &StateT, dest_id: NodeId, msg: MessageType) -> Result<(), String> {
    let source_id = state.read().id;
    let srh = state.write().routing_handler.best_path(source_id, dest_id);
    if srh.is_none() {
        return Err(format!("[{}, {}] best_path failed", file!(), line!()));
    }
    let srh = srh.unwrap();

    // Disassemble the message into packets
    let Ok(packets) = state.write().packet_forge.disassemble(msg, &srh) else {
        return Err(format!("[{}, {}] disassemble failed", file!(), line!()));
    };

    // Get sender
    let next_hop = srh.hops[1];
    let sender = if let Some(s) = state.read().senders.get(&next_hop) {
        s.clone()
    } else {
        return Err(format!(
            "[{}, {}] Sender {dest_id} not found",
            file!(),
            line!()
        ));
    };

    for packet in packets {
        send_packet(state, &sender, &packet)?;
    }

    Ok(())
}

/// Send an `Ack` to `sender_id`
pub fn send_ack(state: &StateT, packet: &Packet) -> Result<(), String> {
    let mut srh = packet.routing_header.get_reversed();
    srh.increase_hop_index();
    let sender_id = srh.hops[1];
    let packet = Packet::new_ack(srh, packet.session_id, packet.get_fragment_index());

    let sender = if let Some(s) = state.read().senders.get(&sender_id) {
        s.clone()
    } else {
        return Err(format!("sender {sender_id} not found"));
    };

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
