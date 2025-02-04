import { useEffect, useRef } from "react";

function IdViewer() {
    const clientIdRef = useRef<HTMLSpanElement | null>(null);

    const get_id = async () => {
        try {
            const response = await fetch("/get-id");
            if (!response.ok) {
                throw new Error("Failed to fetch client ID");
            }
            const data = await response.json();
            if (clientIdRef.current) {
                clientIdRef.current.textContent = data;
            }
        } catch (error) {
            console.error(error);
        }
    };

    useEffect(() => {
        // Handle client ID updates
        // const clientEvtSource = new EventSource("/events");
        // clientEvtSource.onmessage = (event: MessageEvent) => {
        //     if (clientIdRef.current) {
        //         clientIdRef.current.textContent = event.data;
        //     }
        // };

        get_id();
    }, []);

    return (
        <>
            <h1 className="text-xl font-bold mb-4">
                Client ID: <span ref={clientIdRef}>Loading...</span>
            </h1>
        </>
    );
}

export default IdViewer;
