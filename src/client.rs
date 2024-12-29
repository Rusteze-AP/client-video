use bytes::{Bytes, BytesMut};
use crossbeam::channel::{select_biased, Receiver, Sender};
use packet_forge::PacketForge;
use rocket::fs::{relative, FileServer};
use rocket::response::stream::{Event, EventStream};
use rocket::{self, Build, Ignite, Rocket, State};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use std::{io, thread};
use tokio::time::interval;
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

    pub fn get_id(&self) -> NodeId {
        self.state.read().unwrap().id
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
                            // println!("Client {} received a message: {:?}", state_guard.id, msg);
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
    fn configure(client: Client) -> Rocket<Build> {
        rocket::build()
            .manage(client)
            .mount("/", routes![client_info, client_events, video_stream])
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

#[get("/client-info")]
fn client_info(client: &State<Client>) -> String {
    let state = client.state.read().unwrap();
    format!("Client ID: {}", state.id)
}

#[get("/events")]
fn client_events(client: &State<Client>) -> EventStream![] {
    println!("Starting event stream");
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
fn video_stream() -> EventStream![] {
    EventStream! {
        let mut video_chunks = get_video_chunks();
        while let Some(chunk) = video_chunks.next() {
            // Encode the chunk as base64 if needed
            let encoded_chunk = base64::encode(&chunk);
            yield Event::data(encoded_chunk);
        }
    }
}

pub struct VideoChunker {
    file: File,
    chunk_size: usize,
    position: u64,
    file_size: u64,
}

impl VideoChunker {
    pub fn new(path: impl AsRef<Path>, chunk_size: usize) -> io::Result<Self> {
        let file = File::open(path)?;
        let file_size = file.metadata()?.len();

        Ok(VideoChunker {
            file,
            chunk_size,
            position: 0,
            file_size,
        })
    }

    pub fn next_chunk(&mut self) -> io::Result<Option<Bytes>> {
        if self.position >= self.file_size {
            return Ok(None);
        }

        let mut buffer = BytesMut::with_capacity(self.chunk_size);
        buffer.resize(self.chunk_size, 0);

        self.file.seek(SeekFrom::Start(self.position))?;
        let bytes_read = self.file.read(&mut buffer)?;

        if bytes_read == 0 {
            return Ok(None);
        }

        buffer.truncate(bytes_read);
        self.position += bytes_read as u64;

        Ok(Some(buffer.freeze()))
    }

    pub fn reset(&mut self) -> io::Result<()> {
        self.position = 0;
        self.file.seek(SeekFrom::Start(0))?;
        Ok(())
    }
}

// Generator function for the EventStream
pub fn get_video_chunks() -> impl Iterator<Item = Bytes> {
    struct ChunkIterator {
        chunker: VideoChunker,
    }

    impl Iterator for ChunkIterator {
        type Item = Bytes;

        fn next(&mut self) -> Option<Self::Item> {
            match self.chunker.next_chunk() {
                Ok(Some(chunk)) => Some(chunk),
                _ => {
                    // Reset the chunker and return None to end the stream
                    let _ = self.chunker.reset();
                    None
                }
            }
        }
    }

    // Create the chunker with a 1MB chunk size
    let chunker = VideoChunker::new("../client/static/videos/dancing_pirate.mp4", 1024 * 1024)
        .expect("Failed to create video chunker");

    ChunkIterator { chunker }
}
