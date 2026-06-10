# PRD: Portable Mode Windows Install

## 1. Introduction/Overview

Simsapa for Windows is currently distributed as a single Inno Setup installer
(`Simsapa-Setup-<version>.exe`) that performs a **standard install**: the
application is placed in `C:\Program Files\Simsapa` (`{autopf}\{#AppName}`) and
all user data (downloaded language databases, settings, annotations, logs) is
stored in `%LOCALAPPDATA%\profound-labs\simsapa-ng` via the `app_dirs` crate
convention. This requires administrator privileges and ties the data to one
machine/user profile.

This feature adds a **portable install** mode. In portable mode the user picks
any writable target folder (e.g. `Desktop\Simsapa`, or a folder on a USB drive),
and the application files are installed there. A launcher icon is created in the
**parent** of that folder (e.g. on the Desktop, or the USB drive root) pointing
at the installed executable. A `config.txt` next to the executable sets
`SIMSAPA_DIR` to a **sibling data folder** (e.g. `Desktop\SimsapaData`) using a
**relative path resolved against the executable's directory**, so the install
keeps working when a USB drive is moved to another machine and mounts under a
different drive letter. On first launch the app downloads its databases into
that `SIMSAPA_DIR`; on later launches it finds and loads them from there.

The goal: let a user run Simsapa entirely from a self-contained, relocatable
folder (including a USB stick) without administrator rights and without writing
to the system profile.

## 2. Goals

1. Offer the user an explicit choice between **Standard** and **Portable**
   install on a wizard page, with Standard as the default (unchanged behavior).
2. In portable mode, install all app files into a user-chosen target folder and
   create a launcher in that folder's parent that starts the installed exe.
3. In portable mode, write a `config.txt` next to the exe that sets
   `SIMSAPA_DIR` to a **sibling** data folder using a **relative** path
   (e.g. `../SimsapaData`).
4. Make the app discover `config.txt` from the **executable's own directory**
   (not only the current working directory), so it works regardless of how it
   is launched.
5. Resolve a **relative** `SIMSAPA_DIR` against the **executable's directory**,
   so portable installs survive USB drive-letter changes across machines.
6. On first portable launch, create the data folder and download the databases
   into it; on subsequent launches, load the existing databases from it.
7. Portable install must **not** require administrator privileges.
8. Standard install behavior, paths, and the uninstaller's existing user-data
   prompt remain unchanged.

## 3. User Stories

1. **As a user without admin rights**, I want to install Simsapa into a folder on
   my Desktop so that I can run it without an administrator password.
2. **As a traveling user**, I want to install Simsapa onto a USB stick and run it
   on different Windows computers, so that my suttas, dictionaries, and settings
   travel with me even when the USB drive gets a different drive letter.
3. **As a portable user**, I want a single recognizable icon next to my install
   folder (on the Desktop or USB root) so that I can launch the app without
   digging into subfolders.
4. **As a first-time portable user**, I want the app to download its databases
   into my chosen location on first launch, so that everything stays together in
   one place I control.
5. **As a returning portable user**, I want the app to find my previously
   downloaded databases automatically, so that I don't re-download them every
   time.
6. **As a standard user**, I want the normal Program Files install to keep
   working exactly as before, so that nothing I rely on changes.

## 4. Functional Requirements

### Installer — mode selection

1. The installer (`simsapa-installer.iss`) must present a custom wizard page,
   shown early (after Welcome, before the directory page), letting the user
   select one of three options, each with its **default install path shown** so a
   re-installing user can pick the same location:
   - **Standard install — all users**: `{commonpf}\Simsapa`
     (`C:\Program Files\Simsapa`); needs administrator rights; data in
     `%LOCALAPPDATA%\profound-labs\simsapa-ng`.
   - **Standard install — this user only**: `{localappdata}\Programs\Simsapa`;
     no admin required; data in `%LOCALAPPDATA%\profound-labs\simsapa-ng`.
   - **Portable install**: user chooses any target folder (default
     `{userdesktop}\Simsapa`); data stored in a sibling folder; no admin required.
2. When a **Standard** option is selected, the installer must behave as before
   (files, icons, VC++ redist handling, uninstaller user-data prompt); only the
   destination folder differs between the two Standard options.
