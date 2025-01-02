import videojs from "video.js";

function req_video() {
    document.getElementById("req-video")?.addEventListener("click", async () => {
        try {
            const response = await fetch("/req-video/dancing_pirate.mp4", {
                method: "GET",
            });

            if (response.ok) {
                console.log("Message sent successfully:", await response.text());
            } else {
                console.error("Failed to send message:", response.status);
            }
        } catch (error) {
            console.error("Error sending message:", error);
        }
    });
}

function init() {
    // Handle client ID updates
    const clientEvtSource = new EventSource("/events");
    clientEvtSource.onmessage = function (event) {
        const clientIdElement = document.getElementById("client-id");
        if (clientIdElement) {
            clientIdElement.textContent = event.data;
        }
    };

    req_video();

    const elementId = "my-video";
    const videoElement = document.getElementById(elementId) as HTMLVideoElement | null;

    if (!videoElement) {
        throw new Error(`No video element found with id ${elementId}`);
    }

    // Create MediaSource instance
    const mediaSource = new MediaSource();
    const videoURL = URL.createObjectURL(mediaSource);

    // Initialize video.js with the MediaSource URL
    const player = videojs(videoElement, {}, function onPlayerReady() {
        player.src({
            src: videoURL,
            type: "video/mp4",
        });
    });

    let sourceBuffer: SourceBuffer | null = null;

    // Helper function to decode chunks
    async function decodeChunk(data: string): Promise<Uint8Array> {
        // If data is base64 encoded
        if (typeof data === "string") {
            const binaryString = atob(data);
            const bytes = new Uint8Array(binaryString.length);
            for (let i = 0; i < binaryString.length; i++) {
                bytes[i] = binaryString.charCodeAt(i);
            }
            return bytes;
        }
        // If data is already a blob
        // if (data instanceof Blob) {
        //     const arrayBuffer = await data.arrayBuffer();
        //     return new Uint8Array(arrayBuffer);
        // }
        throw new Error("Unsupported data format");
    }

    // Handle buffer updates
    mediaSource.addEventListener("sourceopen", () => {
        sourceBuffer = mediaSource.addSourceBuffer('video/mp4; codecs="avc1.42E01E,mp4a.40.2"');

        // Set up EventSource for receiving video chunks
        const evtSource = new EventSource("/video-stream");

        evtSource.onmessage = async (event) => {
            try {
                // Decode base64 data if your server sends it encoded
                const videoChunk = await decodeChunk(event.data);

                // Wait if the buffer is updating
                if (sourceBuffer!.updating) {
                    await new Promise((resolve) => {
                        sourceBuffer!.addEventListener("updateend", resolve, { once: true });
                    });
                }

                // Append the chunk to the source buffer
                sourceBuffer!.appendBuffer(videoChunk);
            } catch (error) {
                console.error("Error processing video chunk:", error);
            }
        };

        evtSource.onerror = (error) => {
            console.error("EventSource error:", error);
            evtSource.close();
        };

        // Handle buffer updates
        sourceBuffer.addEventListener("updateend", () => {
            // Check if we need to remove old data to prevent memory issues
            if (sourceBuffer!.buffered.length > 0) {
                const bufferEnd = sourceBuffer!.buffered.end(0);
                const currentTime = player.currentTime();

                // Remove data more than 30 seconds behind current playback
                if (bufferEnd - currentTime > 30) {
                    sourceBuffer!.remove(0, currentTime - 10);
                }
            }
        });
    });

    // Cleanup function
    function cleanup() {
        if (mediaSource.readyState === "open") {
            mediaSource.endOfStream();
        }
        URL.revokeObjectURL(videoURL);
    }

    // Handle player disposal
    player.on("dispose", cleanup);
}

init();
