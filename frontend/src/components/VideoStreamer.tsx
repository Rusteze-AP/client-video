import React, { useEffect, useRef, useState } from "react";
import { Download, RefreshCw } from "lucide-react";

type VideoMetadata = {
    id: number;
    title: string;
    description: string;
    duration: number;
    mime_type: string;
    created_at: string;
};

type ServerVideos = {
    serverId: number;
    videos: VideoMetadata[];
};

enum FSMStatus {
    ServerNotFound = "ServerNotFound",
    NotSubscribedToServer = "NotSubscribedToServer",
    SubscribedToServer = "SubscribedToServer",
    Terminated = "Terminated",
}

const VideoStreamer: React.FC = () => {
    const videoRef = useRef<HTMLVideoElement | null>(null);
    const mediaSourceRef = useRef<MediaSource | null>(null);
    const sourceBufferRef = useRef<SourceBuffer | null>(null);

    const [videos, setVideos] = useState<VideoMetadata[]>([]);
    const [videosFromServer, setVideosFromServer] = useState<ServerVideos[]>([]);
    const [fsmStatus, setFsmStatus] = useState<string>("Setup");
    const [selectedVideo, setSelectedVideo] = useState<VideoMetadata | null>(null);
    const [errorMessage, setErrorMessage] = useState<string | null>(null);

    const chunkQueue: Uint8Array[] = [];
    let isAppending = false;

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

    const requestVideo = async (video_id: number): Promise<void> => {
        try {
            // Reset video and buffer
            if (videoRef.current && mediaSourceRef.current) {
                videoRef.current.pause();
                videoRef.current.src = "";
                mediaSourceRef.current = new MediaSource();
                const newVideoURL = URL.createObjectURL(mediaSourceRef.current);
                videoRef.current.src = newVideoURL;

                mediaSourceRef.current.addEventListener("sourceopen", () => {
                    sourceBufferRef.current = mediaSourceRef.current!.addSourceBuffer(
                        'video/mp4; codecs="avc1.42E01E,mp4a.40.2"'
                    );
                });
            }

            // Request new video
            const response = await fetch(`/req-video/${video_id}`, {
                method: "GET",
            });

            if (!response.ok) {
                console.error("Failed to fetch video:", response.status);
                setErrorMessage("Failed to fetch video");
            } else {
                setErrorMessage(null);
            }
        } catch (error) {
            console.error("Error requesting video:", error);
            setErrorMessage("Failed to fetch video");
        }
    };

    const requestVideoList = async (): Promise<void> => {
        try {
            const response = await fetch("/req-video-list-from-db", {
                method: "GET",
            });
            if (response.ok) {
                const text = await response.text();
                const parsedVideos = text
                    .trim()
                    .split("\n")
                    .filter((line) => line.startsWith("data:"))
                    .map((line) => JSON.parse(line.slice(5)));

                setVideos(parsedVideos);
                setErrorMessage(null);
            } else {
                console.error("Failed to fetch videos:", response.status);
                setErrorMessage("Failed to fetch video list from db");
            }
        } catch (error) {
            console.error("Error sending message:", error);
            setErrorMessage("Failed to fetch video list from db");
        }
    };

    const requestVideoListFromServer = async (): Promise<void> => {
        try {
            const response = await fetch("/req-video-list-from-server", {
                method: "GET",
            });
            if (response.ok) {
                setErrorMessage(null);
            } else {
                console.error("Failed to fetch videos from server:", response.status);
                setErrorMessage("Failed to fetch video list from server");
            }
        } catch (error) {
            console.error("Error sending message:", error);
            setErrorMessage("Failed to fetch video list from server");
        }
    };

    const requestFlooding = async (): Promise<void> => {
        try {
            const response = await fetch("/flood-req", {
                method: "GET",
            });
            if (!response.ok) {
                console.error("Failed to send message:", response.status);
                setErrorMessage("Failed to send flood_req");
            } else {
                setErrorMessage(null);
            }
        } catch (error) {
            console.error("Error sending message:", error);
            setErrorMessage("Failed to send flood_req");
        }
    };

    useEffect(() => {
        if (fsmStatus === FSMStatus.SubscribedToServer) {
            requestVideoListFromServer();
        }
    }, [fsmStatus]);

    useEffect(() => {
        if (!videoRef.current) return;

        requestVideoList();

        // New EventSource for video list from server
        const videoListFromServer = new EventSource("/video-list-from-server");
        videoListFromServer.onmessage = function (event) {
            try {
                const data: [number, VideoMetadata[]] = JSON.parse(event.data);
                const serverId = data[0];
                const videos = data[1];

                setVideosFromServer((prev) => {
                    const existingIndex = prev.findIndex((entry) => entry.serverId === serverId);
                    if (existingIndex !== -1) {
                        const updated = [...prev];
                        updated[existingIndex] = { serverId, videos };
                        return updated;
                    } else {
                        return [...prev, { serverId, videos }];
                    }
                });
            } catch (error) {
                console.error("Error parsing video list from server:", error);
                setVideosFromServer([]);
                setErrorMessage("Failed to fetch video list from server");
            }
        };

        // New EventSource for fsm
        const fsmStatusSource = new EventSource("/fsm-status");
        fsmStatusSource.onmessage = function (event) {
            try {
                setFsmStatus(event.data);
            } catch (error) {
                console.error("Error parsing FSM status:", error);
                setFsmStatus("Setup");
                setErrorMessage("Failed to fetch FSM status");
            }
        };

        mediaSourceRef.current = new MediaSource();
        const videoURL = URL.createObjectURL(mediaSourceRef.current);

        videoRef.current.src = videoURL;

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
                    chunkQueue.push(videoChunk); // Queue the chunk
                    processChunkQueue(); // Try to process the queue
                } catch (error) {
                    /* ... */
                }
            };

            const processChunkQueue = async () => {
                if (isAppending || chunkQueue.length === 0 || !sourceBufferRef.current) return;

                isAppending = true;
                try {
                    const chunk = chunkQueue.shift(); // Get the next chunk
                    if (chunk) {
                        if (sourceBufferRef.current.updating) {
                            // Should not happen, but a safeguard
                            await new Promise<void>((resolve) => {
                                sourceBufferRef.current?.addEventListener("updateend", () => resolve(), { once: true });
                            });
                        }

                        sourceBufferRef.current.appendBuffer(chunk);

                        await new Promise<void>((resolve) => {
                            sourceBufferRef.current?.addEventListener("updateend", () => resolve(), { once: true });
                        });
                    }
                } catch (error) {
                    console.error("Error appending chunk:", error);
                    setErrorMessage("Failed to append video chunk");
                    chunkQueue.length = 0; // Clear the queue to prevent further errors
                    // evtSource.close(); // Close the event source
                } finally {
                    isAppending = false;
                    processChunkQueue(); // Process the next chunk if available
                }
            };

            evtSource.onerror = (error: Event) => {
                console.error("EventSource error:", error);
                setErrorMessage("Failed to receive video stream");
                evtSource.close();
            };

            // Handle buffer updates
            sourceBufferRef.current?.addEventListener("updateend", () => {
                if (!sourceBufferRef.current || !videoRef.current) return;

                if (sourceBufferRef.current.buffered.length > 0) {
                    const bufferEnd = sourceBufferRef.current.buffered.end(0);
                    const currentTime = videoRef.current.currentTime;

                    if (bufferEnd - currentTime > 30) {
                        sourceBufferRef.current.remove(0, currentTime - 10);
                    }
                }
            });
        });

        // Cleanup
        return () => {
            if (mediaSourceRef.current?.readyState === "open") {
                mediaSourceRef.current.endOfStream();
            }
            URL.revokeObjectURL(videoURL);
        };
    }, []);

    return (
        <div className="min-h-screen bg-gray-900 text-white">
            <div className="container mx-auto px-4 py-8">
                {/* Error popup */}
                {errorMessage && (
                    <div className="fixed bottom-4 left-4 bg-red-500 text-white px-4 py-2 rounded-lg shadow-lg flex items-center justify-between space-x-4 w-max">
                        <p className="text-sm">{errorMessage}</p>
                        <button
                            onClick={() => setErrorMessage(null)}
                            className="text-white font-bold text-lg leading-none"
                        >
                            ✖
                        </button>
                    </div>
                )}

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
                        {/* Refresh Video List */}
                        <button
                            onClick={() => requestVideoListFromServer()}
                            className="text-blue-400 hover:text-blue-300 transition-colors"
                        >
                            <RefreshCw size={16} />
                        </button>
                        {/* Request flooding */}
                        <button
                            onClick={requestFlooding}
                            className="bg-blue-600 hover:bg-blue-500 text-white px-3 py-1 rounded-full text-sm"
                        >
                            Send flood_req
                        </button>
                    </div>
                </div>

                {/* Video Player Section */}
                <div className="grid md:grid-cols-3 gap-8">
                    {/* Video Player */}
                    <div className="md:col-span-2 rounded-xl overflow-hidden shadow-2xl">
                        <video ref={videoRef} className="w-full bg-black" controls preload="auto">
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
                            <div className="space-y-6">
                                {videosFromServer.map(({ serverId, videos }) => (
                                    <div key={serverId} className="border border-gray-600 p-4 rounded-lg">
                                        <h3 className="text-xl font-semibold text-gray-300">Server {serverId}</h3>
                                        <div className="space-y-4 mt-2">
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
                                            {videos.length === 0 && (
                                                <p className="text-gray-500 text-center">No videos from this server</p>
                                            )}
                                        </div>
                                    </div>
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
