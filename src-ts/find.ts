import findAndReplace, { Recover } from 'dom-find-and-replace';

// Global state storage (alternative to localStorage)
declare global {
    interface Window {
        SimsapaFindState?: Map<string, any>;
    }
}

/**
 * Get global state storage
 */
function getGlobalState(): Map<string, any> {
    if (!window.SimsapaFindState) {
        window.SimsapaFindState = new Map();
    }
    return window.SimsapaFindState;
}

/**
 * Find Manager for implementing Ctrl+F functionality in Simsapa
 * Provides text search and highlighting within DOM content
 */
class FindManager {
    private searchTerm: string = '';
    private currentMatchIndex: number = 0;
    private totalMatches: number = 0;
    private isVisible: boolean = false;
    private recoverFunction: Recover | null = null;
    private debounceTimer: number | null = null;

    // DOM Elements
    private searchButton: HTMLElement | null = null;
    private findBar: HTMLElement | null = null;
    private findInput: HTMLInputElement | null = null;
    private findCounter: HTMLElement | null = null;
    private findError: HTMLElement | null = null;
    private prevButton: HTMLElement | null = null;
    private nextButton: HTMLElement | null = null;
    private accentFoldCheckbox: HTMLInputElement | null = null;
    private caseSensitiveCheckbox: HTMLInputElement | null = null;
    private contentArea: HTMLElement | null = null;

    constructor() {
        this.initializeElements();
    }

    /**
     * Initialize DOM element references
     */
    private initializeElements(): void {
        // Wait for DOM to be ready
        if (document.readyState === 'loading') {
            document.addEventListener('DOMContentLoaded', () => this.bindElements());
        } else {
            this.bindElements();
        }
    }

    /**
     * Bind DOM elements after page load
     */
    private bindElements(): void {
        this.searchButton = document.getElementById('findSearchButton');
        this.findBar = document.getElementById('findBar');
        this.findInput = document.getElementById('findInput') as HTMLInputElement;
        this.findCounter = document.getElementById('findCounter');
        this.findError = document.getElementById('findError');
        this.prevButton = document.getElementById('findPrevButton');
        this.nextButton = document.getElementById('findNextButton');
        this.accentFoldCheckbox = document.getElementById('findAccentFold') as HTMLInputElement;
        this.caseSensitiveCheckbox = document.getElementById('findCaseSensitive') as HTMLInputElement;
        this.contentArea = document.getElementById('ssp_content');

        // Setup event listeners
        this.setupEventListeners();
        
        // Load saved preferences
        this.loadPreferences();
    }

    /**
     * Setup event listeners for find bar elements
     */
    private setupEventListeners(): void {
        // Search button click
        if (this.searchButton) {
            this.searchButton.addEventListener('click', () => this.toggle());
        }

        // Input with debounced search
        if (this.findInput) {
            this.findInput.addEventListener('input', (e) => {
                const target = e.target as HTMLInputElement;
                this.debouncedSearch(target.value);
            });

            // Input keyboard shortcuts
            this.findInput.addEventListener('keydown', (e) => this.handleInputKeydown(e));
        }

        // Navigation buttons
        if (this.nextButton) {
            this.nextButton.addEventListener('click', () => this.nextMatch());
        }
        
        if (this.prevButton) {
            this.prevButton.addEventListener('click', () => this.previousMatch());
        }

        // Options checkboxes
        if (this.accentFoldCheckbox) {
            this.accentFoldCheckbox.addEventListener('change', () => {
                if (this.searchTerm) {
                    this.search(this.searchTerm);
                }
                this.savePreferences();
            });
        }

        if (this.caseSensitiveCheckbox) {
            this.caseSensitiveCheckbox.addEventListener('change', () => {
                if (this.searchTerm) {
                    this.search(this.searchTerm);
                }
                this.savePreferences();
            });
        }

        // Global keyboard shortcuts
        document.addEventListener('keydown', (e) => this.handleGlobalKeydown(e));
    }

    /**
     * Handle keyboard events when find input is focused
     */
    private handleInputKeydown(e: KeyboardEvent): void {
        if (e.key === 'Enter') {
            if (e.shiftKey) {
                e.preventDefault();
                this.previousMatch();
            } else {
                e.preventDefault();
                this.nextMatch();
            }
        } else if (e.key === 'Escape') {
            e.preventDefault();
            this.hide();
        }
    }

    /**
     * Handle global keyboard shortcuts
     */
    private handleGlobalKeydown(e: KeyboardEvent): void {
        // Ctrl+F to open find bar
        if (e.ctrlKey && e.key === 'f') {
            e.preventDefault();
            this.show();
            return;
        }

        // Escape to close find bar (only if find bar is visible)
        if (e.key === 'Escape' && this.isVisible) {
            e.preventDefault();
            this.hide();
            return;
        }
    }

