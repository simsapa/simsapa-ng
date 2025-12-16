# AppImage Build Instructions

This document describes how to build an AppImage for the Simsapa application.

## Prerequisites

1. **Qt6 Installation**: Ensure Qt6 is installed with WebEngine support
   - Default path: `~/Qt/6.8.3/gcc_64/`
   - Alternative: `/opt/Qt/6.8.3/gcc_64/`
   - Or set `QT_BASE_DIR` environment variable

2. **System Dependencies**:
   ```bash
   sudo apt install wget file build-essential cmake
   ```

3. **Rust toolchain**: Required for building the backend
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

## Building AppImage

### Quick Build
```bash
make appimage
```

### Manual Build
```bash
# 1. Build the application
make build -B

# 2. Create AppImage
./build-appimage.sh
```

## Build Process

The build script performs these steps:

1. **Downloads Tools**: Downloads `linuxdeploy` and `linuxdeploy-plugin-qt` if not present
2. **Builds App**: Compiles the Qt6/Rust application using CMake
3. **Creates AppDir**: Sets up the AppImage directory structure with:
   - Executable: `usr/bin/simsapadhammareader`
   - Desktop file: `simsapa.desktop`
   - Icon: `simsapa.png`
4. **Bundles Qt**: Uses linuxdeploy-plugin-qt to bundle Qt6 libraries and WebEngine
5. **Creates AppImage**: Packages everything into a portable AppImage file

## Output

The build creates:
- `simsapa-0.1.0-x86_64.AppImage` - The portable application
- `simsapa.AppDir/` - Temporary directory structure
- `appimage-tools/` - Downloaded build tools

## Running

```bash
# Make executable (if needed)
chmod +x simsapa-0.1.0-x86_64.AppImage

# Run the application
./simsapa-0.1.0-x86_64.AppImage
```

## Cleanup

```bash
make appimage-clean
```

## Troubleshooting

### Strip Errors (Unknown Type Section)
If you encounter errors like:
```
ERROR: Strip call failed: ... unknown type [0x13] section `.relr.dyn'
```

This happens with newer system libraries. The build script now automatically handles this by:
- Setting `NO_STRIP=1` to disable stripping
- Using system strip instead of the AppImage's embedded strip
- Trying fallback options if the first attempt fails

### Qt6 Not Found
Set the Qt6 path manually:
```bash
export QT_BASE_DIR="/path/to/your/qt6/installation"
./build-appimage.sh
```

### Missing WebEngine
Ensure your Qt6 installation includes WebEngine:
- Check that `$QT_BASE_DIR/lib/libQt6WebEngineCore.so*` exists
- Install Qt6 WebEngine package: `sudo apt install qt6-webengine-dev`

### Library Dependencies
If the AppImage fails to run on other systems, check:
```bash
# Test on a clean system or container
docker run -it --rm -v $PWD:/app ubuntu:20.04
cd /app && ./simsapa-0.1.0-x86_64.AppImage
```

### Old linuxdeploy Tools
The script automatically downloads fresh copies if tools are older than 7 days. To force fresh downloads:
```bash
rm -rf appimage-tools/
./build-appimage.sh
```

### Build Environment Issues
If you continue having issues, try building in a clean environment:
```bash
# Using Ubuntu 20.04 container
docker run -it --rm -v $PWD:/workspace ubuntu:20.04
apt update && apt install -y wget file build-essential cmake qt6-base-dev qt6-webengine-dev
cd /workspace && ./build-appimage.sh
```

### Debug Mode
Enable verbose output:
```bash
export VERBOSE=1
./build-appimage.sh
```

## Distribution

The generated AppImage is portable and should run on most Linux distributions with:
- glibc 2.17 or newer
- Linux kernel 2.6.32 or newer
- X11 or Wayland display server

For distribution, you can:
1. Upload to GitHub releases
2. Host on your website
3. Submit to AppImageHub