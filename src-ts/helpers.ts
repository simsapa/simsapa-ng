// Regex pattern for matching sutta references in text
// Ported from backend/src/helpers.rs:25-27
const RE_ALL_BOOK_SUTTA_REF = /\b(DN|MN|SN|AN|Pv|Vv|Vism|iti|kp|khp|snp|th|thag|thig|ud|uda|dhp)[ .]*(\d[\d.:]*)\b/i;

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
 * Makes GET request to /open_sutta/{uid}
 */
async function open_sutta_by_uid(uid: string, original_url?: string): Promise<void> {
    // API_URL is defined as a global const in page.html template
    const API_URL = (globalThis as any).API_URL || 'http://localhost:4848';

    try {
        // Don't encode slashes - Rocket's <uid..> path parameter expects them as-is
        const url = `${API_URL}/open_sutta/${uid}`;
        const response = await fetch(url);

        if (response.status === 404) {
            // Sutta not found - show error dialog with option to open external link
            log_error(`Sutta not found: ${uid}`);
            let message = `Sutta not found in database: ${uid}`;
            if (original_url) {
                message += `\n\nOriginal URL: ${original_url}\n\nWould you like to open this link in your web browser?`;
                if (confirm(message)) {
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
 * Shows a confirmation dialog for external links
 * Returns true if user confirms, false otherwise
 */
function show_external_link_confirmation(url: string): boolean {
    return confirm(`Open this link in your web browser?\n\n${url}`);
}

/**
 * Handles link clicks and classifies them into:
 * - Anchor links (same page) - default behavior
 * - Sutta links - call API to open sutta window
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
        await open_sutta_by_uid(sutta_uid, href);
        return;
    }

    // Case 3: External links - show confirmation
    if (href.startsWith('http://') || href.startsWith('https://')) {
        event.preventDefault();
        if (show_external_link_confirmation(href)) {
            window.open(href, '_blank');
        }
        return;
    }
}

export {
    show_transient_message,
    extract_sutta_uid_from_link,
    open_sutta_by_uid,
    show_external_link_confirmation,
    handle_link_click,
    log_info,
    log_error,
}
