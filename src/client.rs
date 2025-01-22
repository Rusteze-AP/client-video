mod logger_settings;
mod message_handlers;
mod routes;
mod routes_handlers;
mod utils;
mod video_chunker;

use bytes::Bytes;
use crossbeam::channel::{Receiver, Sender};
use logger::{LogLevel, Logger};
use packet_forge::{PacketForge, SessionIdT};
use rocket::fs::{relative, FileServer};
use rocket::{Build, Config, Rocket};
use routes::{client_events, request_video, request_video_list, video_stream};
use routing_handler::RoutingHandler;
use std::collections::HashMap;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use surrealdb::engine::local::{Db, Mem};
use surrealdb::Surreal;
use tokio::sync::broadcast;
use wg_internal::controller::{DroneCommand, DroneEvent};
use wg_internal::network::NodeId;
use wg_internal::packet::{Fragment, Packet};

use crate::utils::pupulate_db;

type StateT<'a> = Arc<RwLock<ClientState>>;
type StateGuardWriteT<'a> = RwLockWriteGuard<'a, ClientState>;
type StateGuardReadT<'a> = RwLockReadGuard<'a, ClientState>;

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
    db: Arc<Surreal<Db>>,
}

impl Client {
    #[must_use]
    /// Create a new client
    /// # Panics
    /// This function might panic if the `Surreal` instance fails to initialize
    pub async fn new(
        id: NodeId,
        command_send: Sender<DroneEvent>,
        command_recv: Receiver<DroneCommand>,
        receiver: Receiver<Packet>,
        senders: HashMap<NodeId, Sender<Packet>>,
    ) -> Self {
        let db = Surreal::new::<Mem>(()).await.unwrap();
        db.use_ns("video_client")
            .use_db(format!("client_{id}"))
            .await
            .unwrap();
        let db = Arc::new(db);
        // Initialize the database with some data
        pupulate_db(&db.clone()).await.unwrap();

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
            logger: Logger::new(
                LogLevel::None as u8,
                false,
                "video-streamer-client".to_string(),
            ),
            flood_id: 0,
        };

        Client {
            state: Arc::new(RwLock::new(state)),
            db,
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
    /// # Panics
    /// This function might panic when called if the lock is already held by the current thread.
    pub async fn run(self) {
        let processing_handle = self.clone().start_message_processing();
        let state = self.state.clone();

        // Launch rocket in a separate task
        let rocket = Self::configure(self).launch();

        // Monitor termination flag in a separate task
        let termination_handle = tokio::spawn(async move {
            loop {
                if state.read().unwrap().terminated {
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
