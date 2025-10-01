# Implementation Tasks: Linux Desktop Launcher Creation

## Overview
Tasks for implementing automatic Linux desktop launcher creation for Simsapa AppImage users, following the PRD at `tasks/prd-linux-desktop-launcher.md`.

## Phase 1: Backend Implementation

### Task 1.1: Extend AppGlobalPaths Structure ✓
**Priority:** High  
**Estimated Time:** 30 minutes  
**Dependencies:** None

**Description:** Add desktop file and AppImage path tracking to the global paths structure.

**Acceptance Criteria:**
- [x] Add `desktop_file_path: Option<PathBuf>` to `AppGlobalPaths` in `backend/src/lib.rs`
- [x] Add `appimage_path: Option<PathBuf>` to `AppGlobalPaths` in `backend/src/lib.rs`
- [x] Update `AppGlobalPaths::new()` to initialize these fields as `None`
- [x] Ensure compilation succeeds after changes

**Files to modify:**
- `backend/src/lib.rs`

### Task 1.2: Implement AppImage Detection Functions ✓
**Priority:** High  
**Estimated Time:** 45 minutes  
**Dependencies:** Task 1.1

**Description:** Create utility functions to detect if running from AppImage and get the AppImage path.

**Acceptance Criteria:**
- [x] Implement `is_running_from_appimage() -> bool` in `backend/src/helpers.rs`
  - [x] Check for `APPIMAGE` environment variable presence
  - [x] Validate that the path exists and is readable
  - [x] Return `false` for any error conditions
- [x] Implement `get_appimage_path() -> Option<PathBuf>` in `backend/src/helpers.rs`
  - [x] Return validated AppImage path from environment variable
  - [x] Return `None` if detection fails or path is invalid
- [x] Add appropriate imports and dependencies
- [x] Add proper error handling with silent failures

**Files to modify:**
- `backend/src/helpers.rs`

### Task 1.3: Implement Desktop File Creation Logic ✓
**Priority:** High  
**Estimated Time:** 90 minutes  
**Dependencies:** Task 1.2

**Description:** Core function to create or update Linux desktop launcher files.

**Acceptance Criteria:**
- [x] Implement `create_or_update_linux_desktop_icon_file() -> anyhow::Result<()>` in `backend/src/helpers.rs`, reference the code block in /home/gambhiro/prods/apps/simsapa-ng-project/simsapa-ng/tasks/create-desktop-launcher-on-linux.txt
- [x] Check if running from AppImage, exit early if not
- [x] Create necessary directories (`~/.local/share/applications/`, `~/.local/share/icons/`)
- [x] Generate desktop file content with correct template:
  - [x] `[Desktop Entry]` section
  - [x] `Encoding=UTF-8`
  - [x] `Name=Simsapa`
  - [x] `Icon=simsapa`
  - [x] `Terminal=false`
  - [x] `Type=Application`
  - [x] `Path=` (AppImage parent directory)
  - [x] `Exec=env QTWEBENGINE_DISABLE_SANDBOX=1 /path/to/appimage`
- [x] Handle existing desktop file updates:
  - [x] Read existing file content
  - [x] Check if AppImage path matches current location
  - [x] Update only `Path` and `Exec` lines if changed
  - [x] Preserve other user modifications
- [x] Copy application icon from `assets/icons/appicons/simsapa.png` to `~/.local/share/icons/simsapa.png`
- [x] Implement robust error handling with silent failures
- [x] Add proper file I/O with appropriate permissions

**Files to modify:**
- `backend/src/helpers.rs`

### Task 1.4: Create FFI Bridge Function ✓
**Priority:** High  
**Estimated Time:** 30 minutes  
**Dependencies:** Task 1.3

**Description:** Expose desktop launcher functionality to C++ layer through FFI.

**Acceptance Criteria:**
- [x] Add FFI function declaration in appropriate bridge file
- [x] Ensure function is callable from C++ code
- [x] Handle any CXX-Qt specific requirements
- [x] Test compilation of bridge layer

**Files to modify:**
- `bridges/src/api.rs` or appropriate bridge file
- Update `bridges/build.rs` if needed

## Phase 2: Frontend Integration

### Task 2.1: Integrate Desktop Filename Setting
**Priority:** Medium  
**Estimated Time:** 30 minutes  
**Dependencies:** Task 1.4

**Description:** Configure Qt application to use the correct desktop filename.

**Acceptance Criteria:**
- [ ] Locate the `app.setWindowIcon()` call in `cpp/gui.cpp`
- [ ] Add call to `app.setDesktopFileName()` after window icon setup
- [ ] Use the desktop file path from `AppGlobalPaths` 
- [ ] Extract filename stem (without extension) for the call
- [ ] Handle case where desktop file path is not available

**Files to modify:**
- `cpp/gui.cpp`

### Task 2.2: Add FFI Function Call
**Priority:** Medium  
**Estimated Time:** 15 minutes  
**Dependencies:** Task 2.1

**Description:** Call the Rust desktop launcher creation function from C++.

**Acceptance Criteria:**
- [ ] Add FFI function call to `create_or_update_linux_desktop_icon_file()` 
- [ ] Place call after `app.setWindowIcon()` and `setDesktopFileName()`
- [ ] Handle any potential FFI errors gracefully
- [ ] Ensure no blocking operations that impact startup time

**Files to modify:**
- `cpp/gui.cpp`

