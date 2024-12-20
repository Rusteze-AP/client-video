use crossbeam::channel::{Receiver, Sender, TryRecvError};
use packet_forge::{PacketForge, TextMessage};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use wg_internal::controller::{DroneCommand, DroneEvent};
use wg_internal::network::NodeId;
use wg_internal::packet::Packet;

use rocket::{self, Build, Ignite, Rocket, State};

#[derive(Debug, Clone)]
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

    /// Runs the client's message processing in a separate thread
    pub fn start_message_processing(mut self, running: Arc<AtomicBool>) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            while running.load(Ordering::SeqCst) {
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
                        break;
                    }
                }

                // Message sending logic
                let text_msg =
                    TextMessage::new("c".repeat(128), String::from("20"), String::from("30"));
                if let Ok(packets) = self.packet_forge.disassemble(&text_msg, vec![20, 1, 30]) {
                    for packet in packets {
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
        })
    }

    /// Configures the Rocket web server
    pub fn configure(client: Client) -> Rocket<Build> {
        rocket::build()
            .manage(client) // Manage the client state
            .mount("/", routes![index, client_info])
    }

    /// Launches the Rocket web server and starts message processing
    pub async fn run(self, running: Arc<AtomicBool>) -> Result<Rocket<Ignite>, rocket::Error> {
        // Start message processing in a separate thread
        let _processing_handle = self.clone().start_message_processing(running);

        // Launch Rocket server
        Self::configure(self).launch().await
    }
}

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

/// Client info endpoint
#[get("/client-info")]
fn client_info(client: &State<Client>) -> String {
    format!("Client ID: {}", client.get_id())
}
