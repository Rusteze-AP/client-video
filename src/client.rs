mod logger_settings;
mod message_handlers;
mod routes;
mod routes_handlers;
mod utils;
mod video_chunker;

use bytes::Bytes;
use crossbeam::channel::{Receiver, Sender};
use logger::{LogLevel, Logger};
use packet_forge::{ClientT, ClientType, PacketForge, SessionIdT};
use parking_lot::RwLock;
use rocket::fs::{relative, FileServer};
use rocket::{Build, Config, Rocket};
use routes::{client_events, request_video, request_video_list, video_stream};
use routing_handler::RoutingHandler;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, LazyLock};
use surrealdb::engine::local::{Db, RocksDb};
use surrealdb::Surreal;
use tokio::sync::broadcast;
use wg_internal::controller::{DroneCommand, DroneEvent};
use wg_internal::network::NodeId;
use wg_internal::packet::{Fragment, Packet};

use crate::utils::{copy_directory, pupulate_db};

type StateT<'a> = Arc<RwLock<ClientState>>;

const BASE_DB_PATH: &str = "db/client_video";
const POPULATE_DB: bool = false;

static RT: LazyLock<tokio::runtime::Runtime> =
    LazyLock::new(|| tokio::runtime::Runtime::new().unwrap());

impl ClientT for Client {
    fn new(
        id: NodeId,
        command_send: Sender<DroneEvent>,
        command_recv: Receiver<DroneCommand>,
        receiver: Receiver<Packet>,
        senders: HashMap<NodeId, Sender<Packet>>,
    ) -> Self {
        RT.block_on(async { Self::new(id, command_send, command_recv, receiver, senders).await })
    }

    fn run(self: Box<Self>, init_client_path: &str) {
        RT.block_on(async { self.run_internal(init_client_path, POPULATE_DB).await });
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn get_id(&self) -> NodeId {
        self.get_id()
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
    terminated: bool,
    routing_handler: RoutingHandler, // Topology graph
    packets_history: HashMap<(u64, SessionIdT), Packet>, // (fragment_index, session_id) -> Packet
    logger: Logger,
    flood_id: u64,
    client_type: ClientType,
}

#[derive(Clone)]
pub struct Client {
    state: Arc<RwLock<ClientState>>,
    db: Arc<Surreal<Db>>,
    video_sender: Arc<RwLock<Option<broadcast::Sender<Bytes>>>>,
}

impl Client {
    #[must_use]
    /// Create a new client
    /// # Panics
    /// This function might panic if the `Surreal` instance fails to initialize
    async fn new(
        id: NodeId,
        command_send: Sender<DroneEvent>,
        command_recv: Receiver<DroneCommand>,
        receiver: Receiver<Packet>,
        senders: HashMap<NodeId, Sender<Packet>>,
    ) -> Self {
        let client_dir = format!("{BASE_DB_PATH}/client_{id}");

        // Initialize directory db
        let db = Surreal::new::<RocksDb>(client_dir).await.unwrap();
        db.use_ns("client_video")
            .use_db(format!("client_{id}"))
            .await
            .unwrap();

        let state = ClientState {
            id,
            controller_send: command_send,
            controller_recv: command_recv,
            packet_recv: receiver,
            senders,
            packet_forge: PacketForge::new(),
            packets_map: HashMap::new(),
            terminated: false,
            routing_handler: RoutingHandler::new(),
            packets_history: HashMap::new(),
            logger: Logger::new(LogLevel::All as u8, false, format!("client-video-{id}")),
            flood_id: 0,
            client_type: ClientType::Video,
        };

        Client {
            state: Arc::new(RwLock::new(state)),
            db: Arc::new(db),
            video_sender: Arc::new(RwLock::new(None)),
        }
    }
    /// Get the ID of the client
    /// # Errors
    /// May create deadlock if the `RwLock` is poisoned
    #[must_use]
    pub fn get_id(&self) -> NodeId {
        self.state.read().id
    }

    async fn init_db(&self, init_client_path: &str, populate_db: bool) {
        // Copy db to client directory
        let client_dir = format!("{BASE_DB_PATH}/client_{}", self.state.read().id);
        let init_db_path = format!("{init_client_path}/db");

        match copy_directory(Path::new(&init_db_path), Path::new(&client_dir)) {
            Ok(()) => self.state.read().logger.log_info("Database copied"),
            Err(e) => self
                .state
                .read()
                .logger
                .log_error(&format!("Failed to copy database from {init_db_path}: {e}")),
        }

        // Initialize db with some data
        if populate_db {
            pupulate_db(&self.db.clone(), init_client_path)
                .await
                .unwrap();

            self.state.read().logger.log_info("Database populated");
        }
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
                routes![
                    client_events,
                    video_stream,
                    request_video,
                    request_video_list
                ],
            )
            .mount("/", FileServer::from(relative!("static")))
    }

    /// Launch the Rocket app
    /// This function will block the current thread until the Rocket app is shut down
    /// # Errors
    /// If the Rocket app fails to launch
    async fn run_internal(self, init_client_path: &str, populate_db: bool) {
        self.init_db(init_client_path, populate_db).await;

        let processing_handle = self.clone().start_message_processing();
        let state = self.state.clone();

        // Launch rocket in a separate task
        let rocket = Self::configure(self).launch();

        // Monitor termination flag in a separate task
        let termination_handle = tokio::spawn(async move {
            loop {
                if state.read().terminated {
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
