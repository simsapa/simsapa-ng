function lookup_selection() {
    const selected_text = window.getSelection().toString().trim();

    if (!selected_text) {
        console.log('No text selected');
        return;
    }

    fetch(`${API_URL}/lookup_window_query/${encodeURIComponent(selected_text)}`);
}

function page_dblclick(_event) {
    lookup_selection();
}

document.addEventListener("DOMContentLoaded", function(_event) {
    let body = document.querySelector("body");
    if (body) {
        body.addEventListener("dblclick", page_dblclick);
    }
});
