use std::time::Duration;

use base64::{engine::general_purpose, Engine};
use bytes::Bytes;
use packet_forge::FileHash;
use rocket::{
    response::stream::{Event, EventStream},
    State,
};
use tokio::{sync::broadcast, time::interval};

use crate::client::VideoListSenderT;

use super::{utils::start_flooding::init_flood_request, ClientVideo};

#[get("/get-id")]
pub(crate) fn get_id(client: &State<ClientVideo>) -> String {
    client.get_id().to_string()
}

#[get("/req-video/<video_id>")]
pub(crate) fn request_video(client: &State<ClientVideo>, video_id: FileHash) {
    client.request_video(video_id);
}

#[get("/req-video-list-from-db")]
pub(crate) fn request_video_list_from_db(client: &State<ClientVideo>) -> EventStream![] {
    // let videos_info = get_video_list(&client.db).await.unwrap_or_default();
    let videos_info = client.db.get_video_list();

    EventStream! {
        for video_info in videos_info {
            let json_data = serde_json::to_string(&video_info).unwrap();
            yield Event::data(json_data);
        }
    }
}

#[get("/req-video-list-from-server")]
pub(crate) fn req_video_list_from_server(client: &State<ClientVideo>) {
    client.send_req_file_list();
}

#[get("/fsm-status")]
pub(crate) fn fsm_status(client: &State<ClientVideo>) -> EventStream![] {
    let client_state = client.state.clone();

    EventStream! {
        let mut interval = interval(Duration::from_secs(1));
        loop {
            let fsm_status = client_state.read().fsm.to_string();
            yield Event::data(fsm_status);
            interval.tick().await;
        }
    }
}

#[get("/video-stream")]
pub(crate) fn video_stream(client: &State<ClientVideo>) -> EventStream![] {
    // Create broadcast channel
    let (sender, _) = broadcast::channel::<Bytes>(1024);
    *client.video_sender.write() = Some(sender.clone());
    let mut receiver = sender.subscribe();

    EventStream! {
        while let Ok(chunk) = receiver.recv().await {
            let encoded =  general_purpose::STANDARD.encode(&chunk);
            yield Event::data(encoded);
        }
    }
}

#[get("/video-list-from-server")]
pub(crate) fn video_list_from_server(client: &State<ClientVideo>) -> EventStream![] {
    // Create broadcast channel
    let (sender, _) = broadcast::channel::<VideoListSenderT>(10);
    *client.file_list_sender.write() = Some(sender.clone());
    let mut receiver = sender.subscribe();

    EventStream! {
        while let Ok(video_metadata) = receiver.recv().await {
            let json_metadata =
                serde_json::to_string(&video_metadata).unwrap_or_else(|_| "[]".to_string());
            yield Event::data(json_metadata);
        }
    }
}

#[get("/flood-req")]
pub(crate) fn flood_req(client: &State<ClientVideo>) {
    init_flood_request(&client.state);
}
