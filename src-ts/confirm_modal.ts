/**
 * Custom confirmation modal for external links
 * Replaces the browser's default confirm() dialog
 */

interface ConfirmModalResult {
    confirmed: boolean;
}

class ConfirmModal {
    private overlay: HTMLElement | null = null;
    private message_element: HTMLElement | null = null;
    private confirm_button: HTMLElement | null = null;
    private cancel_button: HTMLElement | null = null;
    private copy_button: HTMLElement | null = null;
    private current_url: string = '';
    private resolve_callback: ((result: ConfirmModalResult) => void) | null = null;
    private initialized: boolean = false;
    private handlers_bound: boolean = false;

    constructor() {
        // Don't initialize in constructor - wait for DOM to be ready
    }

    private init(): void {
        // Get current DOM elements
        const overlay = document.getElementById('confirmModalOverlay');
        const message_element = document.getElementById('confirmModalMessage');
        const confirm_button = document.getElementById('confirmModalConfirm');
        const cancel_button = document.getElementById('confirmModalCancel');
        const copy_button = document.getElementById('confirmModalCopy');

        if (!overlay || !message_element || !confirm_button || !cancel_button || !copy_button) {
            console.error('[ConfirmModal] Required elements not found in DOM');
            this.initialized = false;
            return;
        }

        // Check if we need to reinitialize (DOM elements changed)
        const needs_reinit = (
            this.overlay !== overlay ||
            this.confirm_button !== confirm_button ||
            this.cancel_button !== cancel_button ||
            this.copy_button !== copy_button
        );

        if (needs_reinit || !this.initialized) {
            // Update references
            this.overlay = overlay;
            this.message_element = message_element;
            this.confirm_button = confirm_button;
            this.cancel_button = cancel_button;
            this.copy_button = copy_button;

            // Bind event handlers fresh (for testing scenarios where DOM is recreated)
            // In production, this only runs once since the DOM is stable
            this.confirm_button.addEventListener('click', () => this.handle_confirm());
            this.cancel_button.addEventListener('click', () => this.handle_cancel());
            this.copy_button.addEventListener('click', () => this.handle_copy());
            this.overlay.addEventListener('click', (e) => this.handle_overlay_click(e));

            // Handle escape key - only bind this once globally
            if (!this.handlers_bound) {
                document.addEventListener('keydown', (e) => this.handle_keydown(e));
                this.handlers_bound = true;
            }

            this.initialized = true;
        }
    }

    /**
     * Show the confirmation modal with a message and URL
     * Returns a Promise that resolves when user clicks confirm or cancel
     */
    public show(message: string, url: string): Promise<boolean> {
        // Always try to initialize (will skip if already initialized with valid DOM)
        this.init();

        if (!this.overlay || !this.message_element) {
            console.error('[ConfirmModal] Modal not initialized');
            return Promise.resolve(false);
        }

        this.current_url = url;
        this.message_element.textContent = message;
        this.overlay.classList.add('show');

        return new Promise((resolve) => {
            this.resolve_callback = (result: ConfirmModalResult) => {
                resolve(result.confirmed);
            };
        });
    }

    private handle_confirm(): void {
        this.close(true);
    }

    private handle_cancel(): void {
        this.close(false);
    }

    private async handle_copy(): Promise<void> {
        if (!this.current_url) {
            console.error('[ConfirmModal] No URL to copy');
            return;
        }

        try {
            // Use the backend API to copy to clipboard
            const API_URL = (globalThis as any).API_URL || 'http://localhost:4848';
            const response = await fetch(`${API_URL}/copy_to_clipboard`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({ text: this.current_url })
            });

            if (response.ok) {
                this.show_copy_feedback(true);
            } else {
                console.error('[ConfirmModal] Copy failed with status:', response.status);
                this.show_copy_feedback(false);
            }
        } catch (error) {
            console.error('[ConfirmModal] Failed to copy URL:', error);
            this.show_copy_feedback(false);
        }
    }

    private show_copy_feedback(success: boolean): void {
        if (!this.copy_button) {
            return;
        }

        const original_text = this.copy_button.textContent;
        this.copy_button.textContent = success ? 'Copied!' : 'Copy Failed';
        this.copy_button.classList.add(success ? 'success' : 'error');

        // Reset after 2 seconds
        setTimeout(() => {
            if (this.copy_button) {
                this.copy_button.textContent = original_text;
                this.copy_button.classList.remove('success', 'error');
            }
        }, 2000);
    }

    private handle_overlay_click(e: MouseEvent): void {
        // Close modal if clicking directly on overlay (not on modal content)
        if (e.target === this.overlay) {
            this.close(false);
        }
    }

    private handle_keydown(e: KeyboardEvent): void {
        // Close modal on Escape key
        if (e.key === 'Escape' && this.overlay?.classList.contains('show')) {
            this.close(false);
        }
    }

    private close(confirmed: boolean): void {
        if (!this.overlay) {
            return;
        }

        this.overlay.classList.remove('show');

        if (this.resolve_callback) {
            this.resolve_callback({ confirmed });
            this.resolve_callback = null;
        }
    }
}

// Export singleton instance
export const confirm_modal = new ConfirmModal();

/**
 * Show a confirmation modal for external links
 * Returns a Promise that resolves to true if user confirms, false otherwise
 */
export async function show_external_link_confirmation(url: string): Promise<boolean> {
    const message = `Open this link in your web browser?\n\n${url}`;
    return confirm_modal.show(message, url);
}
