import videojs from "video.js";

export function initializePlayer(elementId: string) {
    const videoElement = document.getElementById(elementId);

    if (!videoElement) {
        throw new Error(`No video element found with id ${elementId}`);
    }

    const player = videojs(videoElement, {}, function onPlayerReady() {
        console.log("Player ready");
    });

    return player;
}
