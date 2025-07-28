async function send_log(msg, log_level) {
    const response = await fetch(`${API_URL}/logger/`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
        },
        body: JSON.stringify({
            log_level: log_level,
            msg: msg,
        })
    });

    if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status} ${response.statusText}`);
    }
}

async function log_info(msg) {
    console.log(msg);
    send_log(msg, 'info');
}

async function log_error(msg) {
    console.error(msg);
    send_log(msg, 'error');
}

function selection_text() {
    const selection = document.getSelection();
    let text = "";
    if (selection) {
        text = selection.toString().trim();
    }
    return text;
}

function summary_selection() {
    const text = selection_text();
    log_info("summary_selection(): " + text);
    if (text !== "") {
        fetch(`${API_URL}/summary_query/${WINDOW_ID}/${encodeURIComponent(text)}`);
    }
}

function open_dpd_button(button_name = 'grammar') {
    // <meta data_key="data_pāpuṇāti_1">
    const metaElement = document.querySelector('meta[data_key]');
    if (!metaElement) {
        console.error('Meta element with data_key not found');
        return;
    }

    const dataKey = metaElement.getAttribute('data_key');
    if (!dataKey) {
        console.error('data_key attribute is empty');
        return;
    }

    // <a class="button" data-target="grammar_pāpuṇāti_1" href="#">
    // <div class="dpd content hidden" id="grammar_pāpuṇāti_1">
    const targetId = dataKey.replace('data_', button_name + '_');

    const button = document.querySelector(`a.button[data-target="${targetId}"]`);
    if (!button) {
        console.error(`Button with data-target="${targetId}" not found`);
        return;
    }

    button.classList.add('active');

    // Find the corresponding content div and remove 'hidden' class
    const contentDiv = document.getElementById(targetId);
    if (!contentDiv) {
        console.error(`Content div with id="${targetId}" not found`);
        return;
    }

    contentDiv.classList.remove('hidden');
}

document.addEventListener("DOMContentLoaded", function(_event) {
    open_dpd_button('grammar');
    open_dpd_button('examples');

    if (IS_MOBILE) {
        // On mobile in a WebView, there is no double click event, so listen to
        // selection change (from a long press action).
        // FIXME: avoid lookup when selection is changed by dragging the boundaries
        document.addEventListener("selectionchange", function (_event) {
            summary_selection();
        });

    } else {
        // On desktop, double click works to select a word and trigger a lookup.
        // Double click always triggers a lookup.
        document.addEventListener("dblclick", function (_event) {
            summary_selection();
        });
    }
});