    /**
     * Load saved preferences from global state
     */
    private loadPreferences(): void {
        const state = getGlobalState();
        
        // Load search term
        const savedTerm = state.get('simsapa-find-term');
        if (savedTerm && this.findInput) {
            this.findInput.value = savedTerm;
        }

        // Load accent folding preference (default: true)
        const accentFold = state.get('simsapa-find-accent-fold');
        if (this.accentFoldCheckbox) {
            this.accentFoldCheckbox.checked = accentFold !== false;
        }

        // Load case sensitive preference (default: false)
        const caseSensitive = state.get('simsapa-find-case-sensitive');
        if (this.caseSensitiveCheckbox) {
            this.caseSensitiveCheckbox.checked = caseSensitive === true;
        }
    }

    /**
     * Save preferences to global state
     */
    private savePreferences(): void {
        const state = getGlobalState();
        
        if (this.findInput) {
            state.set('simsapa-find-term', this.findInput.value);
        }

        if (this.accentFoldCheckbox) {
            state.set('simsapa-find-accent-fold', this.accentFoldCheckbox.checked);
        }

        if (this.caseSensitiveCheckbox) {
            state.set('simsapa-find-case-sensitive', this.caseSensitiveCheckbox.checked);
        }
    }

    /**
     * Debounced search with 400ms delay
     */
    private debouncedSearch(term: string): void {
        // Clear existing timer
        if (this.debounceTimer !== null) {
            clearTimeout(this.debounceTimer);
        }

        // Set new timer
        this.debounceTimer = setTimeout(() => {
            this.search(term);
            this.savePreferences();
        }, 400) as any; // Type assertion for browser/node compatibility
    }

    /**
     * Show the find bar
     */
    show(): void {
        if (!this.findBar || !this.searchButton) return;
        
        this.isVisible = true;
        this.searchButton.classList.add('active');
        this.findBar.classList.add('show');
        
        // Focus input after animation
        setTimeout(() => {
            if (this.findInput) {
                this.findInput.focus();
                this.findInput.select();
                
                // If there's a previous search term, start searching
                if (this.findInput.value && this.findInput.value.length >= 2) {
                    this.search(this.findInput.value);
                }
            }
        }, 100);
    }

    /**
     * Hide the find bar and clear highlights
     */
    hide(): void {
        if (!this.findBar || !this.searchButton) return;
        
        this.isVisible = false;
        this.searchButton.classList.remove('active');
        this.findBar.classList.remove('show');
        
        // Clear highlights
        this.clearHighlights();
        this.clearError();
    }

    /**
     * Clear search highlights
     */
    private clearHighlights(): void {
        if (this.recoverFunction) {
            this.recoverFunction();
            this.recoverFunction = null;
        }
        this.updateCounter(0, 0);
    }

    /**
     * Clear error message
     */
    private clearError(): void {
        if (this.findError) {
            this.findError.classList.remove('show');
            this.findError.textContent = '';
        }
    }

    /**
     * Update match counter display
     */
    private updateCounter(current: number, total: number): void {
        if (this.findCounter) {
            this.findCounter.textContent = `${current}/${total}`;
        }
        this.currentMatchIndex = current;
        this.totalMatches = total;
    }

    /**
     * Search for text within the content area
     */
    search(term: string): void {
        // Clear previous search
        this.clearHighlights();
        this.clearError();

        if (!term || term.length < 2 || !this.contentArea) {
            return;
        }

        this.searchTerm = term;
        
        try {
            // Use dom-find-and-replace to highlight matches
            this.recoverFunction = this.performSearch(term);
            this.updateMatchNavigation();
        } catch (error) {
            this.showError('Invalid search pattern');
        }
    }

    /**
     * Perform search using dom-find-and-replace library
     */
    private performSearch(term: string): Recover | null {
        if (!this.contentArea) return null;

        // Build regex flags
        let flags = 'g'; // Global search
        if (!this.isCaseSensitive()) {
            flags += 'i'; // Case insensitive
        }

        // Process term for accent folding if enabled
        const searchTerm = this.isAccentFoldingEnabled() ? this.createAccentFoldedPattern(term) : term;

        // Use dom-find-and-replace to highlight matches
        const recover = findAndReplace(this.contentArea, {
            find: searchTerm,
            flag: flags,
            replace: (offsetText: string, foundText: string) => {
                const span = document.createElement('span');
                span.className = 'ssp-find-highlight';
                span.textContent = offsetText;
                return span;
            }
        });

        // Count matches
        const highlights = this.contentArea.querySelectorAll('.ssp-find-highlight');
        this.updateCounter(highlights.length > 0 ? 1 : 0, highlights.length);

        // Handle no matches case
        if (highlights.length === 0) {
            this.showError('No matches found');
            return recover as Recover;
        }

        // Set first match as current
        (highlights[0] as HTMLElement).classList.add('current');
        this.scrollToElement(highlights[0] as HTMLElement);

        return recover as Recover;
    }

