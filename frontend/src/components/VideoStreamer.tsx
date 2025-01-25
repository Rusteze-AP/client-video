import React, { useEffect, useRef, useState } from "react";
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

type VideoInfo = {
    id: number;
    title: string;
    description: string;
    duration: number;
    mime_type: string;
    created_at: string;
};

const VideoStreamer: React.FC = () => {
    const videoRef = useRef<HTMLVideoElement | null>(null);
    const playerRef = useRef<Player | null>(null);
    const mediaSourceRef = useRef<MediaSource | null>(null);
    const sourceBufferRef = useRef<SourceBuffer | null>(null);

    const [videos, setVideos] = useState<VideoInfo[]>([]);

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

    const requestVideo = async (video_name: number): Promise<void> => {
        try {
            const response = await fetch(`/req-video/${video_name}`, {
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

    const requestVideoList = async (): Promise<void> => {
        try {
            const response = await fetch("/req-video-list", {
                method: "GET",
            });
            if (response.ok) {
                const text = await response.text();
                // Parse the SSE format
                const parsedVideos = text
                    .trim()
                    .split("\n")
                    .filter((line) => line.startsWith("data:"))
                    .map((line) => JSON.parse(line.slice(5))); // Remove 'data:' prefix

                setVideos(parsedVideos);
            } else {
                console.error("Failed to fetch videos:", response.status);
            }
        } catch (error) {
            console.error("Error sending message:", error);
        }
    };

    useEffect(() => {
        // Initialize video player
        if (!videoRef.current) return;

        requestVideoList();

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

            if (mediaSourceRef.current?.readyState === "open") {
                sourceBufferRef.current = mediaSourceRef.current.addSourceBuffer(
                    'video/mp4; codecs="avc1.42E01E,mp4a.40.2"'
                );
            } else {
                console.error("MediaSource not ready");
            }

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
            sourceBufferRef.current?.addEventListener("updateend", () => {
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
            {/* <button
                onClick={() => requestVideo("dancing_pirate")}
                className="mb-4 px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600 transition-colors"
            >
                Request Video
            </button> */}

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

            <div className="p-4">
                <h1 className="text-2xl font-bold mb-6">Available Videos</h1>
                <div className="grid gap-4 md:grid-cols-2">
                    {videos.map((video, index) => (
                        <div key={index} className="border rounded-lg p-4 shadow-sm hover:shadow-md transition-shadow">
                            <h2 className="text-lg font-semibold mb-2">{video.title}</h2>
                            <button
                                onClick={() => requestVideo(video.id)}
                                className="mb-4 px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600 transition-colors"
                            >
                                Request Video
                            </button>
                            <p className="text-gray-600 mb-2">{video.description}</p>
                            <div className="text-sm text-gray-500">
                                <p>Type: {video.mime_type}</p>
                                <p>Duration: {video.duration}s</p>
                                <p>Added: {video.created_at}</p>
                                <p>ID: {video.id}</p>
                            </div>
                        </div>
                    ))}
                </div>
                {videos.length === 0 && <div className="text-center text-gray-500 mt-8">No videos available</div>}
            </div>
        </div>
    );
};

export default VideoStreamer;
