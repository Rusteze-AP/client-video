use crate::message::{Message, Serializable};
use crossbeam::channel::{Receiver, Sender, TryRecvError};
use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use wg_internal::controller::{DroneCommand, DroneEvent};
use wg_internal::network::{NodeId, SourceRoutingHeader};
use wg_internal::packet::{Fragment, Packet, PacketType, FRAGMENT_DSIZE};

trait RustezeMessage {
    fn serialize(&self) -> String;
    fn deserialize(serialized: String) -> Self;
    fn disassembly(serialized: String) -> Vec<Fragment>;
    fn assembly(fragments: Vec<Fragment>) -> String;
}

impl<M: Serializable> RustezeMessage for Message<M> {
    /// Takes Message and returns its "content" serialized in a String
    fn serialize(&self) -> String {
        todo!()
    }
    /// Takes a serialized string and returns the "content" of a Message
    fn deserialize(serialized: String) -> Self {
        todo!()
    }
    /// Takes a serialized string and returns a vector of Fragments
    fn disassembly(serialized: String) -> Vec<Fragment> {
        todo!()
    }
    /// Takes a vector of Fragments and returns a serialized string
    fn assembly(fragments: Vec<Fragment>) -> String {
        todo!()
    }
}

#[derive(Debug)]
pub struct Client {
    id: NodeId,
    command_send: Sender<DroneEvent>,
    command_recv: Receiver<DroneCommand>,
    receiver: Receiver<Packet>,
    senders: HashMap<NodeId, Sender<Packet>>,
}

impl Client {
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
        }
    }

    pub fn get_id(&self) -> NodeId {
        self.id
    }

    pub fn run(&self) {
        loop {
            thread::sleep(Duration::from_secs(1));

            // Check if there's a message from the drone
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

            // Create packet
            let frag = Fragment {
                fragment_index: 0,
                total_n_fragments: 1,
                length: 80,
                data: [1; FRAGMENT_DSIZE],
            };
            let source_routing_header = SourceRoutingHeader {
                hop_index: 1,
                hops: vec![20, 1, 30],
            };
            let packet = Packet {
                pack_type: PacketType::MsgFragment(frag),
                routing_header: source_routing_header,
                session_id: 1,
            };

            // Send packet to server
            let id = 1;
            if let Some(sender) = self.senders.get(&id) {
                sender.send(packet).unwrap();
                println!("Client {} sent packet to node {}", self.id, id);
            } else {
                println!("Client {} could not send packet to node {}", self.id, id);
            }
        }
    }
}
