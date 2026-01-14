import * as h from "./helpers";
import { findManager } from "./find";
import "./confirm_modal";
import "./footnote_modal";
import "./invalid_link_modal";

/**
 * Attach link handlers to all links within a specific element
 * This allows us to handle sutta links, external links, etc. consistently
 */
export function attach_link_handlers_to_element(element: HTMLElement): void {
    const links = element.querySelectorAll('a');
    links.forEach(link => {
        // Check if this link is a sutta link and add the class
        const suttaUid = h.extract_sutta_uid_from_link(link as HTMLAnchorElement);
        if (suttaUid) {
            link.classList.add('sutta-link');
        }
        link.addEventListener('click', h.handle_link_click);
    });
}

function attach_link_handlers(): void {
    // h.log_info('[simsapa] Attaching link handlers');

    // Check if this is a sutta page (has ssp_content div)
    const sspContent = document.getElementById('ssp_content');

    if (sspContent) {
        // Sutta page - only add handlers to links within ssp_content
        attach_link_handlers_to_element(sspContent);
    } else {
        // Not a sutta page - add handlers to all links
        const bodyElement = document.body;
        if (bodyElement) {
            attach_link_handlers_to_element(bodyElement);
        }
    }
}

document.SSP = {
    show_transient_message: h.show_transient_message,
    find: findManager,
    attach_link_handlers: attach_link_handlers,
};

document.addEventListener('DOMContentLoaded', () => {
    // h.log_info('[simsapa] DOMContentLoaded event fired');
    attach_link_handlers();
});
