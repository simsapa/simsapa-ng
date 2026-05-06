# Global Hotkeys for Dictionary Lookup

Simsapa can register an OS-level global hotkey so that you can run a Pāli
dictionary lookup from **any** application — a browser, a PDF viewer, an
editor — without alt-tabbing into Simsapa first.

The default hotkey is **`Ctrl+C+C`**. When the hotkey fires, Simsapa reads the
system clipboard, raises its main window, switches the search area to
*Dictionary*, and runs the query.

## Settings

Open **Settings → Keybindings → Global Hotkeys** to configure the feature.

- **Enable global hotkeys** — toggles OS-level registration. The row remains
  editable when unchecked, so you can change the binding before re-enabling.
- **Dictionary lookup** — the action bound by default to `Ctrl+C+C`. Click
  *Edit* to capture a new sequence. The capture dialog supports both single
  chords (e.g. `Ctrl+Alt+L`) and double-tap chords (e.g. `Ctrl+C+C`).

Changes apply immediately — no app restart is required.

## Platform support

| Platform   | Status                                                      |
|------------|-------------------------------------------------------------|
| Linux X11  | Supported (uses the `XRecord` extension)                    |
| Windows    | Pending (tracked in PRD §4.4 / tasks-prd-global-hotkeys 5)  |
| macOS      | Pending (tracked in PRD §4.4 / tasks-prd-global-hotkeys 6)  |
| Linux Wayland | Not supported — see *Wayland workaround* below           |
| Android / iOS | Not supported                                            |

### macOS Accessibility permission

On macOS, Simsapa needs the **Accessibility** permission to synthesize the
`⌘C` keystroke (and to read selected text via the Accessibility API). On the
first hotkey activation Simsapa will prompt with a one-click button that opens
*System Settings → Privacy & Security → Accessibility*. Tick the Simsapa entry
and try the hotkey again.

## Wayland workaround

OS-level global hotkey registration is not feasible from a sandboxed
application on Wayland. Instead, bind a **desktop-environment shortcut**
(GNOME *Settings → Keyboard → Custom Shortcuts*, or KDE's equivalent) that
calls Simsapa's localhost API:

```
GET  http://127.0.0.1:<port>/lookup_window_query?q=<text>
POST http://127.0.0.1:<port>/lookup_window_query    # body: {"q": "<text>"}
```

A copy-pasteable `curl` example using the current clipboard:

```sh
curl -G --data-urlencode "q=$(wl-paste)" http://127.0.0.1:<port>/lookup_window_query
```

The actual port is shown in the **Global Hotkeys** panel on Wayland.

## Troubleshooting

- **"Global hotkey registration failed"** on Linux — make sure your X server
  has the `RECORD` extension enabled, and that no other application is
  grabbing the same key combination.
- **Nothing happens on `Ctrl+C+C`** — verify the source application actually
  copies the selection on `Ctrl+C` (the second `C` is the user's own copy
  keystroke; Simsapa reads the clipboard ~80 ms later).
- **The query is wrong / truncated** — Simsapa trims whitespace and caps the
  query at 200 characters. Highlight a tighter selection.
