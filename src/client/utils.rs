use crossbeam::channel::Sender;
use wg_internal::packet::Packet;

use super::StateGuardT;

/// Send a `Packet` to a client and update the history
pub fn send_packet(
    state_guard: &mut StateGuardT,
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
