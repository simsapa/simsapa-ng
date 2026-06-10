# Tasks: Portable Mode Windows Install

Generated from [prd-portable-windows-install.md](./prd-portable-windows-install.md).

## Relevant Files

- `backend/src/lib.rs` - Core data-path logic. `init_dotenv()` (~line 65) must
  also load `config.txt` from the executable's directory; `get_create_simsapa_dir()`
  (~line 533) must resolve a relative `SIMSAPA_DIR` against the exe directory with
  lexical `..` normalization. Existing helpers
  (`get_create_simsapa_app_assets_path`, `get_create_simsapa_appdata_db_path`,
  `get_create_simsapa_internal_app_root`) consume the resolved path.
- `backend/tests/test_portable_path_resolution.rs` - New unit/integration tests
  for exe-relative `SIMSAPA_DIR` resolution and lexical normalization (no GUI,
  no real DB needed).
- `simsapa-installer.iss` - Inno Setup script. Add mode-selection page,
  conditional privileges, portable directory defaults, sibling data-folder
  create/reuse, `config.txt` emission, launcher-type page, parent-folder
  launcher creation, and conditional `Uninstallable`.
- `build-windows.ps1` - Windows build/packaging script. Updated summary output
  to mention the Portable mode; still produces a single installer. No new
  templates needed (config.txt / launcher emitted inline from `[Code]`).
- `PROJECT_MAP.md` - Updated Directory Paths section with portable-mode
  resolution (`exe_dir()`, `resolve_simsapa_dir()`, `normalize_lexically()`,
  exe-dir `config.txt`) and a link to the new doc.
- `docs/windows-portable-install.md` - (New) Documents portable vs standard
  install, folder layout, USB usage, drive-letter robustness, and the
  `config.txt` / launcher mechanics. Linked from `AGENTS.md` notable docs.

### Notes

- Backend tests: run with `cd backend && cargo test`; run the new file with
  `cd backend && cargo test --test test_portable_path_resolution`.
- Per project guidance, do **not** run `make qml-test`; only run tests after all
  sub-tasks of a top-level task are done, and ignore pre-existing unrelated
  failures (just confirm the build is clean).
- The `.iss` and `.ps1` changes are Windows-only and compiled/tested by the user
  on Windows (Inno Setup + Developer PowerShell); agent-side verification is
  limited to `cargo build`/`cargo test` and static review of the scripts.
- Use `try_exists()` (never `.exists()`) for any new Rust existence checks.

## Tasks

- [x] 1.0 Backend: discover `config.txt` next to the executable
  - [x] 1.1 In `backend/src/lib.rs`, add a small helper (e.g. `exe_dir() -> Option<PathBuf>`)
        that returns the parent directory of `std::env::current_exe()`, returning
        `None` on error (no panic).
  - [x] 1.2 In `init_dotenv()`, after the existing CWD `config.txt` load and
        before/at the `get_create_simsapa_dir()` step, add
        `dotenvy::from_path(exe_dir.join("config.txt")).ok()` guarded by the
        helper from 1.1.
  - [x] 1.3 Preserve `dotenvy` non-override semantics so an already-set
        `SIMSAPA_DIR` env var still wins; confirm ordering does not let a later
        source overwrite an earlier value (dotenvy does not override by default).
  - [x] 1.4 Update the `init_dotenv()` doc comment to list the new exe-dir
        `config.txt` source and the precedence.
  - [x] 1.5 Verify `cargo build` succeeds for the backend.

