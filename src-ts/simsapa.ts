import * as h from "./helpers";
import { findManager } from "./find";
import "./confirm_modal";
import "./footnote_modal";
import "./invalid_link_modal";
import { footnote_bottom_bar } from "./footnote_bottom_bar";

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

// Scroll management functions
const scrollManager = {
    // Scroll by a small amount (for j/k keys)
    scrollSmallUp: function(): void {
        window.scrollBy({ top: -80, behavior: 'smooth' });
    },
    scrollSmallDown: function(): void {
        window.scrollBy({ top: 80, behavior: 'smooth' });
    },
    // Scroll by half a page (for Ctrl+U/Ctrl+D)
    scrollHalfPageUp: function(): void {
        const halfPage = window.innerHeight / 2;
        window.scrollBy({ top: -halfPage, behavior: 'smooth' });
    },
    scrollHalfPageDown: function(): void {
        const halfPage = window.innerHeight / 2;
        window.scrollBy({ top: halfPage, behavior: 'smooth' });
    },
    // Scroll by a full page (for Space/Shift+Space/Page Up/Page Down)
    scrollPageUp: function(): void {
        const page = window.innerHeight - 50; // Leave some overlap
        window.scrollBy({ top: -page, behavior: 'smooth' });
    },
    scrollPageDown: function(): void {
        const page = window.innerHeight - 50; // Leave some overlap
        window.scrollBy({ top: page, behavior: 'smooth' });
    },
    // Scroll to beginning/end (for Home/End)
    scrollToTop: function(): void {
        window.scrollTo({ top: 0, behavior: 'smooth' });
    },
    scrollToBottom: function(): void {
        window.scrollTo({ top: document.body.scrollHeight, behavior: 'smooth' });
    },
};

document.SSP = {
    show_transient_message: h.show_transient_message,
    find: findManager,
    attach_link_handlers: attach_link_handlers,
    show_bottom_footnotes: true, // Default to true, will be updated from QML
    scroll: scrollManager,
};

/**
 * Refresh the footnote bottom bar based on current settings
 * Called from QML when the show_bottom_footnotes setting changes
 */
function footnote_bottom_bar_refresh(): void {
    if (!document.SSP.show_bottom_footnotes) {
        // Setting is disabled, destroy the footnote bar
        footnote_bottom_bar.destroy();
    } else {
        // Setting is enabled, refresh the footnote bar
        footnote_bottom_bar.refresh();
    }
}

// Expose to window for QML access
(window as any).footnote_bottom_bar_refresh = footnote_bottom_bar_refresh;

document.addEventListener('DOMContentLoaded', () => {
    // h.log_info('[simsapa] DOMContentLoaded event fired');
    attach_link_handlers();

    // Initialize footnote bottom bar for sutta pages if enabled
    const sspContent = document.getElementById('ssp_content');
    if (sspContent && document.SSP.show_bottom_footnotes) {
        footnote_bottom_bar.init();
    }
});
