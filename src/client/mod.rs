mod message_handler;
mod routes;
mod routes_handlers;

use bytes::Bytes;
use crossbeam::channel::{Receiver, Sender};
use packet_forge::PacketForge;
use rocket::fs::{relative, FileServer};
use rocket::{Build, Ignite, Rocket};
use routes::{client_events, client_info, request_video, video_stream};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;
use wg_internal::controller::{DroneCommand, DroneEvent};
use wg_internal::network::NodeId;
use wg_internal::packet::{Fragment, Packet};

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
