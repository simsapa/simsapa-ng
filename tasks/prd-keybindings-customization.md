# PRD: Keyboard Shortcuts Customization

## 1. Introduction/Overview

This feature allows users to customize keyboard shortcuts for actions in the Sutta Search Window. Currently, keyboard shortcuts are hardcoded in `SuttaSearchWindow.qml`. This feature will add a new "Keybindings" tab in the App Settings window where users can view, modify, add, and remove keyboard shortcuts for each action.

**Problem:** Users cannot customize keyboard shortcuts to match their preferences or muscle memory from other applications.

**Solution:** A settings interface that displays all available actions with their current shortcuts, allowing users to capture new key combinations and manage multiple shortcuts per action.

## 2. Goals

1. Allow users to view all customizable keyboard shortcuts in one place
2. Enable users to assign custom key combinations to any action
3. Support multiple keyboard shortcuts per action (unlimited)
4. Provide reset functionality (individual and global)
5. Handle shortcut conflicts gracefully with user confirmation
6. Persist custom keybindings across app restarts

## 3. User Stories

1. **As a user**, I want to see all available keyboard shortcuts so that I can learn what actions are available.

2. **As a user**, I want to change a keyboard shortcut to a different key combination so that it matches my preferences.

3. **As a user**, I want to add multiple shortcuts for the same action so that I can use different key combinations depending on my hand position.

4. **As a user**, I want to remove a specific shortcut from an action so that I can free up that key combination for another action.

5. **As a user**, I want to reset an individual action's shortcuts to defaults so that I can undo my customizations for that specific action.

6. **As a user**, I want to reset all shortcuts to defaults so that I can start fresh if my customizations become confusing.

7. **As a user**, I want to be warned when I assign a shortcut that's already in use so that I don't accidentally remove it from another action.

## 4. Functional Requirements

### 4.1 Data Storage (Rust Backend)

1. **FR-1.1:** Create a new `AppKeybindings` struct in `app_settings.rs` to store custom keybindings.

2. **FR-1.2:** The `AppKeybindings` struct must store a mapping of action identifiers to a list of key sequences (e.g., `HashMap<String, Vec<String>>`).

3. **FR-1.3:** Add an `app_keybindings` field to the `AppSettings` struct.

4. **FR-1.4:** Implement default keybindings that match the current hardcoded shortcuts:

   | Action ID | Action Name | Default Shortcuts |
   |-----------|-------------|-------------------|
   | `settings` | Settings | `Ctrl+,` |
   | `close_window` | Close Window | `Alt+F4` |
   | `quit_app` | Quit Simsapa | `Ctrl+Q` |
   | `sutta_search` | Sutta Search | `F5` |
   | `focus_search` | Focus Search Input | `Ctrl+L` |
   | `next_search_area` | Next Search Area | `Ctrl+;` |
   | `prev_result` | Previous Result | `Ctrl+Up`, `Ctrl+K` |
   | `next_result` | Next Result | `Ctrl+Down`, `Ctrl+J` |
   | `find_in_page` | Find in Page | `Ctrl+F` |
   | `find_next` | Find Next in Page | `Ctrl+N` |
   | `find_prev` | Find Previous in Page | `Ctrl+P` |
   | `toggle_reading_mode` | Toggle Reading Mode | `Ctrl+Backspace` |
   | `close_tab` | Close Tab | `Ctrl+W` |
   | `toggle_tab_list` | Toggle Tab List | `Ctrl+Tab` |
   | `prev_tab` | Previous Tab | `Ctrl+[` |
   | `next_tab` | Next Tab | `Ctrl+]` |
   | `prev_sidebar_tab` | Previous Sidebar Tab | `Shift+[` |
   | `next_sidebar_tab` | Next Sidebar Tab | `Shift+]` |
   | `scroll_up` | Scroll Up | `K`, `Up` |
   | `scroll_down` | Scroll Down | `J`, `Down` |
   | `scroll_half_page_up` | Scroll Half Page Up | `Ctrl+U` |

5. **FR-1.5:** Implement bridge functions for keybindings management:
   - `get_keybindings_json()` - Returns all keybindings as JSON
   - `set_keybinding(action_id: String, shortcuts: Vec<String>)` - Sets shortcuts for an action
   - `reset_keybinding(action_id: String)` - Resets a single action to default
   - `reset_all_keybindings()` - Resets all keybindings to defaults
   - `get_default_keybindings_json()` - Returns default keybindings as JSON

### 4.2 Keybindings Settings Tab (QML)

6. **FR-2.1:** Add a new "Keybindings" tab to `AppSettingsWindow.qml` after the existing tabs (General, View, Find).

7. **FR-2.2:** The Keybindings tab must display a scrollable list of all customizable actions.

