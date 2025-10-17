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

function lookup_selection() {
    const selected_text = window.getSelection().toString().trim();

    if (!selected_text) {
        console.log('No text selected');
        return;
    }

    fetch(`${API_URL}/lookup_window_query/${encodeURIComponent(selected_text)}`);
}

function summary_selection() {
    const text = selection_text();
    if (text !== "") {
        fetch(`${API_URL}/summary_query/${WINDOW_ID}/${encodeURIComponent(text)}`);
    }
}

class HamburgerMenu {
    constructor() {
        this.menuButton = document.getElementById('menuButton');
        this.menuDropdown = document.getElementById('menuDropdown');
        this.groupHeaders = document.querySelectorAll('.group-header');
        this.menuItems = document.querySelectorAll('.menu-item');
        this.isOpen = false;

        this.init();
    }

    init() {
        // Menu button click
        this.menuButton.addEventListener('click', () => this.toggleMenu());

        // Group header clicks
        this.groupHeaders.forEach(header => {
            header.addEventListener('click', (e) => this.toggleGroup(e));
            // Prevent text selection clearing
            header.addEventListener('mousedown', (e) => e.preventDefault());
        });

        // Menu item clicks
        this.menuItems.forEach(item => {
            item.addEventListener('click', (e) => this.handleMenuItemClick(e));
            // Prevent text selection clearing
            item.addEventListener('mousedown', (e) => e.preventDefault());
        });

        // Close menu on escape key
        document.addEventListener('keydown', (e) => {
            if (e.key === 'Escape' && this.isOpen) {
                this.closeMenu();
            }
        });
    }

    toggleMenu() {
        if (this.isOpen) {
            this.closeMenu();
        } else {
            this.openMenu();
        }
    }

    openMenu() {
        this.isOpen = true;
        this.menuButton.classList.add('active');
        this.menuDropdown.classList.add('show');
    }

    closeMenu() {
        this.isOpen = false;
        this.menuButton.classList.remove('active');
        this.menuDropdown.classList.remove('show');
    }

    toggleGroup(e) {
        e.preventDefault();
        e.stopPropagation();

        const header = e.currentTarget;
        const groupName = header.dataset.group;
        const groupItems = document.getElementById(`${groupName}-items`);
        const isExpanded = header.classList.contains('expanded');

        if (isExpanded) {
            // Collapse group
            header.classList.remove('expanded');
            groupItems.classList.remove('show');
        } else {
            // Expand group
            header.classList.add('expanded');
            groupItems.classList.add('show');
        }
    }

    async handleMenuItemClick(e) {
        e.preventDefault();
        e.stopPropagation();

        const action = e.currentTarget.dataset.action;
        let query_text = "";
        if (action === "summarize-sutta") {
            query_text = this.getAllContentText();
        } else {
            query_text = this.getSelectedText();
        }

        try {
            const response = await fetch(`${API_URL}/sutta_menu_action/`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({
                    window_id: WINDOW_ID,
                    action: action,
                    text: query_text,
                })
            });

            if (!response.ok) {
                throw new Error(`HTTP error! status: ${response.status} ${response.statusText}`);
            }

            log_info(`Action '${action}' completed`);
        } catch (error) {
            log_error(`Action '${action}' error:` + error);
        }

        this.closeMenu();
    }

    getSelectedText() {
        if (window.getSelection) {
            return window.getSelection().toString();
        } else if (document.selection && document.selection.type !== "Control") {
            return document.selection.createRange().text;
        }
        return '';
    }

    getAllContentText() {
        const content_div = document.getElementById('ssp_content');
        if (!content_div) {
            console.error('Element with id "ssp_content" not found');
            return null;
        }
        // .textContent gets all text content, including text from hidden elements
        // .innerText respects styling and won't include text from hidden elements
        return content_div.innerText || '';
    }
}

function toggle_variant (event) {
    let el = event.target;
    el.parentNode.querySelectorAll(".variant").forEach((i) => {
        i.classList.toggle("hide");
    })
}

function toggle_comment (event) {
    let el = event.target;
    el.parentNode.querySelectorAll(".comment").forEach((i) => {
        i.classList.toggle("hide");
    })
}

class TextResizeController {
    constructor() {
        this.increaseButton = document.getElementById('textSizeIncreaseButton');
        this.decreaseButton = document.getElementById('textSizeDecreaseButton');
        this.contentDiv = document.getElementById('ssp_content');
        this.currentScale = this.getInitialScale();
        this.minScale = 0.3;
        this.maxScale = 2.0;
        this.scaleStep = 0.1;
        this.baseMaxWidth = 75;

        this.init();
    }

    getInitialScale() {
        const mediaQuery = window.matchMedia('(max-width: 768px)');
        return mediaQuery.matches ? 0.8 : 1.0;
    }

    init() {
        if (!this.increaseButton || !this.decreaseButton || !this.contentDiv) {
            return;
        }

        this.increaseButton.addEventListener('click', () => this.increaseTextSize());
        this.decreaseButton.addEventListener('click', () => this.decreaseTextSize());
        this.applyScale();
    }

    increaseTextSize() {
        if (this.currentScale < this.maxScale) {
            this.currentScale += this.scaleStep;
            this.applyScale();
        }
    }

    decreaseTextSize() {
        if (this.currentScale > this.minScale) {
            this.currentScale -= this.scaleStep;
            this.applyScale();
        }
    }

    applyScale() {
        this.contentDiv.style.fontSize = `${this.currentScale}em`;
        const adjustedMaxWidth = this.baseMaxWidth * this.currentScale;
        document.body.style.maxWidth = `${adjustedMaxWidth}ex`;
    }
}

document.addEventListener("DOMContentLoaded", function(_event) {
    new HamburgerMenu();
    new TextResizeController();
    if (IS_MOBILE) {
        // On mobile in a WebView, there is no double click event, so listen to
        // selection change (from a long press action).
        let previous_selection_text = "";
        let word_summary_was_closed = false;

        window.word_summary_closed = function() {
            word_summary_was_closed = true;
        };

        document.addEventListener("selectionchange", function (_event) {
            const selection = document.getSelection();
            if (selection && selection.rangeCount > 0) {
                const range = selection.getRangeAt(0);
                const container = range.commonAncestorContainer;
                const element = container.nodeType === Node.TEXT_NODE ? container.parentElement : container;

                // Skip if selection is within find bar
                if (element && element.closest('#findContainer')) {
                    return;
                }
            }

            const current_text = selection_text();

            if (current_text === "") {
                previous_selection_text = "";
                word_summary_was_closed = false;
                return;
            }

            if (word_summary_was_closed) {
                const is_boundary_drag = current_text.startsWith(previous_selection_text) ||
                                         current_text.endsWith(previous_selection_text) ||
                                         previous_selection_text.startsWith(current_text) ||
                                         previous_selection_text.endsWith(current_text);

                if (is_boundary_drag) {
                    previous_selection_text = current_text;
                    return;
                }

                word_summary_was_closed = false;
            }

            previous_selection_text = current_text;
            summary_selection();
        });

    } else {
        // On desktop, double click works to select a word and trigger a lookup.
        // Double click always triggers a lookup.
        document.addEventListener("dblclick", function (event) {
            // Skip if double click is within find bar
            if (event.target.closest('#findContainer')) {
                return;
            }
            summary_selection();
        });
    }

    document.querySelectorAll(".variant-wrap .mark").forEach((i) => {
        i.addEventListener("click", toggle_variant);
    });
    document.querySelectorAll(".comment-wrap .mark").forEach((i) => {
        i.addEventListener("click", toggle_comment);
    });
});
