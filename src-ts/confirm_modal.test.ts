/**
 * Test cases for the Confirm Modal functionality
 * Tests the custom HTML+CSS modal that replaces JavaScript's confirm()
 */

import { show_external_link_confirmation } from './confirm_modal';

// Mock DOM environment for testing
function createMockDOM(): void {
    document.body.innerHTML = `
        <div class="confirm-modal-overlay" id="confirmModalOverlay">
            <div class="confirm-modal">
                <div class="confirm-modal-header">
                    <h3 class="confirm-modal-title">Confirm Action</h3>
                </div>
                <div class="confirm-modal-body">
                    <p class="confirm-modal-message" id="confirmModalMessage"></p>
                </div>
                <div class="confirm-modal-footer">
                    <button class="confirm-modal-button confirm-modal-cancel" id="confirmModalCancel">Cancel</button>
                    <div class="confirm-modal-button-group">
                        <button class="confirm-modal-button confirm-modal-copy" id="confirmModalCopy">Copy URL</button>
                        <button class="confirm-modal-button confirm-modal-confirm" id="confirmModalConfirm">Open Link</button>
                    </div>
                </div>
            </div>
        </div>
    `;
}

describe('Confirm Modal functionality', () => {
    beforeEach(() => {
        // Setup DOM
        createMockDOM();
    });

    afterEach(() => {
        // Clear any remaining modal state
        const overlay = document.getElementById('confirmModalOverlay');
        overlay?.classList.remove('show');
    });

    describe('Modal display', () => {
        test('should show modal when show() is called', async () => {
            const testUrl = 'https://example.com';
            
            // Show modal (don't wait for result)
            const promise = show_external_link_confirmation(testUrl);
            
            // Wait a tick for the modal to show
            await new Promise(resolve => setTimeout(resolve, 10));
            
            const overlay = document.getElementById('confirmModalOverlay');
            expect(overlay?.classList.contains('show')).toBe(true);
            
            // Clean up by clicking cancel
            const cancelButton = document.getElementById('confirmModalCancel');
            (cancelButton as HTMLElement).click();
            
            await promise;
        });

        test('should display the correct message', async () => {
            const testUrl = 'https://example.com/test';
            
            const promise = show_external_link_confirmation(testUrl);
            
            await new Promise(resolve => setTimeout(resolve, 10));
            
            const messageElement = document.getElementById('confirmModalMessage');
            expect(messageElement?.textContent).toContain(testUrl);
            expect(messageElement?.textContent).toContain('Open this link in your web browser?');
            
            // Clean up
            const cancelButton = document.getElementById('confirmModalCancel');
            (cancelButton as HTMLElement).click();
            
            await promise;
        });
    });

    describe('User interaction', () => {
        test('should copy URL when copy button is clicked', async () => {
            const testUrl = 'https://example.com/test';
            
            // Mock fetch API for copy_to_clipboard endpoint
            global.fetch = jest.fn().mockResolvedValue({
                ok: true,
                status: 200,
            } as Response);
            
            const promise = show_external_link_confirmation(testUrl);
            
            await new Promise(resolve => setTimeout(resolve, 10));
            
            const copyButton = document.getElementById('confirmModalCopy');
            (copyButton as HTMLElement).click();
            
            // Wait for async copy operation
            await new Promise(resolve => setTimeout(resolve, 50));
            
            expect(global.fetch).toHaveBeenCalledWith(
                expect.stringContaining('/copy_to_clipboard'),
                expect.objectContaining({
                    method: 'POST',
                    body: JSON.stringify({ text: testUrl })
                })
            );
            expect(copyButton?.textContent).toBe('Copied!');
            
            // Clean up by clicking cancel
            const cancelButton = document.getElementById('confirmModalCancel');
            (cancelButton as HTMLElement).click();
            
            await promise;
        });

        test('should resolve to true when confirm button is clicked', async () => {
            const testUrl = 'https://example.com';
            
            const promise = show_external_link_confirmation(testUrl);
            
            await new Promise(resolve => setTimeout(resolve, 10));
            
            const confirmButton = document.getElementById('confirmModalConfirm');
            (confirmButton as HTMLElement).click();
            
            const result = await promise;
            expect(result).toBe(true);
        });

        test('should resolve to false when cancel button is clicked', async () => {
            const testUrl = 'https://example.com';
            
            const promise = show_external_link_confirmation(testUrl);
            
            await new Promise(resolve => setTimeout(resolve, 10));
            
            const cancelButton = document.getElementById('confirmModalCancel');
            (cancelButton as HTMLElement).click();
            
            const result = await promise;
            expect(result).toBe(false);
        });

        test('should resolve to false when clicking overlay', async () => {
            const testUrl = 'https://example.com';
            
            const promise = show_external_link_confirmation(testUrl);
            
            await new Promise(resolve => setTimeout(resolve, 10));
            
            const overlay = document.getElementById('confirmModalOverlay');
            const event = new MouseEvent('click', {
                bubbles: true,
                cancelable: true,
            });
            
            // Simulate clicking directly on overlay (not modal content)
            Object.defineProperty(event, 'target', {
                value: overlay,
                writable: false
            });
            
            overlay?.dispatchEvent(event);
            
            const result = await promise;
            expect(result).toBe(false);
        });

        test('should resolve to false when pressing Escape key', async () => {
            const testUrl = 'https://example.com';
            
            const promise = show_external_link_confirmation(testUrl);
            
            await new Promise(resolve => setTimeout(resolve, 10));
            
            const event = new KeyboardEvent('keydown', {
                key: 'Escape',
                bubbles: true,
                cancelable: true,
            });
            
            document.dispatchEvent(event);
            
            const result = await promise;
            expect(result).toBe(false);
        });
    });

    describe('Modal state management', () => {
        test('should hide modal after confirm', async () => {
            const testUrl = 'https://example.com';
            
            const promise = show_external_link_confirmation(testUrl);
            
            await new Promise(resolve => setTimeout(resolve, 10));
            
            const confirmButton = document.getElementById('confirmModalConfirm');
            (confirmButton as HTMLElement).click();
            
            await promise;
            
            await new Promise(resolve => setTimeout(resolve, 10));
            
            const overlay = document.getElementById('confirmModalOverlay');
            expect(overlay?.classList.contains('show')).toBe(false);
        });

        test('should hide modal after cancel', async () => {
            const testUrl = 'https://example.com';
            
            const promise = show_external_link_confirmation(testUrl);
            
            await new Promise(resolve => setTimeout(resolve, 10));
            
            const cancelButton = document.getElementById('confirmModalCancel');
            (cancelButton as HTMLElement).click();
            
            await promise;
            
            await new Promise(resolve => setTimeout(resolve, 10));
            
            const overlay = document.getElementById('confirmModalOverlay');
            expect(overlay?.classList.contains('show')).toBe(false);
        });

        test('should handle multiple sequential modals', async () => {
            // First modal
            const promise1 = show_external_link_confirmation('https://example.com/1');
            await new Promise(resolve => setTimeout(resolve, 10));
            
            const confirmButton = document.getElementById('confirmModalConfirm');
            (confirmButton as HTMLElement).click();
            
            const result1 = await promise1;
            expect(result1).toBe(true);
            
            await new Promise(resolve => setTimeout(resolve, 10));
            
            // Second modal
            const promise2 = show_external_link_confirmation('https://example.com/2');
            await new Promise(resolve => setTimeout(resolve, 10));
            
            const cancelButton = document.getElementById('confirmModalCancel');
            (cancelButton as HTMLElement).click();
            
            const result2 = await promise2;
            expect(result2).toBe(false);
        });
    });
});
