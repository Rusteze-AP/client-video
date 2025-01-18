use crossbeam::channel::Sender;
use wg_internal::{controller::DroneEvent, packet::Packet};

use crate::client::{StateGuardReadT, StateGuardWriteT};

/// Send a `Packet` to a client and update the history
pub fn send_packet(
    state_guard: &mut StateGuardWriteT,
    sender: &Sender<Packet>,
    packet: &Packet,
) -> Result<(), String> {
    if let Err(e) = sender.send(packet.clone()) {
        return Err(format!(
            "Client {}, sending packet {} failed, error: {}",
            state_guard.id, packet.session_id, e
        ));
    }

    // Update history
    state_guard.packets_history.insert(
        (packet.get_fragment_index(), packet.session_id),
        packet.clone(),
    );

    Ok(())
}

pub fn send_sc_packet(state_guard: &StateGuardReadT, packet: &Packet) -> Result<(), String> {
    if let Err(e) = state_guard
        .controller_send
        .send(DroneEvent::PacketSent(packet.clone()))
    {
        return Err(format!(
            "[CLIENT {}][SC EVENT], sending packet {} failed, error: {}",
            state_guard.id, packet.session_id, e
        ));
    }

    state_guard.logger.log_info("Sent packet to SC");

    Ok(())
}
