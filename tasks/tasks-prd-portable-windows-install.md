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
- `build-windows.ps1` - Windows build/packaging script. Ensure any new templates
  are available to the installer and the output stays a single installer.
- `assets/installer/config.txt.template` - (New, optional) Template for the
  portable `config.txt` if a file-based template is preferred over inline
  emission from `[Code]`.
- `assets/installer/Simsapa-portable.cmd.template` - (New, optional) Template for
  the `.cmd` relative launcher if preferred over inline emission.
- `PROJECT_MAP.md` - Update to note portable-mode path resolution and installer
  modes.
- `docs/windows-portable-install.md` - (New) Document portable vs standard
  install, folder layout, USB usage, and the `config.txt` / launcher mechanics.

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

- [ ] 1.0 Backend: discover `config.txt` next to the executable
  - [ ] 1.1 In `backend/src/lib.rs`, add a small helper (e.g. `exe_dir() -> Option<PathBuf>`)
        that returns the parent directory of `std::env::current_exe()`, returning
        `None` on error (no panic).
  - [ ] 1.2 In `init_dotenv()`, after the existing CWD `config.txt` load and
        before/at the `get_create_simsapa_dir()` step, add
        `dotenvy::from_path(exe_dir.join("config.txt")).ok()` guarded by the
        helper from 1.1.
  - [ ] 1.3 Preserve `dotenvy` non-override semantics so an already-set
        `SIMSAPA_DIR` env var still wins; confirm ordering does not let a later
        source overwrite an earlier value (dotenvy does not override by default).
  - [ ] 1.4 Update the `init_dotenv()` doc comment to list the new exe-dir
        `config.txt` source and the precedence.
  - [ ] 1.5 Verify `cargo build` succeeds for the backend.

