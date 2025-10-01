# PRD: Automatic Linux Desktop Launcher Creation

## Overview

### Problem Statement
Simsapa AppImage users on Linux currently need to manually create desktop launchers to integrate the application with their desktop environment. This creates friction in the user experience and prevents seamless desktop integration.

### Solution Summary
Implement automatic creation and maintenance of .desktop files for Simsapa AppImages on Linux systems, providing seamless desktop environment integration without user intervention.

### Success Metrics
- Desktop launcher automatically created on first AppImage launch
- Desktop launcher correctly updated when AppImage location changes
- Zero user intervention required for desktop integration
- Compatibility with standard Linux desktop environments

## Requirements

### Functional Requirements

#### Core Functionality
- **Automatic Desktop File Creation**: Create `~/.local/share/applications/simsapa.desktop` file automatically when running from AppImage
- **Path Detection**: Use the `APPIMAGE` environment variable which reliably contains the absolute path to the AppImage file when launched from AppImage runtime
- **Desktop File Updates**: Update existing desktop files when AppImage path changes
- **Icon Management**: Copy application icon to `~/.local/share/icons/simsapa.png`
- **Directory Creation**: Create necessary directories if they don't exist

#### Desktop File Content
- Standard .desktop file format with proper encoding
- Application name: "Simsapa"
- Icon reference: "simsapa"
- Terminal: false
- Type: Application
- Path: AppImage parent directory
- Exec: AppImage path with `QTWEBENGINE_DISABLE_SANDBOX=1` environment variable

#### Update Logic
- Check if desktop file exists and contains current AppImage path
- Update Path and Exec lines if AppImage location has changed
- Preserve user modifications to other desktop file fields

### Technical Requirements

#### Platform Support
- **Linux Only**: Feature exclusively for Linux platforms
- **AppImage Only**: Check for the presence of the `APPIMAGE` environment variable, which is automatically set by the AppImage runtime and contains the path to the AppImage file
- **Architecture Integration**: Integrate with existing Rust backend and C++ GUI layers

#### Implementation Components
1. **Rust Backend**: 
   - `create_or_update_linux_desktop_icon_file()` helper function in `backend/src/helpers.rs`
   - AppImage detection functions: `is_running_from_appimage()` and `get_appimage_path()`
2. **C++ Integration**: FFI function call from `gui.cpp` after `app.setWindowIcon()`
3. **Path Management**: Add `desktop_file_path` and `appimage_path` to `AppGlobalPaths` structure in `backend/src/lib.rs`
4. **Desktop Filename**: Set application desktop filename using Qt's `setDesktopFileName()`

#### AppImage Detection Strategy
- **Primary Method**: Check for `APPIMAGE` environment variable presence and validate the path exists
- **Validation**: Ensure the path points to an existing, readable file
- **Integration**: Use existing `AppGlobalPaths` infrastructure to store detected paths
- **Fallback**: Silent failure if AppImage detection fails (no desktop launcher creation)

#### Error Handling
- Silent failure for all error conditions (permissions, I/O, etc.)
- No user notifications or error messages
- Graceful degradation when operations fail

### Non-Functional Requirements

#### Performance
- Execute only once per application startup
- Minimal impact on application launch time
- No background processes or periodic checks

#### Reliability
- Handle missing directories gracefully
- Robust file I/O operations
- Safe concurrent access to desktop files

#### Maintainability
- Clear separation between Rust backend logic and C++ integration
- Reusable path management through global configuration
- Standard file format compliance

## Technical Specifications

### Architecture

#### File Structure
```
backend/src/
├── app_settings.rs          # Add desktop_file_path to AppGlobalPaths
└── helpers.rs               # Implement create_or_update_linux_desktop_icon_file()

cpp/
└── gui.cpp                  # Add FFI call and setDesktopFileName()
```

#### Data Flow
1. Application startup in `gui.cpp`
2. Call `setDesktopFileName()` with desktop file path
3. Call Rust FFI function `create_or_update_linux_desktop_icon_file()`
4. Rust function checks environment and creates/updates desktop file