3. The **installed app must not require administrator rights at runtime** in any
   mode. Only the *Standard — all users* install needs admin **at install time**
   (it writes to Program Files).

   **Implemented approach (lowest privileges + explicit choice).** The installer
   uses `PrivilegesRequired=lowest` with **no** `PrivilegesRequiredOverridesAllowed`,
   so there is **no UAC prompt and no "Select Setup Install Mode" dialog** at
   startup in any launch. The location is chosen explicitly on the mode page. The
   *all users* option requires the user to have launched the installer with "Run
   as administrator"; `NextButtonClick` **warns and blocks** that option when not
   elevated (`IsAdmin()`), steering the user to re-run as administrator or pick a
   no-admin option. The default option is *all users* when launched elevated,
   else *this user only*; Portable is never the default (so a Standard option is
   always preselected). Directory defaults use explicit folder constants
   (`{commonpf}`, `{localappdata}\Programs`, `{userdesktop}`), not `{autopf}`,
   which under `lowest` always resolves per-user.

   *Trade-off:* under `lowest` install mode, even an *all users* install registers
   its uninstaller per-user (HKCU), not HKLM — acceptable for this single-user
   desktop app. (An earlier iteration used
   `PrivilegesRequiredOverridesAllowed=dialog` so Portable could install without
   elevation, but that "Select Setup Install Mode" dialog appeared even when the
   exe was started with "Run as administrator", which was confusing, so it was
   removed.)

### Installer — portable target and data folders

4. In portable mode, the directory page must let the user pick the **install
   (app) folder** target (e.g. `Desktop\Simsapa`, or `E:\Simsapa`). The default
   suggestion should be a user-writable location (e.g. the Desktop), not
   Program Files.
5. The installer must derive the **data folder** as a **sibling** of the install
   folder, named by appending `Data` to the install folder name (e.g. install
   `Desktop\Simsapa` → data `Desktop\SimsapaData`). The installer must create
   this data folder at install time if it does not exist. If it **already
   exists**, the installer must **reuse it as-is** (no warning, no rename, no
   deletion) — this lets a user reinstall over an existing portable data folder
   and keep already-downloaded databases. (The databases themselves are
   downloaded by the app on first launch, not by the installer.)
6. The installer must write a `config.txt` file into the install (app) folder,
   next to `simsapadhammareader.exe`, containing a single setting:
   ```
   SIMSAPA_DIR=../SimsapaData
   ```
   The path must be **relative**, **unquoted**, and use **forward slashes**
   (`../SimsapaData`, not `..\SimsapaData`). Rationale: Rust's `Path` accepts
   `/` as a separator on Windows, while `dotenvy` treats backslash as an escape
   character (so `..\S...` is fragile/ambiguous). The exact relative string must
   match the sibling folder chosen in requirement 5.

### Installer — launcher icon in the parent folder

7. In portable mode, the installer must create a launcher in the **parent of the
   install folder** (e.g. on the Desktop, or the USB drive root) that starts the
   installed `simsapadhammareader.exe`.
8. The installer must let the user choose the launcher type on a wizard page,
   presenting both options with a concise note describing the implications of
   each:
   - **Relative-path launcher (`.cmd`)** that resolves the exe relative to its
     own location using `%~dp0`. Note: "Recommended for USB drives — keeps
     working if the drive letter changes on another computer." The `.cmd` must
     start the exe detached so no console window lingers, e.g.:
     ```bat
     @echo off
     start "" "%~dp0Simsapa\simsapadhammareader.exe"
     ```
     (`%~dp0` expands to the launcher's own drive+path; the install subfolder
     name must match requirement 4.)
   - **Standard Windows shortcut (`.lnk`)** pointing at the installed exe. Note:
     "Simplest; may stop working if a USB drive is given a different letter on
     another computer."
9. The launcher must use the app icon (`simsapa.ico`) where the chosen launcher
   type supports a custom icon (the `.lnk` option; the `.cmd` keeps the generic
   batch icon).
10. The launcher must launch the exe such that the app can locate `config.txt`
    (see app requirements 13–15). It must not depend on the current working
    directory being the install folder.

### Installer — hide Standard-only wizard elements in Portable mode

10b. In Portable mode the installer must **not** show wizard elements that have
    no effect in that mode, to avoid confusing the user:
    - The built-in **"Select Start Menu Folder"** page (`wpSelectProgramGroup`)
      is skipped (no group icons are created in portable mode).
    - The **"Create a desktop icon"** task is hidden (the portable launcher is
      created in the parent folder instead).
    Implemented via `ShouldSkipPage` (for `wpSelectProgramGroup`) and a
    `Check: ShouldRunStandard` on the `[Tasks]` `desktopicon` entry. Standard
    mode shows both exactly as before.

### Installer — no uninstaller in portable mode

