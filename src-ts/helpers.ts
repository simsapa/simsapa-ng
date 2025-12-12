// Regex pattern for matching sutta references in text
// Ported from backend/src/helpers.rs:25-27
const RE_ALL_BOOK_SUTTA_REF = /\b(DN|MN|SN|AN|Pv|Vv|Vism|iti|kp|khp|snp|th|thag|thig|ud|uda|dhp)[ .]*(\d[\d.:]*)\b/i;

// Import the confirmation modal function
import { show_external_link_confirmation } from "./confirm_modal";

/**
 * Send log message to backend logger
 */
async function send_log(msg: string, log_level: string): Promise<void> {
    const API_URL = (globalThis as any).API_URL || 'http://localhost:4848';
    const response = await fetch(`${API_URL}/logger`, {
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

async function log_info(msg: string): Promise<void> {
    console.log(msg);
    send_log(msg, 'info');
}

async function log_error(msg: string): Promise<void> {
    console.error(msg);
    send_log(msg, 'error');
}

/**
 * Opens an external URL using the backend API
 * This uses Qt's QDesktopServices to open the URL in the system browser
 */
async function open_external_url(url: string): Promise<void> {
    const API_URL = (globalThis as any).API_URL || 'http://localhost:4848';
    try {
        const response = await fetch(`${API_URL}/open_external_url`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({ url: url })
        });

        if (!response.ok) {
            log_error(`Failed to open external URL: ${response.status}`);
        }
    } catch (error) {
        const errorMsg = error instanceof Error ? error.message : String(error);
        log_error(`Error opening external URL: ${errorMsg}`);
    }
}

function show_transient_message(text: string, msg_div_id: string): void {
    const div = document.createElement('div');
    div.className = 'message';

    const content = document.createElement('div');
    content.className = 'msg-content';
    content.textContent = text;
    div.appendChild(content);

    let el = document.getElementById(msg_div_id);
    if (!el) {
        console.error("Cannot find: transient-messages");
        return;
    }

    el.appendChild(div);

    div.style.transition = 'opacity 1.5s ease-in-out';
    div.style.opacity = '1';

    // After 3 seconds, start fading out
    setTimeout(() => {
        div.style.opacity = '0';
    }, 1000);

    // After the transition ends, remove the div from the DOM
    div.addEventListener('transitionend', () => {
        div.remove();
    });
}

/**
 * Extract sutta UID from an anchor element based on priority:
 * 1. ssp:// protocol in href
 * 2. thebuddhaswords.net URL
 * 3. Text-based reference (e.g., "SN 56.11")
 * Returns the sutta UID or null if not found
 */
function extract_sutta_uid_from_link(anchor: HTMLAnchorElement): string | null {
    const href = anchor.getAttribute('href') || '';
    const text = anchor.textContent || '';

    // Priority 1: ssp:// protocol
    // Format: ssp://suttas/{uid} where uid can be sn47.8/en/thanissaro
    if (href.startsWith('ssp://')) {
        const match = href.match(/^ssp:\/\/suttas\/(.+)$/);
        if (match) {
            return match[1];
        }
    }

    // Priority 2: Suttacentral URL
    // Format: https://suttacentral.net/sn56.11/en/bodhi
    if (href.includes('suttacentral.net')) {
        const match = href.match(/suttacentral\.net\/(.+)$/);
        if (match) {
            return match[1];
        }
    }

    // Priority 3: thebuddhaswords.net URL
    // Format: https://thebuddhaswords.net/suttas/an4.41.html
    if (href.includes('thebuddhaswords.net')) {
        const match = href.match(/\/suttas\/([^.]+)\.html/);
        if (match) {
            // Extract the sutta code (e.g., an4.41) and append /pli/ms
            return `${match[1]}/pli/ms`;
        }
    }

    // Priority 4: Text-based reference (e.g., "SN 56.11" or "MN 10")
    // Convert colons to dots before matching, 'AN 5:114' to 'AN 5.114'
    const normalized_text = text.replace(/:/g, '.');
    const match = normalized_text.match(RE_ALL_BOOK_SUTTA_REF);
    if (match) {
        const book = match[1].toLowerCase();
        const number = match[2];
        // Construct UID with /pli/ms fallback
        return `${book}${number}/pli/ms`;
    }

    return null;
}

/**
 * Opens a sutta by UID through the backend API
 * Makes GET request to /open_sutta_window/{uid}
 */
async function open_sutta_by_uid(uid: string, original_url?: string): Promise<void> {
    // API_URL is defined as a global const in page.html template
    const API_URL = (globalThis as any).API_URL || 'http://localhost:4848';

    try {
        // Don't encode slashes - Rocket's <uid..> path parameter expects them as-is
        const url = `${API_URL}/open_sutta_window/${uid}`;
        const response = await fetch(url);

        if (response.status === 404) {
            // Sutta not found - show error dialog with option to open external link
            log_error(`Sutta not found: ${uid}`);
            let message = `Sutta not found in database: ${uid}`;
            if (original_url) {
                message += `\n\nOriginal URL: ${original_url}\n\nWould you like to open this link in your web browser?`;
                const confirmed = await show_external_link_confirmation(original_url);
                if (confirmed) {
                    window.open(original_url, '_blank');
                }
            } else {
                alert(message);
            }
        } else if (!response.ok) {
            log_error(`Failed to open sutta ${uid}: ${response.status}`);
        } else {
            log_info(`Successfully opened sutta ${uid}`);
        }
    } catch (error) {
        const errorMsg = error instanceof Error ? error.message : String(error);
        log_error(`Error opening sutta ${uid}: ${errorMsg}`);
    }
}

/**
 * Opens a sutta by UID in a new tab in the current window
 * Makes GET request to /open_sutta_tab/{window_id}/{uid}
 */
async function open_sutta_in_tab(uid: string, original_url?: string): Promise<void> {
    // API_URL and WINDOW_ID are defined as global consts in page.html template
    // Try multiple ways to access these globals
    const win = window as any;

    const API_URL = win.API_URL || (globalThis as any).API_URL || 'http://localhost:4848';
    const WINDOW_ID = win.WINDOW_ID || (globalThis as any).WINDOW_ID;

    // If WINDOW_ID is not defined, fall back to opening in a new window
    if (!WINDOW_ID || WINDOW_ID === '') {
        await log_error(`WINDOW_ID not defined, falling back to open_sutta_by_uid for uid='${uid}'`);
        await open_sutta_by_uid(uid, original_url);
        return;
    }

    await log_info(`open_sutta_in_tab: WINDOW_ID='${WINDOW_ID}', uid='${uid}'`);

    try {
        // Don't encode slashes - Rocket's <uid..> path parameter expects them as-is
        const url = `${API_URL}/open_sutta_tab/${WINDOW_ID}/${uid}`;
        const response = await fetch(url);

        if (response.status === 404) {
            // Sutta not found - show error dialog with option to open external link
            log_error(`Sutta not found: ${uid}`);
            let message = `Sutta not found in database: ${uid}`;
            if (original_url) {
                message += `\n\nOriginal URL: ${original_url}\n\nWould you like to open this link in your web browser?`;
                const confirmed = await show_external_link_confirmation(original_url);
                if (confirmed) {
                    await open_external_url(original_url);
                }
            } else {
                alert(message);
            }
        } else if (!response.ok) {
            log_error(`Failed to open sutta ${uid}: ${response.status}`);
        }
    } catch (error) {
        const errorMsg = error instanceof Error ? error.message : String(error);
        log_error(`Error opening sutta ${uid}: ${errorMsg}`);
    }
}

/**
 * Opens a book page in a new tab in the current window
 * Makes GET request to /open_book_page_tab/{window_id} with the book page URL
 */
async function open_book_page_in_tab(book_page_url: string): Promise<void> {
    // API_URL and WINDOW_ID are defined as global consts in page.html template
    const win = window as any;
    const API_URL = win.API_URL || (globalThis as any).API_URL || 'http://localhost:4848';
    const WINDOW_ID = win.WINDOW_ID || (globalThis as any).WINDOW_ID;

    // If WINDOW_ID is not defined, just navigate to the page normally
    if (!WINDOW_ID || WINDOW_ID === '') {
        await log_info(`WINDOW_ID not defined, navigating to book page: ${book_page_url}`);
        window.location.href = book_page_url;
        return;
    }

    await log_info(`open_book_page_in_tab: WINDOW_ID='${WINDOW_ID}', url='${book_page_url}'`);

    try {
        // Send the book page URL to the backend to open in a new tab
        const url = `${API_URL}/open_book_page_tab/${WINDOW_ID}`;
        const response = await fetch(url, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({ book_page_url: book_page_url })
        });

        if (!response.ok) {
            await log_error(`Failed to open book page ${book_page_url}: ${response.status}`);
        }
    } catch (error) {
        const errorMsg = error instanceof Error ? error.message : String(error);
        await log_error(`Error opening book page ${book_page_url}: ${errorMsg}`);
    }
}

/**
 * Shows a confirmation dialog for external links
 * Returns a Promise that resolves to true if user confirms, false otherwise
 * NOTE: This function is now imported from confirm_modal.ts
 * Import statement at top of file: import { show_external_link_confirmation } from "./confirm_modal";
 */

/**
 * Handles link clicks and classifies them into:
 * - Anchor links (same page) - default behavior
 * - Sutta links - call API to open sutta in tab
 * - Book page links (to different resource) - open in new tab
 * - External links - show confirmation before opening
 */
async function handle_link_click(event: MouseEvent): Promise<void> {
    const target = event.target as HTMLElement;

    // Find the anchor element (might be a child element that was clicked)
    let anchor: HTMLAnchorElement | null = null;
    if (target.tagName === 'A') {
        anchor = target as HTMLAnchorElement;
    } else {
        anchor = target.closest('a');
    }

    if (!anchor) {
        return;
    }

    const href = anchor.getAttribute('href') || '';

    // Case 1: Anchor links (same page navigation) - allow default behavior
    if (href.startsWith('#')) {
        return;
    }

    // Case 2: Try to extract sutta UID
    const sutta_uid = extract_sutta_uid_from_link(anchor);
    if (sutta_uid) {
        event.preventDefault();
        // Pass the original href so we can offer to open it if sutta not found
        // Open in tab instead of new window
        await open_sutta_in_tab(sutta_uid, href);
        return;
    }

    // Case 3: Book page links to different resources
    // Format: /book_pages/<book_uid>/<resource_path>
    if (href.startsWith('/book_pages/')) {
        const current_url = window.location.pathname;
        // Extract the path without the fragment
        const href_without_fragment = href.split('#')[0];
        const current_without_fragment = current_url.split('#')[0];

        // If it's a link to a different resource (not just an anchor on same page)
        if (href_without_fragment !== current_without_fragment && href_without_fragment !== '') {
            event.preventDefault();
            await open_book_page_in_tab(href);
            return;
        }
        // Otherwise, it's either an anchor on the same page or the same page, allow default behavior
        return;
    }

    // Case 4: External links - show confirmation
    if (href.startsWith('http://') || href.startsWith('https://')) {
        event.preventDefault();
        const confirmed = await show_external_link_confirmation(href);
        if (confirmed) {
            await open_external_url(href);
        }
        return;
    }
}

export {
    show_transient_message,
    extract_sutta_uid_from_link,
    open_sutta_by_uid,
    open_sutta_in_tab,
    open_book_page_in_tab,
    open_external_url,
    handle_link_click,
    log_info,
    log_error,
}