- [ ] 2.0 Backend: resolve a relative `SIMSAPA_DIR` against the exe directory
  - [ ] 2.1 Add a `normalize_lexically(path: &Path) -> PathBuf` helper that
        collapses `.`/`..` segments without touching the filesystem (do **not**
        use `std::fs::canonicalize`, which yields `\\?\` paths on Windows).
  - [ ] 2.2 In `get_create_simsapa_dir()`, in the `Ok(s)` branch where
        `SIMSAPA_DIR` is read, detect whether the value is relative
        (`Path::is_relative()`); if relative, join it onto `exe_dir()` (from 1.1)
        and pass through `normalize_lexically`; if absolute, keep current behavior.
  - [ ] 2.3 Ensure forward-slash values (`../SimsapaData`) resolve correctly on
        Windows (Rust treats `/` as a separator) and that a missing target dir is
        created via the existing `create_dir_all` path.
  - [ ] 2.4 Confirm the resolved path flows through
        `get_create_simsapa_app_assets_path()`,
        `get_create_simsapa_appdata_db_path()`, and logging so DBs, `app-assets/`,
        and logs all land under the portable data folder.
  - [ ] 2.5 Add `backend/tests/test_portable_path_resolution.rs` covering:
        relative `SIMSAPA_DIR` resolves against a given exe dir; `..` is
        normalized lexically; absolute `SIMSAPA_DIR` is returned unchanged;
        forward-slash relative path works. Structure the resolution logic so it is
        unit-testable (e.g. a pure inner function taking `exe_dir` + raw value).
  - [ ] 2.6 Run `cd backend && cargo test` and confirm the new tests pass and the
        build is clean (ignore unrelated pre-existing failures).

- [ ] 3.0 Installer: Standard/Portable mode selection page + conditional privileges
  - [ ] 3.1 In `simsapa-installer.iss` `[Code]`, add a custom wizard page (via
        `CreateInputOptionPage`/radio buttons) after Welcome and before the
        directory page, offering "Standard install" (default) and "Portable
        install", each with a one-line description.
  - [ ] 3.2 Store the selected mode in a script-global (e.g. `IsPortable: Boolean`)
        and expose a `ShouldRunPortable` helper for `Check:` parameters.
  - [ ] 3.3 Keep `PrivilegesRequiredOverridesAllowed=dialog` and set privileges so
        Portable runs as the current user (lowest) without elevation, while
        Standard retains current elevation behavior (e.g. via
        `PrivilegesRequired` handling / `InitializeSetup` logic appropriate to the
        chosen mode).
  - [ ] 3.4 Ensure the Standard path is byte-for-byte unchanged: same
        `{autopf}\Simsapa` default, same `[Files]`, `[Icons]`, VC++ redist flow,
        and uninstaller user-data prompt when Portable is not selected.
  - [ ] 3.5 (User) Compile the installer on Windows with Inno Setup and confirm
        the new page appears and Standard install still works end-to-end.

- [ ] 4.0 Installer: portable install behavior (target, data folder, config.txt, no uninstaller)
  - [ ] 4.1 When Portable is selected, set the directory page default to a
        user-writable location (e.g. `{userdesktop}\Simsapa`) instead of
        `{autopf}\Simsapa`; let the user pick any folder (Desktop, USB, etc.).
  - [ ] 4.2 Derive the sibling data folder by appending `Data` to the install
        folder name (install `…\Simsapa` -> data `…\SimsapaData`); compute its
        absolute path from `{app}` parent.
  - [ ] 4.3 Create the data folder if absent; if it already exists, reuse it
        as-is (no warn/rename/delete) — preserves downloaded DBs.
  - [ ] 4.4 Emit `config.txt` into `{app}` next to the exe containing exactly
        `SIMSAPA_DIR=../SimsapaData` (relative, unquoted, forward slashes; the
        suffix must match the data-folder name from 4.2). Use `[Code]`
        `SaveStringToFile` or an `[Files]` template with constant substitution.
  - [ ] 4.5 Set `Uninstallable` (and skip uninstall registry writes) so Portable
        mode registers **no** uninstaller / Add-Remove-Programs entry; Standard
        mode remains fully uninstallable.
  - [ ] 4.6 Make portable `[Files]` install all dist files into `{app}` exactly
        as standard (only the location and the extra `config.txt` differ).
  - [ ] 4.7 (User) Compile and run a portable install on Windows; verify
        `{app}\config.txt`, the sibling `…Data` folder, no uninstaller entry, and
        first-launch download into the data folder.

- [ ] 5.0 Installer: launcher-type choice + parent-folder launcher
  - [ ] 5.1 Add a launcher-type wizard page (Portable only) offering: ".lnk
        shortcut" (note: simplest, custom icon, may break on USB drive-letter
        change) and ".cmd launcher" (note: recommended for USB, survives
        drive-letter changes). Store choice in a script-global.
  - [ ] 5.2 Compute the parent of `{app}` as the launcher destination
        (e.g. `Desktop\` or the USB root).
  - [ ] 5.3 For the `.lnk` choice, create a shortcut in the parent folder
        targeting `{app}\simsapadhammareader.exe` with `IconFilename`
        `{app}\simsapa.ico` (Inno `[Icons]` with `Check: ShouldRunPortable and
        ChoseLnk`, or `CreateShellLink` in `[Code]`).
  - [ ] 5.4 For the `.cmd` choice, write `<ParentName>.cmd` (e.g. `Simsapa.cmd`)
        into the parent folder with contents:
        `@echo off` / `start "" "%~dp0<InstallFolderName>\simsapadhammareader.exe"`,
        substituting the actual install subfolder name so `%~dp0` resolution is
        correct.
  - [ ] 5.5 Ensure the launcher does not rely on a "Start in"/CWD value — the app
        finds `config.txt` via the exe directory (task 1.0), so the launcher only
        needs to start the exe.
  - [ ] 5.6 (User) On Windows, verify each launcher type starts the app, and test
        the `.cmd` from a USB drive mounted under a different letter on a second
        machine (no re-download, no reconfiguration).

- [ ] 6.0 Build script & docs: package, verify, document
  - [ ] 6.1 If file-based templates are used (config.txt / .cmd), add them under
        `assets/installer/` and ensure `build-windows.ps1` / `[Files]` make them
        available to the compiler; otherwise confirm inline `[Code]` emission
        needs no extra packaging.
  - [ ] 6.2 Confirm `build-windows.ps1` still produces a single installer
        (`Simsapa-Setup-<version>.exe`) containing both modes — no second output
        binary, checksum step unchanged.
  - [ ] 6.3 Update `PROJECT_MAP.md` with the portable-mode path resolution
        (exe-relative `SIMSAPA_DIR`, exe-dir `config.txt`) and the installer mode
        structure.
  - [ ] 6.4 Add `docs/windows-portable-install.md` documenting standard vs
        portable, the folder layout diagram, USB usage, drive-letter robustness,
        and the `config.txt` / launcher mechanics; link it from `CLAUDE.md`/
        `AGENTS.md` notable docs if appropriate.
  - [ ] 6.5 Final verification: `cd backend && cargo test` passes and the project
        builds with `make build -B`; confirm no regression to standard install
        review. (Windows installer compile is performed by the user.)