    /**
     * Create accent-folded regex pattern for Pali text
     */
    private createAccentFoldedPattern(term: string): string {
        if (!this.isAccentFoldingEnabled()) return term;

        // Pali/Sanskrit character mappings as specified in PRD
        const accented = ["ā","ī","ū","ṃ","ṁ","ṅ","ñ","ṭ","ḍ","ṇ","ḷ","ṛ","ṣ","ś"];
        const latin = ["a","i","u","m","m","n","n","t","d","n","l","r","s","s"];

        // Create a mapping from each character to its folded character class
        const foldMap = new Map<string, string>();
        
        for (let i = 0; i < accented.length; i++) {
            const accentedChar = accented[i];
            const latinChar = latin[i];
            
            // Both characters map to the same character class
            const charClass = `[${latinChar}${accentedChar}]`;
            foldMap.set(latinChar, charClass);
            foldMap.set(accentedChar, charClass);
        }

        // Replace each character in the term with its folded version
        let pattern = '';
        for (const char of term) {
            pattern += foldMap.get(char) || char;
        }

        return pattern;
    }

    /**
     * Check if accent folding is enabled
     */
    private isAccentFoldingEnabled(): boolean {
        return this.accentFoldCheckbox ? this.accentFoldCheckbox.checked : true;
    }

    /**
     * Check if case sensitive search is enabled
     */
    private isCaseSensitive(): boolean {
        return this.caseSensitiveCheckbox ? this.caseSensitiveCheckbox.checked : false;
    }

    /**
     * Update navigation button states
     */
    private updateMatchNavigation(): void {
        if (!this.prevButton || !this.nextButton) return;

        const hasMatches = this.totalMatches > 0;
        this.prevButton.style.opacity = hasMatches ? '1' : '0.5';
        this.nextButton.style.opacity = hasMatches ? '1' : '0.5';
    }

    /**
     * Scroll element into view
     */
    private scrollToElement(element: HTMLElement): void {
        element.scrollIntoView({ 
            behavior: 'smooth', 
            block: 'center',
            inline: 'nearest'
        });
    }

    /**
     * Show error message
     */
    private showError(message: string): void {
        if (this.findError) {
            this.findError.textContent = message;
            this.findError.classList.add('show');
        }
    }

    /**
     * Navigate to next match
     */
    nextMatch(): void {
        if (!this.contentArea || this.totalMatches === 0) return;

        const highlights = this.contentArea.querySelectorAll('.ssp-find-highlight');
        if (highlights.length === 0) return;

        // Remove current class from all highlights
        highlights.forEach(h => h.classList.remove('current'));

        // Calculate next index with wrap-around
        let nextIndex = this.currentMatchIndex % this.totalMatches;
        
        // Add current class to next match
        (highlights[nextIndex] as HTMLElement).classList.add('current');
        
        // Update counter (1-based display)
        this.updateCounter(nextIndex + 1, this.totalMatches);
        
        // Scroll to the new current match
        this.scrollToElement(highlights[nextIndex] as HTMLElement);
    }

    /**
     * Navigate to previous match
     */
    previousMatch(): void {
        if (!this.contentArea || this.totalMatches === 0) return;

        const highlights = this.contentArea.querySelectorAll('.ssp-find-highlight');
        if (highlights.length === 0) return;

        // Remove current class from all highlights
        highlights.forEach(h => h.classList.remove('current'));

        // Calculate previous index with wrap-around
        let prevIndex = this.currentMatchIndex - 2; // -2 because currentMatchIndex is 1-based
        if (prevIndex < 0) {
            prevIndex = this.totalMatches - 1; // Wrap to last match
        }
        
        // Add current class to previous match
        (highlights[prevIndex] as HTMLElement).classList.add('current');
        
        // Update counter (1-based display)
        this.updateCounter(prevIndex + 1, this.totalMatches);
        
        // Scroll to the new current match
        this.scrollToElement(highlights[prevIndex] as HTMLElement);
    }

    /**
     * Toggle find bar visibility
     */
    toggle(): void {
        if (this.isVisible) {
            this.hide();
        } else {
            this.show();
        }
    }

    /**
     * Check if find bar is visible
     */
    isShown(): boolean {
        return this.isVisible;
    }
}

// Export instance for use in simsapa.ts
const findManager = new FindManager();

export { findManager, FindManager };