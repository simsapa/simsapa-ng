/**
 * Footnote bottom bar for displaying visible footnote definitions
 * Automatically shows definitions for footnotes currently in the viewport
 */

import { extract_footnote_number, get_footnote_content, is_footnote_link } from "./footnote_modal";
import { attach_link_handlers_to_element } from "./simsapa";

interface VisibleFootnote {
    number: string;
    content: string;
    id: string;
}

class FootnoteBottomBar {
    private container: HTMLElement | null = null;
    private observer: IntersectionObserver | null = null;
    private visible_footnotes: Map<string, VisibleFootnote> = new Map();
    private initialized: boolean = false;

    constructor() {
        // Don't initialize in constructor - wait for DOM to be ready
    }

    /**
     * Initialize the footnote bottom bar
     * Creates the container and sets up the Intersection Observer
     */
    public init(): void {
        if (this.initialized) {
            return;
        }

        // Get or create the container element
        this.container = document.getElementById('footnoteBottomBar');
        if (!this.container) {
            console.error('[FootnoteBottomBar] Container element not found in DOM');
            return;
        }

        // Set up Intersection Observer to track visible footnote references
        this.setup_observer();

        // Find all footnote references on the page and observe them
        this.observe_all_footnotes();

        this.initialized = true;
    }

    /**
     * Set up the Intersection Observer to track when footnote references enter/leave viewport
     */
    private setup_observer(): void {
        // Observer options: trigger when 50% of the element is visible
        const options: IntersectionObserverInit = {
            root: null, // Use viewport
            rootMargin: '0px',
            threshold: 0.5 // 50% visible
        };

        this.observer = new IntersectionObserver((entries) => {
            entries.forEach(entry => {
                const anchor = entry.target as HTMLAnchorElement;
                const footnote_id = is_footnote_link(anchor);

                if (!footnote_id) {
                    return;
                }

                if (entry.isIntersecting) {
                    // Footnote reference is visible, add it to the display
                    this.add_visible_footnote(anchor, footnote_id);
                } else {
                    // Footnote reference is no longer visible, remove it
                    this.remove_visible_footnote(footnote_id);
                }
            });
        }, options);
    }

    /**
     * Find all footnote reference links on the page and observe them
     */
    private observe_all_footnotes(): void {
        if (!this.observer) {
            return;
        }

        // Find all potential footnote links
        // They are either:
        // 1. Inside elements with class "footnote-reference"
        // 2. Inside <span class="fn">
        const footnote_selectors = '.footnote-reference a[href^="#"], .fn a[href^="#"]';
        const footnote_links = document.querySelectorAll(footnote_selectors);

        footnote_links.forEach(link => {
            const anchor = link as HTMLAnchorElement;
            // Verify it's actually a footnote link before observing
            if (is_footnote_link(anchor)) {
                this.observer!.observe(anchor);
            }
        });
    }

    /**
     * Add a footnote to the visible footnotes list and update the display
     */
    private add_visible_footnote(anchor: HTMLAnchorElement, footnote_id: string): void {
        // Don't add if already visible
        if (this.visible_footnotes.has(footnote_id)) {
            return;
        }

        const footnote_content = get_footnote_content(footnote_id);
        if (!footnote_content) {
            return;
        }

        const link_text = anchor.textContent || '';
        const footnote_number = extract_footnote_number(link_text);

        this.visible_footnotes.set(footnote_id, {
            number: footnote_number,
            content: footnote_content,
            id: footnote_id
        });

        this.update_display();
    }

    /**
     * Remove a footnote from the visible footnotes list and update the display
     */
    private remove_visible_footnote(footnote_id: string): void {
        this.visible_footnotes.delete(footnote_id);
        this.update_display();
    }

    /**
     * Update the bottom bar display with the current visible footnotes
     */
    private update_display(): void {
        if (!this.container) {
            return;
        }

        // If no footnotes are visible, hide the container
        if (this.visible_footnotes.size === 0) {
            this.container.classList.remove('show');
            document.body.classList.remove('footnote-bar-visible');
            return;
        }

        // Sort footnotes by their number for display
        const sorted_footnotes = Array.from(this.visible_footnotes.values()).sort((a, b) => {
            // Try to parse as numbers for proper numeric sorting
            const num_a = parseInt(a.number, 10);
            const num_b = parseInt(b.number, 10);

            if (!isNaN(num_a) && !isNaN(num_b)) {
                return num_a - num_b;
            }

            // Fallback to string comparison
            return a.number.localeCompare(b.number);
        });

        // Build the HTML for all visible footnotes
        let html = '';
        sorted_footnotes.forEach(footnote => {
            html += `
                <div class="footnote-item" data-footnote-id="${footnote.id}">
                    <span class="footnote-number">[${footnote.number}]</span>
                    <div class="footnote-content">${footnote.content}</div>
                </div>
            `;
        });

        // Update container content
        this.container.innerHTML = html;

        // Attach link handlers to any links within footnote content
        attach_link_handlers_to_element(this.container);

        // Show the container and add body class
        this.container.classList.add('show');
        document.body.classList.add('footnote-bar-visible');
    }

    /**
     * Clean up: disconnect observer and clear state
     */
    public destroy(): void {
        if (this.observer) {
            this.observer.disconnect();
            this.observer = null;
        }
        this.visible_footnotes.clear();
        document.body.classList.remove('footnote-bar-visible');
        this.initialized = false;
    }

    /**
     * Re-observe all footnotes (useful if page content changes)
     */
    public refresh(): void {
        this.destroy();
        this.init();
    }
}

// Export singleton instance
export const footnote_bottom_bar = new FootnoteBottomBar();