8. **FR-2.3:** Each action row must display:
   - The human-readable action name (e.g., "Focus Search Input")
   - One or more buttons showing the current shortcut(s) (e.g., `[Ctrl+L]`)
   - A `[+]` button to add a new shortcut (if there's room or always available)

9. **FR-2.4:** Shortcut buttons that differ from the default must use a different background color to visually indicate customization.

10. **FR-2.5:** Clicking a shortcut button must open the Keybinding Capture Dialog for that specific shortcut.

11. **FR-2.6:** Clicking the `[+]` button must open the Keybinding Capture Dialog to add a new shortcut.

12. **FR-2.7:** Provide a "Reset to Default" button for each action row.

13. **FR-2.8:** Provide a "Reset All to Defaults" button at the bottom of the keybindings list.

### 4.3 Keybinding Capture Dialog (QML)

14. **FR-3.1:** Create a new `KeybindingCaptureDialog.qml` using the `ApplicationWindow` pattern from `AboutDialog.qml`.

15. **FR-3.2:** The dialog must display:
   - A title indicating the action being configured (e.g., "Set Shortcut for: Focus Search Input")
   - Instructions for the user (e.g., "Press the desired key combination")
   - A display area showing the currently captured key combination
   - A "Clear" button to reset the capture area

16. **FR-3.3:** The dialog must capture key press events and display the resulting key sequence in Qt's native format (e.g., "Ctrl+Shift+A").

17. **FR-3.4:** The dialog must reject modifier-only shortcuts (e.g., just "Ctrl" or "Shift+Alt"). At least one non-modifier key must be included in the shortcut.

18. **FR-3.5:** The dialog must have the following buttons:
   - "Accept" - Saves the captured shortcut and closes the dialog
   - "Cancel" - Closes the dialog without saving
   - "Remove" - Removes this shortcut from the action (only shown when editing existing shortcut)

19. **FR-3.6:** The dialog must have proper spacing for mobile compatibility (bottom margin for button row).

### 4.4 Conflict Handling

20. **FR-4.1:** When the user accepts a shortcut that is already assigned to another action, show a confirmation dialog.

21. **FR-4.2:** The conflict dialog must display:
   - The shortcut being assigned
   - The action that currently uses this shortcut
   - A question: "Remove shortcut from [action name]?"
   - "Yes" and "No" buttons

22. **FR-4.3:** If the user confirms, remove the shortcut from the conflicting action and assign it to the new action.

23. **FR-4.4:** If the user declines, return to the capture dialog without changes.

### 4.5 SuttaSearchWindow Integration

24. **FR-5.1:** Modify `SuttaSearchWindow.qml` to load shortcuts from the stored keybindings instead of using hardcoded values.

25. **FR-5.2:** Shortcuts must be loaded when the window opens.

26. **FR-5.3:** When keybindings are changed in settings, the changes must take effect immediately (or at minimum after closing and reopening the Sutta Search Window).

### 4.6 Menu Accessibility

27. **FR-6.1:** Menu actions must always remain accessible via the menu, even when all keyboard shortcuts have been removed from an action.

## 5. Non-Goals (Out of Scope)

1. **NG-1:** Customizing shortcuts for windows other than SuttaSearchWindow (future enhancement)
2. **NG-2:** Import/export of keybinding configurations
3. **NG-3:** Search or filter functionality in the keybindings list
4. **NG-4:** Grouping shortcuts by category (File, Windows, Find, Tabs) - display as flat list
5. **NG-5:** Platform-specific key name display (using Qt's native format on all platforms)
6. **NG-6:** Keyboard shortcut chords (sequences like "Ctrl+K Ctrl+C")

## 6. Design Considerations

### 6.1 Keybindings Settings Tab Layout

```
┌─────────────────────────────────────────────────────────────┐
│ [General] [View] [Find] [Keybindings]                       │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌───────────────────────────────────────────────────────┐  │
│  │ Settings              [Ctrl+,]           [+] [Reset]  │  │
│  ├───────────────────────────────────────────────────────┤  │
│  │ Close Window          [Alt+F4]           [+] [Reset]  │  │
│  ├───────────────────────────────────────────────────────┤  │
│  │ Focus Search Input    [Ctrl+L]           [+] [Reset]  │  │
│  ├───────────────────────────────────────────────────────┤  │
│  │ Previous Result       [Ctrl+Up] [Ctrl+K] [+] [Reset]  │  │
│  ├───────────────────────────────────────────────────────┤  │
│  │ Scroll Up             [K] [Up]           [+] [Reset]  │  │
│  └───────────────────────────────────────────────────────┘  │
│                                                             │
│                            [Reset All to Defaults]          │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 6.2 Keybinding Capture Dialog Layout

```
┌─────────────────────────────────────────────────┐
│ Set Shortcut for: Focus Search Input            │
├─────────────────────────────────────────────────┤
│                                                 │
│  Press the desired key combination:             │
│                                                 │
│  ┌─────────────────────────────────────────┐    │
│  │                                         │    │
│  │            Ctrl+Shift+L                 │    │
│  │                                         │    │
│  └─────────────────────────────────────────┘    │
│                                                 │
│              [Clear]                            │
│                                                 │
├─────────────────────────────────────────────────┤
│         [Remove]    [Cancel]    [Accept]        │
└─────────────────────────────────────────────────┘
```

### 6.3 Conflict Dialog Layout

```
┌─────────────────────────────────────────────────┐
│ Shortcut Conflict                               │
├─────────────────────────────────────────────────┤
│                                                 │
│  The shortcut "Ctrl+L" is already assigned to:  │
│                                                 │
│  "Focus Search Input"                           │
│                                                 │
│  Remove shortcut from this action?              │
│                                                 │
├─────────────────────────────────────────────────┤
│                    [No]    [Yes]                │
└─────────────────────────────────────────────────┘
```

### 6.4 Visual Style

- Shortcut buttons should use a distinct style (border or background) to indicate they are clickable
- **Customized shortcuts** (differing from defaults) must use a different background color than default shortcuts to provide visual feedback
- The capture area in the dialog should have a clear visual focus state
- Consider using a monospace font for displaying key combinations

## 7. Technical Considerations

### 7.1 Rust Backend

- The `AppKeybindings` struct should implement `Default` trait with hardcoded defaults
- Use `serde` for JSON serialization of keybindings
- The keybindings should be stored in the same JSON settings file as other app settings
- Follow the existing pattern of getter/setter bridge functions

### 7.2 QML Frontend

- Follow the existing `AboutDialog.qml` pattern for the capture dialog
- Register `KeybindingCaptureDialog.qml` in `bridges/build.rs` under `qml_files`
- Create a corresponding type definition for qmllint
- Use Qt's `Keys` attached property for capturing key events
- Use `QKeySequence` format strings for storing shortcuts (e.g., "Ctrl+Shift+A")

### 7.3 Key Capture Implementation

```qml
// Example key capture approach
Item {
    focus: true
    Keys.onPressed: (event) => {
        // Build key sequence string from event
        var modifiers = [];
        if (event.modifiers & Qt.ControlModifier) modifiers.push("Ctrl");
        if (event.modifiers & Qt.ShiftModifier) modifiers.push("Shift");
        if (event.modifiers & Qt.AltModifier) modifiers.push("Alt");
        if (event.modifiers & Qt.MetaModifier) modifiers.push("Meta");

        // Get the non-modifier key name
        var keyName = get_key_name(event.key);

        // Only accept if there's a non-modifier key (reject modifier-only shortcuts)
        if (keyName && !is_modifier_key(event.key)) {
            modifiers.push(keyName);
            captured_shortcut = modifiers.join("+");
            is_valid_shortcut = true;
        } else {
            // Show modifiers being held but mark as incomplete
            captured_shortcut = modifiers.length > 0 ? modifiers.join("+") + "..." : "";
            is_valid_shortcut = false;
        }
        event.accepted = true;
    }
}

function is_modifier_key(key) {
    return key === Qt.Key_Control || key === Qt.Key_Shift ||
           key === Qt.Key_Alt || key === Qt.Key_Meta;
}
```

### 7.4 Dynamic Shortcut Loading

The `SuttaSearchWindow.qml` shortcuts need to be made dynamic. Consider:
- Using a JavaScript object to map action IDs to their current shortcuts
- Loading this mapping from the bridge on window creation
- Optionally supporting live updates via signals

## 8. Success Metrics

1. **Usability:** Users can successfully customize at least one shortcut without errors
2. **Persistence:** Custom shortcuts persist across application restarts
3. **Conflict Resolution:** Users are properly warned about conflicts and can resolve them
4. **Reset Functionality:** Both individual and global reset work correctly
5. **No Regressions:** Existing shortcut functionality continues to work for users who don't customize

## 9. Resolved Design Decisions

1. **Q1: Visual indicator for customized shortcuts?**
   - **Decision:** Yes. Shortcut buttons that differ from defaults use a different background color.

2. **Q2: Group shortcuts by category?**
   - **Decision:** No. Display as a flat list for simplicity.

3. **Q3: Menu accessibility when shortcuts removed?**
   - **Decision:** Yes. Menu actions always remain accessible regardless of keyboard shortcut assignments.

4. **Q4: Allow modifier-only shortcuts?**
   - **Decision:** No. At least one non-modifier key is required to avoid conflicts with typing.
