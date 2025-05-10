function selection_text() {
    const selection = document.getSelection();
    let text = "";
    if (selection) {
        text = selection.toString().trim();
    }
    return text;
}

function lookup_selection() {
    const selected_text = window.getSelection().toString().trim();

    if (!selected_text) {
        console.log('No text selected');
        return;
    }

    fetch(`${API_URL}/lookup_window_query/${encodeURIComponent(selected_text)}`);
}

function summary_selection() {
    const text = selection_text();
    if (text !== "") {
        fetch(`${API_URL}/summary_query/${WINDOW_ID}/${encodeURIComponent(text)}`);
    }
}

// TODO: Both Double click and selection event runs the summary search, lookup query is stated from the summary UI.
// TODO: Allow the user to configure which action should run a lookup query.
function page_dblclick(_event) {
    summary_selection();
}

function page_selection(_event) {
    summary_selection();
}

document.addEventListener("DOMContentLoaded", function(_event) {
    document.addEventListener("dblclick", page_dblclick);
    document.addEventListener("selectionchange", page_selection);
});
