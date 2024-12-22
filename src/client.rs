use crossbeam::channel::{select_biased, Receiver, Sender};
use packet_forge::PacketForge;
use rocket::{self, Build, Ignite, Rocket, State};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;
use wg_internal::controller::{DroneCommand, DroneEvent};
use wg_internal::network::NodeId;
use wg_internal::packet::Packet;

#[derive(Debug)]
pub struct ClientState {
    id: NodeId,
    controller_send: Sender<DroneEvent>,
    controller_recv: Receiver<DroneCommand>,
    packet_recv: Receiver<Packet>,
    senders: HashMap<NodeId, Sender<Packet>>,
    packet_forge: PacketForge,
    terminated: bool,
}

#[derive(Debug, Clone)]
pub struct Client {
    state: Arc<RwLock<ClientState>>,
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
        let state = ClientState {
            id,
            controller_send: command_send,
            controller_recv: command_recv,
            packet_recv: receiver,
            senders,
            packet_forge: PacketForge::new(),
            terminated: false,
        };

        Client {
            state: Arc::new(RwLock::new(state)),
        }
    }

    fn command_dispatcher(&self, command: &DroneCommand) {
        let mut state = self.state.write().unwrap();

        match command {
            DroneCommand::Crash => {
                state.terminated = true;
            }
            DroneCommand::SetPacketDropRate(_) => {
                eprintln!(
                    "Client {}, error: received a SetPacketDropRate command",
                    state.id
                );
            }
            _ => {
                eprintln!(
                    "Client {}, error: received an unknown command: {:?}",
                    state.id, command
                );
            }
        }
    }

    #[must_use]
    pub fn start_message_processing(self) -> thread::JoinHandle<()> {
        let state = self.state.clone();

        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_secs(1));

                // Get mutable access to state
                let mut state_guard = state.write().unwrap();

                if state_guard.terminated {
                    break;
                }

                select_biased! {
                    recv(state_guard.controller_recv) -> command => {
                        if let Ok(command) = command {
                            println!("Client {} received a command: {:?}", state_guard.id, command);
                            self.command_dispatcher(&command);
                        } else {
                            println!("Client {}, SC disconnected", state_guard.id);
                            break;
                        }
                    }
                    recv(state_guard.packet_recv) -> msg => {
                        if let Ok(msg) = msg {
                            println!("Client {} received a message: {:?}", state_guard.id, msg);
                            if state_guard.id == 20 {
                                state_guard.id = 69;
                            } else {
                                state_guard.id = 20;
                            }
                        } else {
                            eprintln!(
                                "Error receiving message for client {}", state_guard.id);
                            break;
                        }
                    }
                }

                // RwLock is automatically released here when state_guard goes out of scope
            }
        })
    }

    #[must_use]
    pub fn configure(client: Client) -> Rocket<Build> {
        rocket::build()
            .manage(client)
            .mount("/", routes![client_info])
    }

    /// Launch the Rocket app
    /// This function will block the current thread until the Rocket app is shut down
    /// # Errors
    /// If the Rocket app fails to launch
    pub async fn run(self) -> Result<Rocket<Ignite>, rocket::Error> {
        let _processing_handle = self.clone().start_message_processing();
        Self::configure(self).launch().await
    }
}

#[get("/client-info")]
fn client_info(client: &State<Client>) -> String {
    let state = client.state.read().unwrap();
    format!("Client ID: {}", state.id)
}
