mod message_handlers;
mod routes;
mod routes_handlers;
mod utils;

use bytes::Bytes;
use crossbeam::channel::{Receiver, Sender};
use logger::{LogLevel, Logger};
use packet_forge::{PacketForge, SessionIdT};
use rocket::fs::{relative, FileServer};
use rocket::{Build, Config, Ignite, Rocket};
use routes::{client_events, client_info, request_video, video_stream};
use routing_handler::RoutingHandler;
use std::collections::HashMap;
use std::sync::{Arc, RwLock, RwLockWriteGuard};
use tokio::sync::broadcast;
use wg_internal::controller::{DroneCommand, DroneEvent};
use wg_internal::network::NodeId;
use wg_internal::packet::{Fragment, Packet};

type StateGuardT<'a> = RwLockWriteGuard<'a, ClientState>;

pub(crate) struct ClientState {
    id: NodeId,
    controller_send: Sender<DroneEvent>,
    controller_recv: Receiver<DroneCommand>,
    packet_recv: Receiver<Packet>,
    senders: HashMap<NodeId, Sender<Packet>>,
    packet_forge: PacketForge,
    packets_map: HashMap<u64, Vec<Fragment>>,
    terminated: bool,
    video_sender: Option<broadcast::Sender<Bytes>>,
    routing_handler: RoutingHandler, // Topology graph
    packets_history: HashMap<(u64, SessionIdT), Packet>, // (fragment_index, session_id) -> Packet
    logger: Logger,
    flood_id: u64,
}

#[derive(Clone)]
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
            routing_handler: RoutingHandler::new(),
            packets_history: HashMap::new(),
            logger: Logger::new(LogLevel::None as u8, false, "RustezeDrone".to_string()),
            flood_id: 0,
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

    #[must_use]
    fn configure(client: Client) -> Rocket<Build> {
        // Config rocket to use a different port for each client
        let config = Config {
            port: 8000 + u16::from(client.get_id()),
            ..Config::default()
        };

        rocket::custom(&config)
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
