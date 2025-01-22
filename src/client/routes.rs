use std::time::Duration;

use base64::{engine::general_purpose, Engine};
use bytes::Bytes;
use packet_forge::FileHash;
use rocket::{
    response::stream::{Event, EventStream},
    State,
};
use tokio::{sync::broadcast, time::interval};

use crate::db::queries::get_video_list;

use super::Client;

#[get("/req-video/<video_id>")]
pub(crate) async fn request_video(client: &State<Client>, video_id: FileHash) {
    client.request_video(video_id).await;
}

#[get("/req-video-list")]
pub(crate) async fn request_video_list(client: &State<Client>) -> EventStream![] {
    let videos_info = get_video_list(client.db.clone()).await.unwrap_or_default();

    EventStream! {
        for video_info in videos_info {
            let json_data = serde_json::to_string(&video_info).unwrap();
            yield Event::data(json_data);
        }
    }
}

#[get("/events")]
pub(crate) fn client_events(client: &State<Client>) -> EventStream![] {
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
pub(crate) fn video_stream(client: &State<Client>) -> EventStream![] {
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
