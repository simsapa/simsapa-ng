# Qt 6.10.1 AppImage Build Issues

This document summarizes the issues encountered when upgrading from Qt 6.8.3 to Qt 6.10.1 for AppImage builds on Arch Linux.

## Issue 1: libtiff Library Dependency Mismatch

### Problem
Qt 6.10.1's TIFF image format plugin (`libqtiff.so`) was built against `libtiff.so.5`, but modern Arch Linux systems (as of late 2024) ship with `libtiff.so.6` (libtiff 4.7+). The SONAME change from `.so.5` to `.so.6` indicates ABI changes that make them incompatible.

When building the AppImage, linuxdeploy's Qt plugin tries to deploy dependencies for `libqtiff.so` and fails with:
```
ERROR: Could not find dependency: libtiff.so.5
ERROR: Failed to run plugin: qt (exit code: 1)
```

### Why This Didn't Happen with Qt 6.8.3
Qt 6.8.3 either:
- Was built against a compatible libtiff version, or
- Didn't include the TIFF plugin, or  
- The user's system had libtiff.so.5 at that time

### Solution
Temporarily move the problematic image format plugins (`libqtiff.so` and `libqtga.so`) out of the Qt plugins directory before linuxdeploy scans dependencies, then restore them after AppImage creation.

**Implementation in `build-appimage.sh`:**
```bash
# Create temporary directory for problematic plugins
TEMP_PLUGIN_DIR="$(mktemp -d)"

# Move plugins out temporarily
mv "$qt6_path/plugins/imageformats/libqtiff.so" "$TEMP_PLUGIN_DIR/"
mv "$qt6_path/plugins/imageformats/libqtga.so" "$TEMP_PLUGIN_DIR/"

# Run linuxdeploy...

# Restore plugins after AppImage creation
mv "$TEMP_PLUGIN_DIR/"* "$qt6_path/plugins/imageformats/"
rm -rf "$TEMP_PLUGIN_DIR"
```

**Impact:** TIFF and TGA image format support is excluded from the AppImage. This is acceptable as the application primarily uses PNG, JPG, and SVG images.

## Issue 2: Qt WebEngine FUSE Mount Incompatibility

### Problem  
Qt 6.10.1's QtWebEngine crashes with SIGSEGV when the AppImage is run from a FUSE-mounted filesystem (the default AppImage behavior). The application starts, initializes databases, but crashes immediately after `SuttaBridge::appdata_first_query()` completes, before WebEngine content loads.

**Crash behavior:**
- Extracted AppImage works perfectly (via `--appimage-extract-and-run`)
- FUSE-mounted AppImage crashes consistently
- Crash happens only when WebEngine is about to render content

### Root Cause
Qt WebEngine's `QtWebEngineProcess` helper cannot be executed from FUSE-mounted filesystems. When QtWebEngine tries to spawn its subprocess helpers, they fail to execute from the read-only FUSE mount, causing a segmentation fault.

### Why This Didn't Happen with Qt 6.8.3
This is likely a regression or behavior change in Qt 6.10.1's WebEngine implementation. The exact reason is unclear, but possible explanations:
- Changes to how QtWebEngineProcess is launched
- Different sandboxing or security model
- Modified subprocess execution path
- Changes to Chrome/Chromium base version

Qt 6.8.3 apparently handled FUSE-mounted execution differently or had workarounds that Qt 6.10.1 doesn't.

### Solutions Attempted

#### 1. Environment Variable Only (Failed)
Setting `APPIMAGE_EXTRACT_AND_RUN=1` works when explicitly set by the user, but doesn't help when users simply double-click the AppImage or run it without setting the variable.

#### 2. Embedded Launcher Script (Build Issues)
Attempted to create a launcher script inside the AppImage that detects FUSE mounting and re-execs with extract-and-run. This approach had build cache issues and complexity.

#### 3. Wrapper Script (Working Solution)
Create a small bash wrapper script that automatically uses extract-and-run mode:

**Implementation in `build-appimage.sh`:**
```bash
# After AppImage is created, rename it
mv "$APPIMAGE_NAME" "${APPIMAGE_NAME}.bin"

# Create wrapper script
cat > "$APPIMAGE_NAME" << 'WRAPPER'
#!/bin/bash
exec "$(readlink -f "$0").bin" --appimage-extract-and-run "$@"
WRAPPER
chmod +x "$APPIMAGE_NAME"
```

**Result:**
- User runs: `./Simsapa-v0.1.7-x86_64.AppImage`
- Wrapper automatically calls: `./Simsapa-v0.1.7-x86_64.AppImage.bin --appimage-extract-and-run`
- AppImage extracts to temp directory and runs normally
- Qt WebEngine works without crashes

**Trade-offs:**
- Two files to distribute (.AppImage and .AppImage.bin)
- Slightly slower startup due to extraction
- Uses more disk space temporarily (~600MB extracted vs ~270MB compressed)
- More awkward compared to Qt 6.8.3 which "just worked"

## Performance Impact

The wrapper script approach has notable performance implications:

1. **Startup Time:** Extract-and-run mode extracts the entire AppImage (~270MB) to a temporary directory on every launch, adding 2-5 seconds to startup time
2. **Disk Usage:** Requires ~600MB temporary disk space during execution
3. **User Experience:** Less seamless than direct execution

## Recommendation

**Revert to Qt 6.8.3** for AppImage and Android builds until:
1. Qt 6.10.x fixes the WebEngine FUSE compatibility issue, or
2. The Qt Company provides official guidance on QtWebEngine + AppImage, or
3. A cleaner workaround is discovered

The wrapper script solution works but is significantly less elegant than Qt 6.8.3's behavior where the AppImage could run directly from FUSE mounts without extraction.

## Files Modified

- `build-appimage.sh` - Added libtiff plugin exclusion and wrapper script creation

## Testing Results

With both fixes applied:
- ✓ AppImage builds successfully  
- ✓ No libtiff dependency errors
- ✓ Qt WebEngine loads and renders properly
- ✓ Application runs without crashes
- ✓ All functionality works correctly
- ✗ Requires wrapper script (awkward)
- ✗ Slower startup due to extraction
- ✗ Two files to distribute

## References

- Qt Bug Tracker: Search for "QtWebEngine AppImage FUSE" issues
- AppImage Best Practices: https://docs.appimage.org/packaging-guide/index.html
- linuxdeploy Qt Plugin: https://github.com/linuxdeploy/linuxdeploy-plugin-qt
