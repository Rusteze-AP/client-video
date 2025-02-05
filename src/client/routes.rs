use std::time::Duration;

use base64::{engine::general_purpose, Engine};
use bytes::Bytes;
use packet_forge::{FileHash, VideoMetaData};
use rocket::{
    response::stream::{Event, EventStream},
    State,
};
use tokio::{sync::broadcast, time::interval};

use crate::db::queries::get_video_list;

use super::Client;

#[get("/get-id")]
pub(crate) fn get_id(client: &State<Client>) -> String {
    client.get_id().to_string()
}

#[get("/req-video/<video_id>")]
pub(crate) async fn request_video(client: &State<Client>, video_id: FileHash) {
    client.request_video(video_id).await;
}

#[get("/req-video-list-from-db")]
pub(crate) async fn request_video_list_from_db(client: &State<Client>) -> EventStream![] {
    let videos_info = get_video_list(&client.db).await.unwrap_or_default();

    EventStream! {
        for video_info in videos_info {
            let json_data = serde_json::to_string(&video_info).unwrap();
            yield Event::data(json_data);
        }
    }
}

#[get("/req-video-list-from-server")]
pub(crate) fn req_video_list_from_server(client: &State<Client>) {
    client.send_req_file_list();
}

#[get("/fsm-status")]
pub(crate) fn fsm_status(client: &State<Client>) -> EventStream![] {
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
pub(crate) fn video_stream(client: &State<Client>) -> EventStream![] {
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
pub(crate) fn video_list_from_server(client: &State<Client>) -> EventStream![] {
    let (sender, _) = broadcast::channel::<Vec<VideoMetaData>>(10);
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
