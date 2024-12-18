use crossbeam::channel::{Receiver, Sender, TryRecvError};
use packet_forge::{PacketForge, TextMessage};
use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use wg_internal::controller::{DroneCommand, DroneEvent};
use wg_internal::network::NodeId;
use wg_internal::packet::Packet;

#[derive(Debug)]
pub struct Client {
    id: NodeId,
    command_send: Sender<DroneEvent>,
    command_recv: Receiver<DroneCommand>,
    receiver: Receiver<Packet>,
    senders: HashMap<NodeId, Sender<Packet>>,
    packet_forge: PacketForge,
}

impl Client {
    #[must_use]
    pub fn new(
        id: NodeId,
        command_send: Sender<DroneEvent>,
        command_recv: Receiver<DroneCommand>,
        receiver: Receiver<Packet>,
        senders: HashMap<NodeId, Sender<Packet>>,
    ) -> Self {
        Client {
            id,
            command_send,
            command_recv,
            receiver,
            senders,
            packet_forge: PacketForge::new(),
        }
    }

    pub fn get_id(&self) -> NodeId {
        self.id
    }

    pub fn run(&mut self) {
        loop {
            thread::sleep(Duration::from_secs(1));

            // Check if there's a message
            match self.receiver.try_recv() {
                Ok(packet) => {
                    println!("Client {} received a message: {:?}", self.id, packet);
                }
                Err(TryRecvError::Empty) => {
                    println!("No messages for client {}", self.id);
                }
                Err(err) => {
                    eprintln!("Error receiving message for client {}: {:?}", self.id, err);
                }
            }

            let text_msg =
                TextMessage::new("c".repeat(128), String::from("20"), String::from("30"));
            let packets = self.packet_forge.disassemble(&text_msg, vec![20, 1, 30]);
            if let Ok(packets) = packets {
                for packet in packets {
                    // Send packet to server
                    let id = 1;
                    if let Some(sender) = self.senders.get(&id) {
                        if let Err(err) = sender.send(packet) {
                            eprintln!("Error sending packet to node {id}: {err:?}");
                        } else {
                            println!("Client {} sent packet to node {}", self.id, id);
                        }
                    } else {
                        println!("Client {} could not send packet to node {}", self.id, id);
                    }
                }
            } else {
                eprintln!("Error disassembling message: {text_msg:?}");
            }
        }
    }
}
