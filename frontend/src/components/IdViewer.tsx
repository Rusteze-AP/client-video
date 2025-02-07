import React, { useEffect, useState } from "react";

const IdViewer: React.FC = () => {
    const [clientId, setClientId] = useState<string>("Loading...");
    const [isLoading, setIsLoading] = useState<boolean>(true);
    const [error, setError] = useState<string | null>(null);

    const get_id = async () => {
        try {
            setIsLoading(true);
            const response = await fetch("/get-id");

            if (!response.ok) {
                throw new Error("Failed to fetch client ID");
            }

            const data = await response.json();
            setClientId(data);
            setError(null);
        } catch (error) {
            console.error(error);
            setError(error instanceof Error ? error.message : "An unknown error occurred");
            setClientId("Error");
        } finally {
            setIsLoading(false);
        }
    };

    useEffect(() => {
        get_id();

        // const clientEvtSource = new EventSource("/events");
        // clientEvtSource.onmessage = (event: MessageEvent) => {
        //     setClientId(event.data);
        // };

        // Optional: Add periodic refresh
        // const intervalId = setInterval(get_id, 60000); // Refresh every minute

        // return () => {
        // clearInterval(intervalId);
        // clientEvtSource.close();
        // };
    }, []);

    return (
        <div className="bg-gray-800 p-4 shadow-md">
            <h2 className="text-xl font-bold mb-2 text-gray-200">Client Video</h2>
            <div className="flex items-center space-x-2">
                <div
                    className={`w-3 h-3 rounded-full ${
                        isLoading ? "bg-yellow-500" : error ? "bg-red-500" : "bg-green-500"
                    }`}
                />
                <span
                    className={`font-mono ${isLoading ? "text-yellow-400" : error ? "text-red-400" : "text-blue-400"}`}
                >
                    {clientId}
                </span>
                {isLoading && <div className="animate-spin text-gray-400">↻</div>}
            </div>
            {error && <p className="text-sm text-red-400 mt-2">{error}</p>}
        </div>
    );
};

export default IdViewer;
