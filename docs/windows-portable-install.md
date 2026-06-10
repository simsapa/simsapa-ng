# Windows Portable Install

The Windows installer (`Simsapa-Setup-<version>.exe`, built by
`build-windows.ps1` from `simsapa-installer.iss`) offers two install modes,
chosen on a wizard page shown right after Welcome:

- **Standard install** (default, unchanged): installs to
  `C:\Program Files\Simsapa` (`{autopf}\Simsapa`), stores user data in
  `%LOCALAPPDATA%\profound-labs\simsapa-ng`, may require administrator rights,
  and registers a normal uninstaller (with the user-data prompt).
- **Portable install**: installs into any folder the user picks (Desktop, a USB
  drive, …), keeps all data in a sibling folder next to the app, requires no
  administrator rights, and registers **no** uninstaller.

This document covers the portable mode. Standard mode is unchanged from earlier
releases.

## Privileges and mode coherence

The installer uses a plain administrator install: `PrivilegesRequired=admin`
(Inno's default) with **no** `PrivilegesRequiredOverridesAllowed`. This means:

- Launched normally, Setup requests elevation (a **UAC prompt**) at startup.
- Launched via **"Run as administrator"**, Setup is already elevated and runs
  straight through — **no "Select Setup Install Mode" dialog** is shown.

Both Standard and Portable installs therefore run elevated. The elevation is an
**install-time** matter only: writing to `C:\Program Files` (Standard) needs
admin, and running elevated is harmless for Portable (it can still write to the
chosen folder and the sibling data folder). **The installed app never needs
administrator rights at runtime** in either mode — Standard stores data under
`%LOCALAPPDATA%`, Portable in its sibling data folder.

An earlier iteration used `PrivilegesRequiredOverridesAllowed=dialog` (the
"Select Setup Install Mode" dialog) so Portable could install without elevation,
but that dialog appeared even when the exe was started with "Run as
administrator", which was confusing. It was removed in favour of the plain admin
model above.

The Standard/Portable choice is made on the `ModePage` (after Welcome). The
option titles ("**Standard Install**", "**Portable Install**") are shown in bold;
the page is built from custom `TNewRadioButton` + `TNewStaticText` controls
(rather than `CreateInputOptionPage`) so the title is bold while the description
stays normal weight. **Standard is always the default** (`StandardRadio.Checked
:= True`, `IsPortable := False`) so users who do not expect a portable mode are
not surprised; there is no `IsAdminInstallMode()`-based flipping.

## Folder layout (Desktop example)

```
Desktop\
  Simsapa.lnk  (or Simsapa.cmd)      <- launcher in the parent folder
  Simsapa\                            <- install (app) folder = {app}
    simsapadhammareader.exe
    config.txt        (SIMSAPA_DIR=../SimsapaData)
    simsapa.ico
    <Qt libs, plugins, ...>
  SimsapaData\                        <- SIMSAPA_DIR data folder
    app-assets\   (downloaded on first launch: appdata.sqlite3, dictionaries, ...)
    logs\
```

On a USB stick the same layout is rooted at the drive, e.g. `E:\Simsapa.cmd`,
`E:\Simsapa\...`, `E:\SimsapaData\...`.

## How it works

### Installer (`simsapa-installer.iss`)

1. **Mode page** (`ModePage`, a `CreateInputOptionPage` after Welcome) sets the
   `IsPortable` script-global. `ShouldRunPortable` / `ShouldRunStandard` gate
   `[Icons]` and the `Uninstallable=ShouldRunStandard` directive (a `[Code]`
   Boolean function — `Uninstallable` takes a boolean expression, not a
   `{code:...}` string constant — so Portable registers no uninstaller /
   Add-Remove-Programs entry).
2. **Directory page** default is set per mode in `CurPageChanged`: Portable
   suggests `{userdesktop}\Simsapa`; Standard keeps `{autopf}\Simsapa`. The user
   may pick any folder.
3. **Data folder** is a sibling of the install folder named by appending `Data`
   to the install folder's name (`…\Simsapa` → `…\SimsapaData`), computed by
   `GetPortableDataDir`. In `CurStepChanged` (`ssPostInstall`) it is created if
   absent and **reused as-is if it already exists** (so a reinstall keeps
   already-downloaded databases).
4. **`config.txt`** is written into the install folder next to the exe with a
   single line:
   ```
   SIMSAPA_DIR=../SimsapaData
   ```
   The path is **relative**, **unquoted**, and uses **forward slashes**. Rust
   accepts `/` as a path separator on Windows; `dotenvy` treats `\` as an escape
   character, so backslashes are avoided. The suffix matches the data-folder name.
   In Portable mode the standard **"Select Start Menu Folder"** page
   (`wpSelectProgramGroup`) is skipped via `ShouldSkipPage`, and the
   **"Create a desktop icon"** task (`[Tasks]` `desktopicon`) is gated with
   `Check: ShouldRunStandard` — neither does anything for a portable install
   (the launcher lives in the parent folder), so they are hidden to avoid
   confusion.
5. **Launcher page** (`LauncherPage`, Portable only via `ShouldSkipPage`) lets
   the user choose, and the launcher is created in the **parent** of the install
   folder in `CurStepChanged`:
   - **`.lnk` shortcut** — created with `CreateShellLink`, targets the installed
     exe and uses `simsapa.ico`. Simplest; may break if a USB drive is given a
     different letter on another computer (a `.lnk` stores an absolute target).
   - **`.cmd` launcher** — recommended for USB. Contents:
     ```bat
     @echo off
     start "" "%~dp0Simsapa\simsapadhammareader.exe"
     ```
     `%~dp0` expands to the launcher's own drive+path, so the exe is resolved
     relative to the `.cmd`'s location and survives drive-letter changes. The
     install subfolder name is substituted for `Simsapa`. Downsides: a brief
     console flash and the generic batch icon.

   Neither launcher relies on a "Start in"/CWD value — the app finds `config.txt`
   via the exe's own directory (see below).

### Application (`backend/src/lib.rs`)

1. `init_dotenv()` loads `config.txt` from the **executable's own directory**
   (via `exe_dir()`), in addition to the existing CWD `.env`/`config.txt` and
   the `get_create_simsapa_dir()` `config.txt`. `dotenvy` does not override
   variables already set, so an explicit `SIMSAPA_DIR` env var still wins. The
   exe-dir `config.txt` is loaded **before** any path resolution that reads
   `SIMSAPA_DIR`. If `current_exe()` fails, `exe_dir()` returns `None` and the
   app falls back to existing behavior without panicking.
2. `resolve_simsapa_dir(raw, exe_dir)` resolves the value: a **relative** path is
   joined onto the exe directory and normalized with `normalize_lexically()`
   (lexical `..`/`.` collapse, **not** `std::fs::canonicalize()` — which returns
   `\\?\`-prefixed paths on Windows that break Qt/SQLite). An **absolute** path
   is used unchanged. A relative value with no exe dir falls back to the raw
   string.
3. The resolved `SIMSAPA_DIR` flows through `get_create_simsapa_dir()` and the
   downstream helpers (`get_create_simsapa_app_assets_path()`,
   `get_create_simsapa_appdata_db_path()`, logging), so databases, `app-assets/`,
   and logs all land under the portable data folder.

On **first** portable launch the data folder exists but has no databases, so the
existing first-run/download flow downloads them into it. On **subsequent**
launches the existing asset-presence checks find them and load without
re-downloading.

## USB drive-letter robustness

Because `config.txt` uses a relative `SIMSAPA_DIR` resolved against the exe
directory, and the `.cmd` launcher resolves the exe via `%~dp0`, a portable
install copied to a USB stick keeps working when the drive mounts under a
different letter on another machine — no reconfiguration and no re-download.

## Removal

Portable mode registers no uninstaller. To remove it, delete the install folder,
the sibling `…Data` folder, and the launcher in the parent folder. The Standard
uninstaller (with its `%LOCALAPPDATA%` user-data prompt) is unchanged and applies
to Standard installs only.

## Tests

`backend/tests/test_portable_path_resolution.rs` covers `resolve_simsapa_dir`
and `normalize_lexically`: relative resolution against a given exe dir, lexical
`..` collapse, absolute pass-through, forward-slash handling, and the no-exe-dir
fallback. Run with `cd backend && cargo test --test test_portable_path_resolution`.
