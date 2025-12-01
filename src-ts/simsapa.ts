import * as h from "./helpers";
import { findManager } from "./find";

function attach_link_handlers(): void {
    h.log_info('[simsapa] Attaching link handlers');

    // Check if this is a sutta page (has ssp_content div)
    const sspContent = document.getElementById('ssp_content');

    if (sspContent) {
        // Sutta page - only add handlers to links within ssp_content
        const links = sspContent.querySelectorAll('a');
        links.forEach(link => {
            link.addEventListener('click', h.handle_link_click);
        });
    } else {
        // Not a sutta page - add handlers to all links
        const links = document.querySelectorAll('a');
        links.forEach(link => {
            link.addEventListener('click', h.handle_link_click);
        });
    }
}

document.SSP = {
    show_transient_message: h.show_transient_message,
    find: findManager,
    attach_link_handlers: attach_link_handlers,
};

document.addEventListener('DOMContentLoaded', () => {
    h.log_info('[simsapa] DOMContentLoaded event fired');
    attach_link_handlers();
});
