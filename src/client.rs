use base64::{engine::general_purpose, Engine};
use bytes::Bytes;
use crossbeam::channel::{Receiver, Sender, TryRecvError};
use packet_forge::{ChunkRequest, Index, MessageType, PacketForge};
use rocket::fs::{relative, FileServer};
use rocket::response::stream::{Event, EventStream};
use rocket::{self, Build, Ignite, Rocket, State};
use std::collections::HashMap;
use std::sync::{Arc, RwLock, RwLockWriteGuard};
use std::thread;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time::interval;
use wg_internal::controller::{DroneCommand, DroneEvent};
use wg_internal::network::NodeId;
use wg_internal::packet::{Fragment, Packet, PacketType};

#[derive(Debug)]
pub struct ClientState {
    id: NodeId,
    controller_send: Sender<DroneEvent>,
    controller_recv: Receiver<DroneCommand>,
    packet_recv: Receiver<Packet>,
    senders: HashMap<NodeId, Sender<Packet>>,
    packet_forge: PacketForge,
    packets_map: HashMap<u64, Vec<Fragment>>,
    terminated: bool,
    video_sender: Option<broadcast::Sender<Bytes>>,
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
            packets_map: HashMap::new(),
            terminated: false,
            video_sender: None,
        };

        Client {
            state: Arc::new(RwLock::new(state)),
        }
    }

    /// Get the ID of the client
    /// # Errors
    /// May create deadlock if the `RwLock` is poisoned
    /// # Panics
    /// This function might panic when called if the lock is already held by the current thread.
    #[must_use]
    pub fn get_id(&self) -> NodeId {
        self.state.read().unwrap().id
    }

    fn request_video(&self, video_name: &str) {
        let mut state = self.state.write().unwrap();
        let msg = ChunkRequest::new(video_name.to_string(), Index::All);
        let packets = state
            .packet_forge
            .disassemble(msg, vec![20, 1, 30])
            .unwrap();
        let sender = state.senders.get(&1).unwrap();
        for packet in packets {
            sender.send(packet).unwrap();
        }
    }

    fn command_dispatcher(
        &self,
        state: &mut RwLockWriteGuard<'_, ClientState>,
        command: &DroneCommand,
    ) {
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

    fn handle_messages(&self, state: &mut RwLockWriteGuard<'_, ClientState>, message: MessageType) {
        match message {
            MessageType::SubscribeClient(content) => {
                println!(
                    "Client {} received a SubscribeClient message: {:?}",
                    state.id, content
                );
            }
            MessageType::ChunkResponse(content) => {
                // Send data to event stream
                if let Some(sender) = &state.video_sender {
                    let _ = sender.send(content.chunk_data);
                }
            }
            _ => {
                println!("Client {} received an unimplemented message", state.id);
            }
        }
    }

    fn handle_packets(&self, state: &mut RwLockWriteGuard<'_, ClientState>, packet: Packet) {
        let session_id = packet.session_id;
        match packet.pack_type {
            PacketType::MsgFragment(frag) => {
                // Add fragment to packets_map
                state.packets_map.entry(session_id).or_default().push(frag);
                let fragments = state.packets_map.get(&session_id).unwrap();
                let total_fragments = fragments[0].total_n_fragments;

                // If all fragments are received, assemble the message
                if fragments.len() as u64 == total_fragments {
                    let assembled = match state.packet_forge.assemble_dynamic(fragments.clone()) {
                        Ok(message) => message,
                        Err(e) => panic!("Error assembling: {e}"),
                    };
                    state.packets_map.remove(&session_id);
                    self.handle_messages(state, assembled);
                }
            }
            _ => {
                println!(
                    "Client {} received an unimplemented packet: {:?}",
                    state.id, packet
                );
            }
        }
    }

    #[must_use]
    fn start_message_processing(self) -> thread::JoinHandle<()> {
        let state = self.state.clone();

        thread::spawn(move || {
            loop {
                // thread::sleep(Duration::from_secs(1));

                // Get mutable access to state
                let mut state_guard = state.write().unwrap();

                if state_guard.terminated {
                    break;
                }

                match state_guard.controller_recv.try_recv() {
                    Ok(command) => {
                        self.command_dispatcher(&mut state_guard, &command);
                    }
                    Err(TryRecvError::Empty) => {}
                    Err(e) => {
                        eprintln!(
                            "Error receiving command for server {}: {:?}",
                            state_guard.id, e
                        );
                    }
                }

                match state_guard.packet_recv.try_recv() {
                    Ok(packet) => {
                        if state_guard.id == 20 {
                            state_guard.id = 69;
                        } else {
                            state_guard.id = 20;
                        }
                        self.handle_packets(&mut state_guard, packet);
                    }
                    Err(TryRecvError::Empty) => {}
                    Err(e) => {
                        eprintln!(
                            "Error receiving message for server {}: {:?}",
                            state_guard.id, e
                        );
                    }
                }

                // RwLock is automatically released here when state_guard goes out of scope
            }
        })
    }

    #[must_use]
    fn configure(client: Client) -> Rocket<Build> {
        rocket::build()
            .manage(client)
            .mount(
                "/",
                routes![client_info, client_events, video_stream, request_video],
            )
            .mount("/", FileServer::from(relative!("static")))
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

#[get("/req-video/<video_name>")]
fn request_video(client: &State<Client>, video_name: &str) {
    client.request_video(video_name);
}

#[get("/client-info")]
fn client_info(client: &State<Client>) -> String {
    let state = client.state.read().unwrap();
    format!("Client ID: {}", state.id)
}

#[get("/events")]
fn client_events(client: &State<Client>) -> EventStream![] {
    let client_state = client.state.clone();

    EventStream! {
        let mut interval = interval(Duration::from_secs(1));
        loop {
            let id = client_state.read().unwrap().id;
            yield Event::data(id.to_string());
            interval.tick().await;
        }
    }
}

#[get("/video-stream")]
fn video_stream(client: &State<Client>) -> EventStream![] {
    let (sender, _) = broadcast::channel::<Bytes>(1024);
    {
        let mut state = client.state.write().unwrap();
        state.video_sender = Some(sender.clone());
    }

    let mut receiver = sender.subscribe();

    EventStream! {
        while let Ok(chunk) = receiver.recv().await {
            let encoded =  general_purpose::STANDARD.encode(&chunk);
            yield Event::data(encoded);
        }
    }
}
