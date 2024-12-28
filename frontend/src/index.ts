import videojs from "video.js";
import { initializePlayer } from "./video_player.js";

function init() {
    const evtSource = new EventSource("/events");

    evtSource.onmessage = function (event) {
        const clientIdElement = document.getElementById("client-id");
        if (clientIdElement) {
            clientIdElement.textContent = event.data;
        }
    };

    const player = initializePlayer("my-video");
}

init();
