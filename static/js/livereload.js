if (window.EventSource) {
    const es = new EventSource('/__reload__');
    es.onmessage = function(e) {
        if (e.data === 'reload') {
            location.reload();
        }
    };
}