#### Key Functions

##### Rust Backend
```rust
// In app_settings.rs - Add to AppGlobalPaths struct
pub desktop_file_path: Option<PathBuf>,
pub appimage_path: Option<PathBuf>,

// In helpers.rs
pub fn create_or_update_linux_desktop_icon_file() -> anyhow::Result<()>

// AppImage detection functions
pub fn is_running_from_appimage() -> bool
pub fn get_appimage_path() -> Option<PathBuf>
```

##### C++ Integration
```cpp
// In gui.cpp after app.setWindowIcon()
if (desktop_file_path.has_value()) {
    app.setDesktopFileName(desktop_file_path->stem());
}
// Call Rust FFI function
```

### Implementation Details

#### Desktop File Template
```ini
[Desktop Entry]
Encoding=UTF-8
Name=Simsapa
Icon=simsapa
Terminal=false
Type=Application
Path=/path/to/appimage/directory
Exec=env QTWEBENGINE_DISABLE_SANDBOX=1 /path/to/simsapa.AppImage

```

#### Path Resolution
- Desktop file: `~/.local/share/applications/simsapa.desktop`
- Icon file: `~/.local/share/icons/simsapa.png`
- Source icon: `assets/icons/appicons/simsapa.png`

#### Update Detection
- Read existing desktop file content
- Check if current AppImage path is present in file
- Use regex patterns to update Path and Exec lines while preserving user modifications

## Dependencies

### Internal Dependencies
- Existing `AppGlobalPaths` structure in Rust backend
- CXX-Qt bridge infrastructure for FFI calls
- Asset management system for icon access

### External Dependencies
- Standard Linux filesystem permissions for user directories
- `APPIMAGE` environment variable (automatically provided by AppImage runtime)
- Qt6 `setDesktopFileName()` functionality

### Environment Variable Details
The AppImage runtime automatically sets these environment variables when the application is launched from an AppImage:
- `APPIMAGE`: Absolute path to the AppImage file (e.g., `/home/user/Downloads/Simsapa-v1.0.0-x86_64.AppImage`)
- `APPDIR`: Path to the mounted SquashFS filesystem (e.g., `/tmp/.mount_SimsapXXXXXX`)
- `OWD`: Original working directory when AppImage was launched

These variables are reliable and present in both Type 1 and Type 2 AppImages.

### Build System
- No additional build dependencies required
- Utilizes existing Rust and C++ compilation pipeline

## Testing Strategy

No need to add unit and integration tests.

## Implementation Phases

### Phase 1: Backend Implementation
- Add `desktop_file_path` and `appimage_path` to `AppGlobalPaths` in `backend/src/lib.rs`
- Implement AppImage detection functions in `backend/src/helpers.rs`:
  - `is_running_from_appimage()` - checks `APPIMAGE` env var and validates path
  - `get_appimage_path()` - returns validated AppImage path
- Implement `create_or_update_linux_desktop_icon_file()` in `backend/src/helpers.rs`
- Add FFI bridge function to expose functionality to C++

### Phase 2: Frontend Integration
- Modify `gui.cpp` to call desktop filename setting
- Add FFI function call after window icon setup
- Test integration between C++ and Rust layers

### Phase 3: Testing and Validation
- Manual testing by the user

### Mitigation Strategies
- Silent error handling eliminates user-facing failures
- Standard file formats ensure broad compatibility
- Single execution per startup minimizes performance impact

## Success Criteria

### Acceptance Criteria
- [ ] Desktop launcher created automatically on AppImage first run
- [ ] Desktop launcher updated when AppImage path changes
- [ ] Application icon properly installed and referenced
- [ ] Qt application correctly identifies with desktop file
- [ ] No user intervention required for any functionality
- [ ] Silent operation with no error messages or notifications
- [ ] Compatibility with major Linux desktop environments

### Definition of Done
- All functional requirements implemented
- Documentation updated in PROJECT_MAP.md
- Feature verified with user manually testing the AppImage 