### Task 2.3: Update Global Paths Initialization
**Priority:** Medium  
**Estimated Time:** 30 minutes  
**Dependencies:** Task 1.2

**Description:** Populate AppImage and desktop file paths during application initialization.

**Acceptance Criteria:**
- [ ] Modify `AppGlobalPaths` initialization to detect AppImage path
- [ ] Set `appimage_path` field using `get_appimage_path()`
- [ ] Set `desktop_file_path` to `~/.local/share/applications/simsapa.desktop` when AppImage detected
- [ ] Ensure paths are available when FFI function is called

**Files to modify:**
- `backend/src/lib.rs` or appropriate initialization code

## Phase 3: Testing and Validation

### Task 3.1: Build System Verification
**Priority:** Medium  
**Estimated Time:** 15 minutes  
**Dependencies:** All previous tasks

**Description:** Ensure all changes compile correctly and don't break existing functionality.

**Acceptance Criteria:**
- [ ] Run `make build -B` successfully
- [ ] Verify no compilation errors in Rust backend
- [ ] Verify no compilation errors in C++ frontend
- [ ] Confirm all FFI bindings work correctly

**Commands to run:**
```bash
make build -B
```

### Task 3.2: AppImage Testing Preparation
**Priority:** Low  
**Estimated Time:** 30 minutes  
**Dependencies:** Task 3.1

**Description:** Prepare testing instructions and verification steps for manual AppImage testing.

**Acceptance Criteria:**
- [ ] Document test scenarios for user validation:
  - [ ] First run from AppImage (desktop file creation)
  - [ ] Move AppImage to different location (desktop file update)
  - [ ] Verify icon installation and display
  - [ ] Check desktop integration works across different desktop environments
- [ ] Create test checklist for user validation
- [ ] Document expected file locations and content

**Files to create:**
- Testing instructions in this task file or separate document

### Task 3.3: Documentation Updates
**Priority:** Low  
**Estimated Time:** 20 minutes  
**Dependencies:** Task 3.1

**Description:** Update project documentation to reflect new functionality.

**Acceptance Criteria:**
- [ ] Update `PROJECT_MAP.md` with new functions and their locations
- [ ] Document the desktop launcher feature and its components
- [ ] Update any relevant architecture documentation
- [ ] Note Linux-specific functionality and AppImage requirements

**Files to modify:**
- `PROJECT_MAP.md`

## Risk Mitigation

### Technical Risks
- **File permissions issues:** Handled through silent error handling
- **Environment variable availability:** Validated before use with fallback
- **Desktop environment compatibility:** Using standard .desktop file format

### Implementation Risks
- **FFI integration complexity:** Start with simple function calls, verify compilation early
- **Path handling across platforms:** Linux-only implementation reduces complexity
- **AppImage detection reliability:** Multiple validation steps for robustness

## Definition of Done

**Feature Complete When:**
- [ ] All Phase 1 tasks completed (backend implementation)
- [ ] All Phase 2 tasks completed (frontend integration)  
- [ ] All Phase 3 tasks completed (testing and documentation)
- [ ] User has manually verified AppImage desktop integration works
- [ ] No regressions in existing functionality
- [ ] PROJECT_MAP.md updated with new components

## Manual Testing Checklist (for User)

**Test Case 1: First Run Desktop File Creation**
- [ ] Download/build fresh AppImage
- [ ] Ensure no existing `~/.local/share/applications/simsapa.desktop`
- [ ] Run AppImage: `./simsapa.AppImage`
- [ ] Verify desktop file created with correct content
- [ ] Verify icon copied to `~/.local/share/icons/simsapa.png`
- [ ] Verify application appears in desktop environment app menu

**Test Case 2: AppImage Path Change Update**
- [ ] Move AppImage to different directory
- [ ] Run AppImage from new location
- [ ] Verify desktop file `Exec` and `Path` lines updated
- [ ] Verify other desktop file content preserved
- [ ] Verify desktop launcher still works

**Test Case 3: Non-AppImage Execution**
- [ ] Run application not from AppImage (if possible)
- [ ] Verify no desktop file operations attempted
- [ ] Verify application runs normally

**Expected File Locations:**
- Desktop file: `~/.local/share/applications/simsapa.desktop`
- Icon file: `~/.local/share/icons/simsapa.png`

**Expected Desktop File Content:**
```ini
[Desktop Entry]
Encoding=UTF-8
Name=Simsapa
Icon=simsapa
Terminal=false
Type=Application
Path=/path/to/appimage/directory
Exec=env QTWEBENGINE_DISABLE_SANDBOX=1 /full/path/to/simsapa.AppImage
```

## Relevant Files

**Modified Files:**
- `backend/src/lib.rs` - Extended AppGlobalPaths structure with desktop_file_path and appimage_path fields
- `backend/src/helpers.rs` - Added AppImage detection functions and desktop file creation logic  
- `bridges/src/api.rs` - Added FFI bridge function for desktop file creation

**File Changes:**
- `backend/src/lib.rs`: Added `desktop_file_path` and `appimage_path` Optional PathBuf fields to AppGlobalPaths
- `backend/src/helpers.rs`: Implemented `is_running_from_appimage()`, `get_appimage_path()`, and `create_or_update_linux_desktop_icon_file()` functions
- `bridges/src/api.rs`: Added `create_linux_desktop_icon_file()` FFI function for C++ integration
