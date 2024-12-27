function init() {
    const evtSource = new EventSource("/events");
    evtSource.onmessage = function (event) {
        document.getElementById("client-id").textContent = event.data;
    };
}

init();
