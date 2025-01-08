import { useEffect, useRef } from "react";

function IdViewer() {
    const clientIdRef = useRef<HTMLSpanElement | null>(null);

    useEffect(() => {
        // Handle client ID updates
        const clientEvtSource = new EventSource("/events");
        clientEvtSource.onmessage = (event: MessageEvent) => {
            if (clientIdRef.current) {
                clientIdRef.current.textContent = event.data;
            }
        };

        // Cleanup
        return () => {
            clientEvtSource.close();
        };
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
