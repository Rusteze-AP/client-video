import React, { useEffect, useRef, useState } from "react";
import videojs from "video.js";
import Player from "video.js/dist/types/player";
import "video.js/dist/video-js.css";
import { Download, RefreshCw } from "lucide-react";

interface VideoJSOptions {
    controls: boolean;
    responsive: boolean;
    fluid: boolean;
    sources: {
        src: string;
        type: string;
    }[];
}

type VideoMetadata = {
    id: number;
    title: string;
    description: string;
    duration: number;
    mime_type: string;
    created_at: string;
};

enum FSMStatus {
    ServerNotFound = "ServerNotFound",
    NotSubscribedToServer = "NotSubscribedToServer",
    SubscribedToServer = "SubscribedToServer",
    Terminated = "Terminated",
}

const VideoStreamer: React.FC = () => {
    const videoRef = useRef<HTMLVideoElement | null>(null);
    const playerRef = useRef<Player | null>(null);
    const mediaSourceRef = useRef<MediaSource | null>(null);
    const sourceBufferRef = useRef<SourceBuffer | null>(null);

    const [videos, setVideos] = useState<VideoMetadata[]>([]);
    const [videosFromServer, setVideosFromServer] = useState<VideoMetadata[]>([]);
    const [fsmStatus, setFsmStatus] = useState<string>("Setup");
    const [selectedVideo, setSelectedVideo] = useState<VideoMetadata | null>(null);

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
            if (!response.ok) {
                console.error("Failed to send message:", response.status);
            }
        } catch (error) {
            console.error("Error sending message:", error);
        }
    };

    const requestVideoList = async (): Promise<void> => {
        try {
            const response = await fetch("/req-video-list-from-db", {
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

    const requestVideoListFromServer = async (): Promise<void> => {
        try {
            const response = await fetch("/req-video-list-from-server", {
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

                setVideosFromServer(parsedVideos);
            } else {
                console.error("Failed to fetch videos from server:", response.status);
            }
        } catch (error) {
            console.error("Error sending message:", error);
        }
    };

    useEffect(() => {
        if (fsmStatus === FSMStatus.SubscribedToServer) {
            requestVideoListFromServer();
        }
    }, [fsmStatus]);

    useEffect(() => {
        // Initialize video player
        if (!videoRef.current) return;

        requestVideoList();
        // requestVideoListFromServer();

        // New EventSource for video list from server
        const videoListFromServer = new EventSource("/video-list-from-server");
        videoListFromServer.onmessage = function (event) {
            try {
                const data = JSON.parse(event.data);
                setVideosFromServer(data);
            } catch (error) {
                console.error("Error parsing video list from server:", error);
                setVideosFromServer([]);
            }
        };

        // New EventSource for fsm
        const fsmStatusSource = new EventSource("/fsm-status");
        fsmStatusSource.onmessage = function (event) {
            try {
                // Directly set the FSM status string
                setFsmStatus(event.data);
            } catch (error) {
                console.error("Error parsing FSM status:", error);
                setFsmStatus("Setup"); // Default to Setup on error
            }
        };

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
        <div className="min-h-screen bg-gray-900 text-white">
            <div className="container mx-auto px-4 py-8">
                {/* Status Indicator */}
                <div className="fixed top-4 right-4 z-50">
                    <div className="flex items-center space-x-2 bg-gray-800 rounded-full px-4 py-2 shadow-lg">
                        <div
                            className={`w-3 h-3 rounded-full ${
                                fsmStatus === FSMStatus.NotSubscribedToServer
                                    ? "bg-yellow-500"
                                    : fsmStatus === FSMStatus.SubscribedToServer
                                    ? "bg-green-500"
                                    : fsmStatus === FSMStatus.Terminated
                                    ? "bg-red-500"
                                    : "bg-gray-500"
                            }`}
                        />
                        <span className="text-sm font-medium">{fsmStatus}</span>
                        <button
                            onClick={() => requestVideoListFromServer()}
                            className="text-blue-400 hover:text-blue-300 transition-colors"
                        >
                            <RefreshCw size={16} />
                        </button>
                    </div>
                </div>

                {/* Video Player Section */}
                <div className="grid md:grid-cols-3 gap-8">
                    {/* Video Player */}
                    <div className="md:col-span-2 rounded-xl overflow-hidden shadow-2xl">
                        <video
                            ref={videoRef}
                            className="video-js vjs-big-play-centered w-full"
                            controls
                            preload="auto"
                            data-setup="{}"
                        >
                            <p className="vjs-no-js">To view this video, please enable JavaScript.</p>
                        </video>

                        {selectedVideo && (
                            <div className="p-4 bg-gray-700">
                                <h2 className="text-xl font-bold">{selectedVideo.title}</h2>
                                <p className="text-gray-300">{selectedVideo.description}</p>
                            </div>
                        )}
                    </div>

                    {/* Video Lists */}
                    <div className="space-y-6">
                        <div>
                            <h2 className="text-2xl font-bold mb-4 text-gray-200">Local Videos</h2>
                            <div className="space-y-4">
                                {videos.map((video) => (
                                    <VideoCard
                                        key={video.id}
                                        video={video}
                                        onSelect={() => {
                                            setSelectedVideo(video);
                                            requestVideo(video.id);
                                        }}
                                    />
                                ))}
                                {videos.length === 0 && <p className="text-gray-500 text-center">No local videos</p>}
                            </div>
                        </div>

                        <div>
                            <h2 className="text-2xl font-bold mb-4 text-gray-200">Server Videos</h2>
                            <div className="space-y-4">
                                {videosFromServer.map((video) => (
                                    <VideoCard
                                        key={video.id}
                                        video={video}
                                        onSelect={() => {
                                            setSelectedVideo(video);
                                            requestVideo(video.id);
                                        }}
                                    />
                                ))}
                                {videosFromServer.length === 0 && (
                                    <p className="text-gray-500 text-center">No server videos</p>
                                )}
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
};

// Separate component for video cards
const VideoCard: React.FC<{
    video: VideoMetadata;
    onSelect: () => void;
}> = ({ video, onSelect }) => {
    return (
        <div
            className="bg-gray-800 rounded-lg p-4 hover:bg-gray-700 transition-colors cursor-pointer group"
            onClick={onSelect}
        >
            <div className="flex justify-between items-center mb-2">
                <h3 className="text-lg font-semibold text-gray-200 group-hover:text-blue-400 transition-colors">
                    {video.title}
                </h3>
                <Download size={20} className="text-gray-500 hover:text-blue-500 transition-colors" />
            </div>
            <p className="text-gray-400 text-sm mb-2">{video.description}</p>
            <div className="flex justify-between text-xs text-gray-500">
                <span>{video.mime_type}</span>
                <span>{video.duration}s</span>
            </div>
        </div>
    );
};

export default VideoStreamer;