10a. Portable mode must **not** register an uninstaller (no Add/Remove Programs
    entry, no `unins*.exe`, no registry uninstall keys). Removal is by the user
    deleting the install folder and the sibling data folder manually. The
    existing uninstaller (with its `%LOCALAPPDATA%` user-data prompt) applies to
    **Standard** install only. Use Inno's `Uninstallable` directive (and/or
    skipping registry writes) conditioned on the chosen mode.

### Application — config discovery next to the executable

11. `init_dotenv()` (backend/src/lib.rs) must additionally attempt to load
    `config.txt` from the **directory containing the running executable**
    (`std::env::current_exe()` → parent), in addition to the existing sources
    (`.env` in CWD, `config.txt` in CWD, `config.txt` in
    `get_create_simsapa_dir()`).
12. Loading must continue to **not override** environment variables already set
    in the process environment (preserve `dotenvy` non-override semantics and
    the existing precedence/ordering, with the exe-dir `config.txt` slotted in so
    that an explicit `SIMSAPA_DIR` env var still wins).
13. On platforms where `current_exe()` is unavailable or errors, the app must
    fall back gracefully to existing behavior (no panic; standard install path
    unaffected).

### Application — relative SIMSAPA_DIR resolution

14. When `SIMSAPA_DIR` is set to a **relative** path (as written by the portable
    installer), the app must resolve it against the **executable's directory**,
    not the process current working directory. The resolver must **normalize the
    `..` segments manually** (lexical normalization), **not** via
    `std::fs::canonicalize()` — on Windows `canonicalize()` returns
    `\\?\`-prefixed extended-length paths that can break downstream Qt/SQLite
    path handling.
15. When `SIMSAPA_DIR` is an **absolute** path, it must be used as-is (existing
    behavior unchanged).
16. The resolved `SIMSAPA_DIR` must be used by all existing data-path functions
    (`get_create_simsapa_dir`, `get_create_simsapa_app_assets_path`,
    `get_create_simsapa_appdata_db_path`, etc.) so the databases, `app-assets/`,
    and logs all live under the portable data folder.

### Application — first launch vs. later launches

17. On **first portable launch**, when the resolved `SIMSAPA_DIR` (data folder)
    exists but contains no databases, the app must follow the existing
    first-run/download flow to download databases into that folder.
18. On **subsequent launches**, the app must detect the existing databases in
    the portable data folder and load them without re-downloading (existing
    `appdata_db_exists()` / asset-presence checks operating on the resolved
    path).

### Build script

19. `build-windows.ps1` must continue to build a single installer that contains
    both modes (no separate output binary required). Any new installer
    inputs (e.g. the `config.txt` template, launcher template) must be packaged
    by the script/`.iss` as needed.

## 5. Non-Goals (Out of Scope)

1. **Changing standard-install behavior** — Program Files location,
   `%LOCALAPPDATA%` data path, and the existing uninstaller user-data prompt are
   unchanged.
2. **Portable mode on macOS, Linux, or Android** — this PRD covers Windows only.
   (Mobile already has its own `storage-path.txt` mechanism; that is untouched.)
3. **In-app switching between standard and portable** after install — mode is
   chosen at install time.
4. **Migrating an existing standard install's data** into a portable folder (or
   vice versa) — no automatic data migration is provided.
5. **A portable uninstaller** — portable mode registers no uninstaller; removal
   is the user deleting the folders (see requirement 10a). The standard
   uninstaller is unchanged.
6. **Auto-detecting and offering portable mode based on the chosen folder** — the
   choice is explicit on the wizard page.

## 6. Design Considerations

- **Wizard flow (portable):** Welcome → [startup "Select Setup Install Mode"
  dialog: choose "Install for me only" for no admin] → Install type
  (Standard/Portable; defaults to Portable when non-elevated) → [Portable]
  target app folder → launcher type (relative launcher vs `.lnk`, with the
  explanatory notes from requirement 8) → ready → install. The "Select Start
  Menu Folder" page and the "Create a desktop icon" task are **not** shown in
  Portable mode (requirement 10b).
- **Folder layout example (Desktop, portable):**
  ```
  Desktop\
    Simsapa.lnk  (or Simsapa.cmd)      <- launcher in parent folder
    Simsapa\                            <- install (app) folder
      simsapadhammareader.exe
      config.txt        (SIMSAPA_DIR=../SimsapaData)
      simsapa.ico
      <Qt libs, plugins, ...>
    SimsapaData\                        <- SIMSAPA_DIR (created at install, or reused if present)
      app-assets\   (downloaded on first launch: appdata.sqlite3, dictionaries, ...)
      logs\
  ```
- **USB example:** identical layout rooted at the drive (e.g. `E:\Simsapa.cmd`,
  `E:\Simsapa\...`, `E:\SimsapaData\...`); the relative `config.txt` and
  exe-relative resolution mean it still resolves when mounted as `F:` elsewhere.
- The launcher should not rely on "Start in"/CWD; config discovery is driven by
  the exe-directory `config.txt` (requirement 11).

## 7. Technical Considerations

- **Files to change:**
  - `simsapa-installer.iss` — custom mode page, conditional privilege handling,
    portable directory defaults, sibling data-folder creation, `config.txt`
    emission, launcher-type page, parent-folder launcher creation. Inno Setup
    `[Code]` (Pascal scripting) needed for the dynamic logic.
  - `backend/src/lib.rs` — `init_dotenv()` to also read `config.txt` from
    `current_exe()` parent (requirements 11–13); `get_create_simsapa_dir()` (or
    a helper) to resolve a relative `SIMSAPA_DIR` against the exe directory
    (requirements 14–16).
  - `build-windows.ps1` — package any new templates; ensure single-installer
    output (requirement 19).
- **`init_dotenv()` ordering gotcha:** the existing step 3 calls
  `get_create_simsapa_dir()`, which reads `SIMSAPA_DIR`. Ensure the exe-dir
  `config.txt` is loaded **before** any path resolution that depends on
  `SIMSAPA_DIR`, and that `dotenvy`'s non-override semantics keep an explicitly
  set env var authoritative.
- **Relative-path resolution must use `current_exe()`**, not
  `env::current_dir()`, because a `.lnk` or double-click can set an arbitrary
  CWD. Guard against `current_exe()` errors (requirement 13).
- **Android safety:** continue using `try_exists()` rather than `.exists()` for
  any new existence checks (per project guideline), though this feature is
  Windows-only.
- **No new FTS/DB schema changes**; databases are still produced by the existing
  download/bootstrap flow, only the destination directory changes.
- Keep `PROJECT_MAP.md` and the Windows packaging docs updated.

## 8. Success Metrics

1. A portable install to `Desktop\Simsapa` runs **without** an admin prompt,
   creates a working launcher on the Desktop, downloads databases into
   `Desktop\SimsapaData` on first launch, and loads them on subsequent launches.
2. The same portable folder, copied to a USB stick and run on a **second**
   Windows machine under a **different drive letter**, launches and loads its
   databases without re-download and without manual reconfiguration.
3. Standard install is byte-for-byte equivalent in behavior to the pre-feature
   installer (location, data path, uninstall prompt).
4. No regression: existing Rust tests pass and the Windows build/installer
   compiles cleanly.

## 9. Resolved Decisions

These were previously open questions; all are now decided.

1. **Launcher implementation:** Two choices offered during install — a plain
   **`.lnk`** (simple, custom icon, same-machine) and a **`.cmd`** relative
   launcher using `start "" "%~dp0Simsapa\simsapadhammareader.exe"` (USB-safe via
   `%~dp0`, survives drive-letter changes). Rejected alternatives and why:
   - *Plain `.lnk` as the USB option* — stores an absolute target + drive letter;
     the relative field is only a fallback Inno can't author. Not USB-portable.
   - *`.vbs`* — clean and no console flash, but VBScript is deprecated/disabled by
     default on current Windows; high AV risk. Rejected.
   - *Relative-target `.lnk`* — absolute target is tried first; authoring the
     relative field needs COM/helper work Inno doesn't provide. Rejected.
   - *Tiny launcher `.exe`* — nicest UX (custom icon, no flash, robust) but adds
     a binary to build and ships unsigned (SmartScreen). Recorded as a possible
     **future upgrade**, not in initial scope.
   The `.cmd`'s only downsides are a brief console flash and the generic batch
   icon, both acceptable for the USB use case.
2. **Portable uninstall:** No uninstaller is registered in portable mode; the
   user removes the install folder and the sibling data folder manually
   (requirement 10a). Standard uninstaller unchanged.
3. **Data folder collision:** If the sibling data folder already exists, the
   installer **reuses it as-is** (requirement 5), preserving already-downloaded
   databases.
4. **Existing `SIMSAPA_DIR` env var:** A pre-existing process/global
   `SIMSAPA_DIR` **wins** over `config.txt` — this is the default non-override
   behavior of `dotenvy` in `init_dotenv()` (requirement 12). Intentional.
5. **Relative path format:** `SIMSAPA_DIR=../SimsapaData` — relative, unquoted,
   **forward slashes** (requirement 6). Forward slashes are valid Rust path
   separators on Windows and avoid `dotenvy`'s backslash-escape pitfall. The
   resolver normalizes `..` lexically, not via `canonicalize()` (requirement 14).
