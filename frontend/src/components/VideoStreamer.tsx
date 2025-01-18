import React, { useEffect, useRef } from "react";
import videojs from "video.js";
import Player from "video.js/dist/types/player";
import "video.js/dist/video-js.css";

interface VideoJSOptions {
    controls: boolean;
    responsive: boolean;
    fluid: boolean;
    sources: {
        src: string;
        type: string;
    }[];
}

const VideoStreamer: React.FC = () => {
    const videoRef = useRef<HTMLVideoElement | null>(null);
    const playerRef = useRef<Player | null>(null);
    const mediaSourceRef = useRef<MediaSource | null>(null);
    const sourceBufferRef = useRef<SourceBuffer | null>(null);

    const decodeChunk = async (data: string): Promise<Uint8Array> => {
        if (typeof data === "string") {
            const binaryString = atob(data);
            const bytes = new Uint8Array(binaryString.length);
            for (let i = 0; i < binaryString.length; i++) {
                bytes[i] = binaryString.charCodeAt(i);
            }
            return bytes;
        }
        throw new Error("Unsupported data format");
    };

    const requestVideo = async (): Promise<void> => {
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
    };

    useEffect(() => {
        // Initialize video player
        if (!videoRef.current) return;

        mediaSourceRef.current = new MediaSource();
        const videoURL = URL.createObjectURL(mediaSourceRef.current);

        const videoJsOptions: VideoJSOptions = {
            controls: true,
            responsive: true,
            fluid: true,
            sources: [
                {
                    src: videoURL,
                    type: "video/mp4",
                },
            ],
        };

        playerRef.current = videojs(videoRef.current, videoJsOptions, function onPlayerReady(this: Player) {
            this.src({
                src: videoURL,
                type: "video/mp4",
            });
        });

        // Handle MediaSource setup
        mediaSourceRef.current.addEventListener("sourceopen", () => {
            if (!mediaSourceRef.current) return;

            sourceBufferRef.current = mediaSourceRef.current.addSourceBuffer(
                'video/mp4; codecs="avc1.42E01E,mp4a.40.2"'
            );

            const evtSource = new EventSource("/video-stream");

            evtSource.onmessage = async (event: MessageEvent) => {
                try {
                    const videoChunk = await decodeChunk(event.data);

                    if (sourceBufferRef.current?.updating) {
                        await new Promise<void>((resolve) => {
                            sourceBufferRef.current?.addEventListener("updateend", () => resolve(), { once: true });
                        });
                    }

                    sourceBufferRef.current?.appendBuffer(videoChunk);
                } catch (error) {
                    console.error("Error processing video chunk:", error);
                }
            };

            evtSource.onerror = (error: Event) => {
                console.error("EventSource error:", error);
                evtSource.close();
            };

            // Handle buffer updates
            sourceBufferRef.current.addEventListener("updateend", () => {
                if (!sourceBufferRef.current || !playerRef.current) return;

                if (sourceBufferRef.current.buffered.length > 0) {
                    const bufferEnd = sourceBufferRef.current.buffered.end(0);
                    const currentTime = playerRef.current.currentTime();

                    if (currentTime !== undefined && bufferEnd - currentTime > 30) {
                        sourceBufferRef.current.remove(0, currentTime - 10);
                    }
                }
            });
        });

        // Cleanup
        return () => {
            if (playerRef.current) {
                playerRef.current.dispose();
            }
            if (mediaSourceRef.current?.readyState === "open") {
                mediaSourceRef.current.endOfStream();
            }
            URL.revokeObjectURL(videoURL);
        };
    }, []);

    return (
        <div className="w-full max-w-4xl mx-auto p-4">
            <button
                onClick={requestVideo}
                className="mb-4 px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600 transition-colors"
            >
                Request Video
            </button>

            <div>
                <video
                    ref={videoRef}
                    className="video-js vjs-big-play-centered"
                    controls
                    preload="auto"
                    width={640}
                    height={360}
                    data-setup="{}"
                >
                    <p className="vjs-no-js">
                        To view this video, please enable JavaScript and consider upgrading to a web browser that
                        supports HTML5 video.
                    </p>
                </video>
            </div>
        </div>
    );
};

export default VideoStreamer;
