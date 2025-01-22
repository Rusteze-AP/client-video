import React, { useEffect, useRef, useState } from "react";

type VideoInfo = {
    title: string;
    description: string;
    duration: number;
    mime_type: string;
    created_at: string;
};

const VideoStreamer: React.FC = () => {
    const videoRef = useRef<HTMLVideoElement | null>(null);
    const mediaSourceRef = useRef<MediaSource | null>(null);
    const sourceBufferRef = useRef<SourceBuffer | null>(null);
    const eventSourceRef = useRef<EventSource | null>(null);
    const pendingBuffers = useRef<Uint8Array[]>([]);

    const [videos, setVideos] = useState<VideoInfo[]>([]);
    const [error, setError] = useState<string | null>(null);

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

    const appendBuffer = async (chunk: Uint8Array) => {
        if (!sourceBufferRef.current) {
            pendingBuffers.current.push(chunk);
            return;
        }

        try {
            if (!sourceBufferRef.current.updating) {
                sourceBufferRef.current.appendBuffer(chunk);
            } else {
                pendingBuffers.current.push(chunk);
            }
        } catch (error) {
            console.error("Error appending buffer:", error);
            setError("Error appending video data");
        }
    };

    const processBufferQueue = () => {
        if (!sourceBufferRef.current || sourceBufferRef.current.updating) {
            return;
        }

        if (pendingBuffers.current.length > 0) {
            const nextBuffer = pendingBuffers.current.shift();
            if (nextBuffer) {
                try {
                    sourceBufferRef.current.appendBuffer(nextBuffer);
                } catch (error) {
                    console.error("Error processing buffer queue:", error);
                }
            }
        }
    };

    const cleanupMediaSource = () => {
        setError(null);
        pendingBuffers.current = [];

        if (eventSourceRef.current) {
            eventSourceRef.current.close();
            eventSourceRef.current = null;
        }

        if (sourceBufferRef.current && mediaSourceRef.current) {
            try {
                if (mediaSourceRef.current.readyState === "open") {
                    mediaSourceRef.current.removeSourceBuffer(sourceBufferRef.current);
                }
            } catch (e) {
                console.warn("Error removing source buffer:", e);
            }
            sourceBufferRef.current = null;
        }

        if (mediaSourceRef.current?.readyState === "open") {
            try {
                mediaSourceRef.current.endOfStream();
            } catch (e) {
                console.warn("Error ending media stream:", e);
            }
        }

        if (videoRef.current) {
            videoRef.current.src = "";
        }
    };

    const initializeMediaSource = () => {
        cleanupMediaSource();

        try {
            mediaSourceRef.current = new MediaSource();
            const videoURL = URL.createObjectURL(mediaSourceRef.current);

            if (videoRef.current) {
                videoRef.current.src = videoURL;
            }

            mediaSourceRef.current.addEventListener("sourceopen", () => {
                if (!mediaSourceRef.current || mediaSourceRef.current.readyState !== "open") {
                    setError("Failed to initialize video player");
                    return;
                }

                try {
                    // Try different codec strings
                    const codecStrings = ['video/mp4; codecs="avc1.42E01E,mp4a.40.2"'];

                    let supported = false;
                    for (const codec of codecStrings) {
                        if (MediaSource.isTypeSupported(codec)) {
                            sourceBufferRef.current = mediaSourceRef.current.addSourceBuffer(codec);
                            supported = true;
                            break;
                        }
                    }

                    if (!supported) {
                        throw new Error("No supported codec found");
                    }

                    sourceBufferRef.current!.addEventListener("updateend", () => {
                        processBufferQueue();
                    });

                    setupEventSource();
                } catch (error) {
                    console.error("Error setting up media source:", error);
                    setError("Failed to initialize video codec");
                    cleanupMediaSource();
                }
            });
        } catch (error) {
            console.error("Error creating MediaSource:", error);
            setError("Failed to create video player");
        }
    };

    const setupEventSource = () => {
        eventSourceRef.current = new EventSource("/video-stream");

        eventSourceRef.current.onmessage = async (event: MessageEvent) => {
            try {
                const videoChunk = await decodeChunk(event.data);
                await appendBuffer(videoChunk);
            } catch (error) {
                console.error("Error processing video chunk:", error);
                setError("Error processing video data");
            }
        };

        eventSourceRef.current.onerror = (error: Event) => {
            console.error("EventSource error:", error);
            setError("Error streaming video data");
            cleanupMediaSource();
        };
    };

    const requestVideo = async (video_name: string): Promise<void> => {
        setError(null);

        try {
            const response = await fetch(`/req-video/${video_name}`, {
                method: "GET",
            });
            if (response.ok) {
                console.log("Video request successful:", await response.text());
                initializeMediaSource();
            } else {
                console.error("Failed to request video:", response.status);
                setError("Failed to request video");
            }
        } catch (error) {
            console.error("Error requesting video:", error);
            setError("Error requesting video");
        }
    };

    const requestVideoList = async (): Promise<void> => {
        try {
            const response = await fetch("/req-video-list", {
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
            } else {
                console.error("Failed to fetch videos:", response.status);
                setError("Failed to fetch video list");
            }
        } catch (error) {
            console.error("Error fetching video list:", error);
            setError("Error loading video list");
        }
    };

    useEffect(() => {
        requestVideoList();

        return () => {
            cleanupMediaSource();
        };
    }, []);

    return (
        <div className="w-full max-w-4xl mx-auto p-4">
            <div className="mb-4">
                <video ref={videoRef} className="w-full aspect-video bg-black" controls playsInline>
                    Your browser does not support HTML5 video.
                </video>
                {error && <div className="mt-2 p-2 bg-red-100 border border-red-400 text-red-700 rounded">{error}</div>}
            </div>

            <div className="p-4">
                <h1 className="text-2xl font-bold mb-6">Available Videos</h1>
                <div className="grid gap-4 md:grid-cols-2">
                    {videos.map((video, index) => (
                        <div key={index} className="border rounded-lg p-4 shadow-sm hover:shadow-md transition-shadow">
                            <h2 className="text-lg font-semibold mb-2">{video.title}</h2>
                            <button
                                onClick={() => requestVideo(video.title)}
                                className="mb-4 px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600 transition-colors disabled:bg-gray-400"
                            >
                                Play Video
                            </button>
                            <p className="text-gray-600 mb-2">{video.description}</p>
                            <div className="text-sm text-gray-500">
                                <p>Type: {video.mime_type}</p>
                                <p>Duration: {video.duration}s</p>
                                <p>Added: {video.created_at}</p>
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
