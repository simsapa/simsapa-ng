pragma ComponentBehavior: Bound

import QtQuick

/**
 * ScrollableHelper - Reusable component for managing automatic scrolling to bottom
 * 
 * Usage:
 * ScrollableHelper {
 *     id: scroll_helper
 *     target_scroll_view: your_scroll_view
 * }
 * 
 * Then call: scroll_helper.scroll_to_bottom()
 */
QtObject {
    id: root
    
    // The ScrollView to control - can be set by the user or dynamically
    property var target_scroll_view: null
    
    // Internal properties for tracking content height
    property real last_content_height: 0
    
    // Timer for delayed scrolling to ensure layout completion
    property Timer scroll_timer: Timer {
        interval: 150
        repeat: false
        onTriggered: {
            root.perform_scroll_if_needed();
        }
    }
    
    // Main function to trigger scroll to bottom
    function scroll_to_bottom() {
        // Only scroll if needed - check if content exceeds view
        scroll_timer.restart();
    }
    
    // Internal function that performs the actual scroll check and operation
    function perform_scroll_if_needed() {
        if (!target_scroll_view || !target_scroll_view.contentItem) {
            console.warn("ScrollableHelper: target_scroll_view or contentItem is null");
            return;
        }
        
        var contentHeight = target_scroll_view.contentItem.contentHeight;
        var viewHeight = target_scroll_view.height;
        
        // Check if new content was added that exceeds the current view
        var contentGrew = contentHeight > root.last_content_height;
        root.last_content_height = contentHeight;
        
        // Only scroll if:
        // 1. Content actually grew (new content was added), AND
        // 2. The content now exceeds the view height
        if (contentGrew && contentHeight > viewHeight) {
            // Try different approaches to access the vertical scrollbar
            var scrolled = false;
            
            // Method 1: Direct ScrollBar.vertical access
            if (target_scroll_view.ScrollBar && target_scroll_view.ScrollBar.vertical) {
                target_scroll_view.ScrollBar.vertical.position = 1.0 - target_scroll_view.ScrollBar.vertical.size;
                scrolled = true;
            }
            // Method 2: Try accessing via flickable (ScrollView internally uses Flickable)
            else if (target_scroll_view.flickableItem && target_scroll_view.flickableItem.ScrollBar && target_scroll_view.flickableItem.ScrollBar.vertical) {
                target_scroll_view.flickableItem.ScrollBar.vertical.position = 1.0 - target_scroll_view.flickableItem.ScrollBar.vertical.size;
                scrolled = true;
            }
            // Method 3: Try using contentY directly on the ScrollView
            else if (target_scroll_view.hasOwnProperty("contentY")) {
                var maxContentY = Math.max(0, contentHeight - viewHeight);
                target_scroll_view.contentY = maxContentY;
                scrolled = true;
            }
            // Method 4: Try using contentY on contentItem
            else if (target_scroll_view.contentItem.hasOwnProperty("contentY")) {
                var maxContentY = Math.max(0, contentHeight - viewHeight);
                target_scroll_view.contentItem.contentY = maxContentY;
                scrolled = true;
            }
            
            if (!scrolled) {
                console.warn("ScrollableHelper: Unable to scroll - no valid scroll method found");
            }
        }
    }
    
    // Function to initialize the helper (should be called when the scroll view is ready)
    function initialize() {
        if (target_scroll_view && target_scroll_view.contentItem) {
            root.last_content_height = target_scroll_view.contentItem.contentHeight;
        } else {
            // Reset to 0 if no valid target
            root.last_content_height = 0;
        }
    }
}