use crossbeam::channel::Sender;
use wg_internal::{network::NodeId, packet::Packet};

use super::StateGuardT;

/// Send a `Packet` to a client and update the history
pub fn send_packet(
    state_guard: &mut StateGuardT,
    sender: &Sender<Packet>,
    packet: Packet,
    client_id: NodeId,
) {
    if let Err(e) = sender.send(packet.clone()) {
        eprintln!(
            "Client {}, sending packet {} failed, error: {}",
            client_id, packet.session_id, e
        );
        return;
    }

    // Update history
    state_guard
        .packets_history
        .insert((packet.get_fragment_index(), packet.session_id), packet);
}
