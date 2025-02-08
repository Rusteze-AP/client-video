mod logger_settings;
mod message_handlers;
mod routes;
mod routes_handlers;
mod utils;
mod video_chunker;

use bytes::Bytes;
use crossbeam::channel::{Receiver, Sender};
use logger::{LogLevel, Logger};
use packet_forge::{ClientT, ClientType, PacketForge, SessionIdT, VideoMetaData};
use parking_lot::RwLock;
use rocket::fs::{relative, FileServer};
use rocket::{Build, Config, Rocket};
use routes::{
    flood_req, fsm_status, get_id, req_video_list_from_server, request_video,
    request_video_list_from_db, video_list_from_server, video_stream,
};
use routing_handler::RoutingHandler;
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::{Arc, LazyLock};
use tokio::sync::broadcast;
use wg_internal::controller::{DroneCommand, DroneEvent};
use wg_internal::network::NodeId;
use wg_internal::packet::{Fragment, Packet};

use crate::db::structures::VideoDb;

type StateT<'a> = Arc<RwLock<ClientState>>;

const BASE_DB_PATH: &str = "db/client_video";
const FLOODING_TIMER: u64 = 180; // Timer in seconds for sending flood_req

static RT: LazyLock<tokio::runtime::Runtime> =
    LazyLock::new(|| tokio::runtime::Runtime::new().unwrap());

#[derive(Debug, PartialEq, Clone)]
enum FsmStatus {
    ServerNotFound,        // Server not found
    NotSubscribedToServer, // Server found but not connected
    SubscribedToServer,    // Connected to server
    Terminated,            // Client terminated
}

impl Display for FsmStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl ClientT for ClientVideo {
    fn new(
        id: NodeId,
        command_send: Sender<DroneEvent>,
        command_recv: Receiver<DroneCommand>,
        receiver: Receiver<Packet>,
        senders: HashMap<NodeId, Sender<Packet>>,
    ) -> Self {
        Self::new(id, command_send, command_recv, receiver, senders)
    }

    fn run(self: Box<Self>, init_client_path: &str) {
        println!("*********init_client_path: {}", init_client_path);
        RT.block_on(async { self.run_internal(init_client_path).await });
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn get_id(&self) -> NodeId {
        self.get_id()
    }

    fn with_info(&self) {
        self.with_info();
    }
    fn with_debug(&self) {
        self.with_debug();
    }
    fn with_error(&self) {
        self.with_error();
    }
    fn with_warning(&self) {
        self.with_warning();
    }
    fn with_all(&self) {
        self.with_all();
    }
    fn with_web_socket(&self) {
        self.with_web_socket();
    }
}

pub(crate) struct ClientState {
    id: NodeId,
    controller_send: Sender<DroneEvent>,
    controller_recv: Receiver<DroneCommand>,
    packet_recv: Receiver<Packet>,
    senders: HashMap<NodeId, Sender<Packet>>,
    packet_forge: PacketForge,
    packets_map: HashMap<u64, Vec<Fragment>>,
    fsm: FsmStatus,
    routing_handler: RoutingHandler, // Topology graph
    packets_history: HashMap<(u64, SessionIdT), Packet>, // (fragment_index, session_id) -> Packet
    logger: Logger,
    flood_id: u64,
    client_type: ClientType,
    servers_id: Vec<NodeId>,
}

#[derive(Clone)]
pub struct ClientVideo {
    state: Arc<RwLock<ClientState>>,
    video_sender: Arc<RwLock<Option<broadcast::Sender<Bytes>>>>,
    file_list_sender: Arc<RwLock<Option<broadcast::Sender<Vec<VideoMetaData>>>>>,
    db: Arc<VideoDb>,
}

impl ClientVideo {
    #[must_use]
    /// Create a new client
    /// # Panics
    /// This function might panic if the `Surreal` instance fails to initialize
    fn new(
        id: NodeId,
        command_send: Sender<DroneEvent>,
        command_recv: Receiver<DroneCommand>,
        receiver: Receiver<Packet>,
        senders: HashMap<NodeId, Sender<Packet>>,
    ) -> Self {
        let client_dir = format!("{BASE_DB_PATH}/client_{id}");

        let state = ClientState {
            id,
            controller_send: command_send,
            controller_recv: command_recv,
            packet_recv: receiver,
            senders,
            packet_forge: PacketForge::new(),
            packets_map: HashMap::new(),
            fsm: FsmStatus::ServerNotFound,
            routing_handler: RoutingHandler::new(),
            packets_history: HashMap::new(),
            logger: Logger::new(LogLevel::None as u8, false, format!("client-video-{id}")),
            flood_id: 0,
            client_type: ClientType::Video,
            servers_id: Vec::new(),
        };

        ClientVideo {
            state: Arc::new(RwLock::new(state)),
            video_sender: Arc::new(RwLock::new(None)),
            file_list_sender: Arc::new(RwLock::new(None)),
            db: Arc::new(VideoDb::new(&client_dir)),
        }
    }
    /// Get the ID of the client
    /// # Errors
    /// May create deadlock if the `RwLock` is poisoned
    #[must_use]
    pub fn get_id(&self) -> NodeId {
        self.state.read().id
    }

    #[must_use]
    fn configure(client: ClientVideo) -> Rocket<Build> {
        // Config rocket to use a different port for each client
        let config = Config {
            port: 8000 + u16::from(client.get_id()),
            ..Config::default()
        };

        rocket::custom(&config)
            .manage(client)
            .mount(
                "/",
                routes![
                    get_id,
                    fsm_status,
                    video_stream,
                    request_video,
                    request_video_list_from_db,
                    video_list_from_server,
                    req_video_list_from_server,
                    flood_req
                ],
            )
            .mount("/", FileServer::from(relative!("static")))
    }

    /// Launch the Rocket app
    /// This function will block the current thread until the Rocket app is shut down
    /// # Errors
    /// If the Rocket app fails to launch
    async fn run_internal(self, init_client_path: &str) {
        // Initialize the client db
        let res = self.db.init(init_client_path, Some("video_metadata.json"));
        if let Err(err) = res {
            self.state.read().logger.log_error(&err);
            return;
        }

        let processing_handle = self.clone().start_message_processing();
        let state = self.state.clone();

        // Launch rocket in a separate task
        let rocket = Self::configure(self).launch();

        // Monitor termination flag in a separate task
        let termination_handle = tokio::spawn(async move {
            loop {
                if state.read().fsm == FsmStatus::Terminated {
                    // Wait for processing thread to complete
                    let _ = processing_handle.join();
                    break;
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });

        // Run both tasks concurrently
        tokio::select! {
            _ = rocket => {},
            _ = termination_handle => {},
        }
        println!("[CLIENT] Terminated");
    }
}