- [x] 2.0 Backend: resolve a relative `SIMSAPA_DIR` against the exe directory
  - [x] 2.1 Add a `normalize_lexically(path: &Path) -> PathBuf` helper that
        collapses `.`/`..` segments without touching the filesystem (do **not**
        use `std::fs::canonicalize`, which yields `\\?\` paths on Windows).
  - [x] 2.2 In `get_create_simsapa_dir()`, in the `Ok(s)` branch where
        `SIMSAPA_DIR` is read, detect whether the value is relative
        (`Path::is_relative()`); if relative, join it onto `exe_dir()` (from 1.1)
        and pass through `normalize_lexically`; if absolute, keep current behavior.
  - [x] 2.3 Ensure forward-slash values (`../SimsapaData`) resolve correctly on
        Windows (Rust treats `/` as a separator) and that a missing target dir is
        created via the existing `create_dir_all` path.
  - [x] 2.4 Confirm the resolved path flows through
        `get_create_simsapa_app_assets_path()`,
        `get_create_simsapa_appdata_db_path()`, and logging so DBs, `app-assets/`,
        and logs all land under the portable data folder.
  - [x] 2.5 Add `backend/tests/test_portable_path_resolution.rs` covering:
        relative `SIMSAPA_DIR` resolves against a given exe dir; `..` is
        normalized lexically; absolute `SIMSAPA_DIR` is returned unchanged;
        forward-slash relative path works. Structure the resolution logic so it is
        unit-testable (e.g. a pure inner function taking `exe_dir` + raw value).
  - [x] 2.6 Run `cd backend && cargo test` and confirm the new tests pass and the
        build is clean (ignore unrelated pre-existing failures).

- [ ] 3.0 Installer: Standard/Portable mode selection page + conditional privileges
  - [x] 3.1 In `simsapa-installer.iss` `[Code]`, add a custom wizard page (via
        `CreateInputOptionPage`/radio buttons) after Welcome and before the
        directory page, offering "Standard install" (default) and "Portable
        install", each with a one-line description.
  - [x] 3.2 Store the selected mode in a script-global (e.g. `IsPortable: Boolean`)
        and expose a `ShouldRunPortable` helper for `Check:` parameters.
  - [x] 3.3 Keep `PrivilegesRequiredOverridesAllowed=dialog` and set privileges so
        Portable runs as the current user (lowest) without elevation, while
        Standard retains current elevation behavior (e.g. via
        `PrivilegesRequired` handling / `InitializeSetup` logic appropriate to the
        chosen mode). Done: privilege settings left **unchanged** from pre-feature
        (default `admin` + `=dialog`); coherence with the startup install-mode
        dialog added in 3.6 (see PRD req 3 "Implemented approach").
  - [x] 3.4 Ensure the Standard path is byte-for-byte unchanged: same
        `{autopf}\Simsapa` default, same `[Files]`, `[Icons]`, VC++ redist flow,
        and uninstaller user-data prompt when Portable is not selected.
  - [x] 3.6 (This session) Add privilege/mode **coherence** in `[Code]` using
        `IsAdminInstallMode()`: default `ModePage`/`IsPortable` to Portable when
        non-elevated and Standard when elevated (`InitializeWizard`); in
        `NextButtonClick`, block choosing Standard while not in admin install mode
        (info `MsgBox`, keep user on the page); add the "Install for me only" hint
        to the Portable option text. Standard's privilege mechanism unchanged.
  - [ ] 3.5 (User) Compile the installer on Windows with Inno Setup and confirm
        the new page appears and Standard install still works end-to-end.

- [ ] 4.0 Installer: portable install behavior (target, data folder, config.txt, no uninstaller)
  - [x] 4.1 When Portable is selected, set the directory page default to a
        user-writable location (e.g. `{userdesktop}\Simsapa`) instead of
        `{autopf}\Simsapa`; let the user pick any folder (Desktop, USB, etc.).
  - [x] 4.2 Derive the sibling data folder by appending `Data` to the install
        folder name (install `…\Simsapa` -> data `…\SimsapaData`); compute its
        absolute path from `{app}` parent.
  - [x] 4.3 Create the data folder if absent; if it already exists, reuse it
        as-is (no warn/rename/delete) — preserves downloaded DBs.
  - [x] 4.4 Emit `config.txt` into `{app}` next to the exe containing exactly
        `SIMSAPA_DIR=../SimsapaData` (relative, unquoted, forward slashes; the
        suffix must match the data-folder name from 4.2). Use `[Code]`
        `SaveStringToFile` or an `[Files]` template with constant substitution.
  - [x] 4.5 Set `Uninstallable` (and skip uninstall registry writes) so Portable
        mode registers **no** uninstaller / Add-Remove-Programs entry; Standard
        mode remains fully uninstallable.
  - [x] 4.6 Make portable `[Files]` install all dist files into `{app}` exactly
        as standard (only the location and the extra `config.txt` differ).
  - [ ] 4.7 (User) Compile and run a portable install on Windows; verify
        `{app}\config.txt`, the sibling `…Data` folder, no uninstaller entry, and
        first-launch download into the data folder.

- [ ] 5.0 Installer: launcher-type choice + parent-folder launcher
  - [x] 5.1 Add a launcher-type wizard page (Portable only) offering: ".lnk
        shortcut" (note: simplest, custom icon, may break on USB drive-letter
        change) and ".cmd launcher" (note: recommended for USB, survives
        drive-letter changes). Store choice in a script-global.
  - [x] 5.2 Compute the parent of `{app}` as the launcher destination
        (e.g. `Desktop\` or the USB root).
  - [x] 5.3 For the `.lnk` choice, create a shortcut in the parent folder
        targeting `{app}\simsapadhammareader.exe` with `IconFilename`
        `{app}\simsapa.ico` (Inno `[Icons]` with `Check: ShouldRunPortable and
        ChoseLnk`, or `CreateShellLink` in `[Code]`).
  - [x] 5.4 For the `.cmd` choice, write `<ParentName>.cmd` (e.g. `Simsapa.cmd`)
        into the parent folder with contents:
        `@echo off` / `start "" "%~dp0<InstallFolderName>\simsapadhammareader.exe"`,
        substituting the actual install subfolder name so `%~dp0` resolution is
        correct.
  - [x] 5.5 Ensure the launcher does not rely on a "Start in"/CWD value — the app
        finds `config.txt` via the exe directory (task 1.0), so the launcher only
        needs to start the exe.
  - [x] 5.7 (This session) Hide Standard-only wizard elements in Portable mode
        (PRD req 10b): skip the "Select Start Menu Folder" page
        (`wpSelectProgramGroup` in `ShouldSkipPage` when `IsPortable`) and hide the
        "Create a desktop icon" task (`Check: ShouldRunStandard` on the `[Tasks]`
        `desktopicon` entry). Standard mode shows both unchanged.
  - [ ] 5.6 (User) On Windows, verify each launcher type starts the app, and test
        the `.cmd` from a USB drive mounted under a different letter on a second
        machine (no re-download, no reconfiguration).

- [x] 6.0 Build script & docs: package, verify, document
  - [x] 6.1 If file-based templates are used (config.txt / .cmd), add them under
        `assets/installer/` and ensure `build-windows.ps1` / `[Files]` make them
        available to the compiler; otherwise confirm inline `[Code]` emission
        needs no extra packaging. (Chose inline `[Code]` emission via
        `SaveStringToFile` / `CreateShellLink`; no templates, no extra packaging.)
  - [x] 6.2 Confirm `build-windows.ps1` still produces a single installer
        (`Simsapa-Setup-<version>.exe`) containing both modes — no second output
        binary, checksum step unchanged.
  - [x] 6.3 Update `PROJECT_MAP.md` with the portable-mode path resolution
        (exe-relative `SIMSAPA_DIR`, exe-dir `config.txt`) and the installer mode
        structure.
  - [x] 6.4 Add `docs/windows-portable-install.md` documenting standard vs
        portable, the folder layout diagram, USB usage, drive-letter robustness,
        and the `config.txt` / launcher mechanics; link it from `CLAUDE.md`/
        `AGENTS.md` notable docs if appropriate.
  - [x] 6.5 Final verification: `cd backend && cargo test` passes and the project
        builds with `make build -B`; confirm no regression to standard install
        review. (Windows installer compile is performed by the user.)
        Done: new portable tests (5) pass, lib tests pass (189), `make build -B`
        succeeds. Pre-existing unrelated failure in `dict_modes_filtering`
        (needs the real appdata DB at `DbManager::new()`) is ignored per project
        guidance.

- [ ] 7.0 (User) Windows manual testing — to be done together on Windows
  Prerequisite: compile the installer on Windows with Inno Setup
  (`build-windows.ps1`); confirm it compiles cleanly and produces a single
  `Simsapa-Setup-<version>.exe` (subsumes 3.5/4.7/5.6 above).

  **Privilege / mode coherence (task 3.6):**
  - [ ] 7.1 At the startup "Select Setup Install Mode" dialog choose **"Install
        for me only"** → confirm the mode page **defaults to Portable** and the
        whole portable install completes with **no UAC / admin prompt**.
  - [ ] 7.2 In that same non-elevated run, try selecting **Standard** on the mode
        page → confirm the info `MsgBox` appears and the wizard **stays on the
        mode page** (does not advance).
  - [ ] 7.3 Run elevated ("Install for all users") → confirm the mode page
        **defaults to Standard** and a normal Program Files install works exactly
        as before (location, uninstaller + user-data prompt).

  **Vestigial wizard elements (task 5.7):**
  - [ ] 7.4 In a Portable run, confirm the wizard order is Welcome → Mode →
        (VC warning if shown) → Select Directory → Launcher type → Ready, with
        **no** "Select Start Menu Folder" page and **no** "Create a desktop icon"
        checkbox on the Tasks page.
  - [ ] 7.5 In a Standard run, confirm the "Select Start Menu Folder" page and the
        "Create a desktop icon" task **still appear** as before.

  **Portable install artifacts (tasks 4.x):**
  - [ ] 7.6 After a portable install to `Desktop\Simsapa`, verify
        `Desktop\Simsapa\config.txt` contains exactly `SIMSAPA_DIR=../SimsapaData`
        (relative, unquoted, forward slash), the sibling `Desktop\SimsapaData`
        folder exists, and there is **no** Add/Remove Programs entry / `unins*.exe`.
  - [ ] 7.7 Reinstall over an existing `…Data` folder → confirm it is **reused
        as-is** (downloaded DBs preserved, no warning/rename/delete).

  **Path resolution / first launch (tasks 1.0/2.0, req 14):**
  - [ ] 7.8 First launch via the created launcher → confirm databases download
        into `…\SimsapaData\app-assets\` and logs into `…\SimsapaData\logs\`
        (not `%LOCALAPPDATA%`). Inspect a log path to confirm the resolved
        `SIMSAPA_DIR` is a clean `C:\…\SimsapaData` with **no `\\?\` prefix**
        (verifies lexical normalization on Windows, the one path not unit-tested).
  - [ ] 7.9 Second launch → confirm existing databases are detected and loaded
        with **no** re-download.

  **Launcher types (task 5.x):**
  - [ ] 7.10 `.lnk` launcher: confirm it shows the app icon and starts the app.
  - [ ] 7.11 `.cmd` launcher: confirm it starts the app detached (no lingering
        console window).
  - [ ] 7.12 USB drive-letter robustness: copy a `.cmd`-launcher portable install
        to a USB stick, run it on a **second** machine where the drive mounts
        under a **different letter** → confirm it launches and loads its databases
        with no re-download and no reconfiguration.
