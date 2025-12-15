#!/bin/bash

set -e

# AppImage build script for Simsapa
# This script creates an AppImage from the built Qt6 application

# Default values - can be overridden by command line arguments
APP_NAME=""
APP_VERSION=""
ARCH="x86_64"
OS_SUFFIX=""
BUILD_DIR="./build/simsapadhammareader"

# Function to extract app name from desktop file
get_app_name_from_desktop() {
    if [ -f "simsapa.desktop" ]; then
        grep "^Name=" simsapa.desktop | cut -d'=' -f2 | tr -d '\r\n'
    else
        echo "Simsapa"
    fi
}

# Function to extract version from bridges/Cargo.toml
get_version_from_cargo_toml() {
    if [ -f "bridges/Cargo.toml" ]; then
        grep "^version = " bridges/Cargo.toml | head -n1 | cut -d'"' -f2 | sed 's/^/v/'
    else
        echo "v0.1.0"
    fi
}

# Function to show usage
show_usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  --app-name NAME      Set application name (default: read from .desktop file)"
    echo "  --app-version VER    Set application version (default: read from bridges/Cargo.toml)"
    echo "  --arch ARCH          Set architecture (default: x86_64)"
    echo "  --os-suffix SUFFIX   Add OS suffix to filename (e.g., -ubuntu24)"
    echo "  --clean              Clean build artifacts before building"
    echo "  --force-download     Force download fresh linuxdeploy tools"
    echo "  --help, -h           Show this help message"
    echo ""
}

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if required tools are installed
check_dependencies() {
    print_status "Checking dependencies..."

    local missing_deps=()

    if ! command -v wget &> /dev/null; then
        missing_deps+=("wget")
    fi

    if ! command -v file &> /dev/null; then
        missing_deps+=("file")
    fi

    if [ ${#missing_deps[@]} -ne 0 ]; then
        print_error "Missing dependencies: ${missing_deps[*]}"
        print_error "Please install: sudo apt install ${missing_deps[*]}"
        exit 1
    fi
}

# Download linuxdeploy tools if not present
download_tools() {
    print_status "Downloading AppImage tools..."

    local tools_dir="./appimage-tools"
    mkdir -p "$tools_dir"

    # Download linuxdeploy
    if [ ! -f "$tools_dir/linuxdeploy-x86_64.AppImage" ]; then
        print_status "Downloading linuxdeploy..."
        if ! wget -O "$tools_dir/linuxdeploy-x86_64.AppImage" \
            "https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage"; then
            print_error "Failed to download linuxdeploy"
            exit 1
        fi
        chmod +x "$tools_dir/linuxdeploy-x86_64.AppImage"
    else
        print_status "Using existing linuxdeploy tool"
    fi

    # Download linuxdeploy-plugin-qt
    if [ ! -f "$tools_dir/linuxdeploy-plugin-qt-x86_64.AppImage" ]; then
        print_status "Downloading linuxdeploy-plugin-qt..."
        if ! wget -O "$tools_dir/linuxdeploy-plugin-qt-x86_64.AppImage" \
            "https://github.com/linuxdeploy/linuxdeploy-plugin-qt/releases/download/continuous/linuxdeploy-plugin-qt-x86_64.AppImage"; then
            print_error "Failed to download linuxdeploy-plugin-qt"
            exit 1
        fi
        chmod +x "$tools_dir/linuxdeploy-plugin-qt-x86_64.AppImage"
    else
        print_status "Using existing linuxdeploy-plugin-qt tool"
    fi

    export PATH="$PWD/$tools_dir:$PATH"

    # Test that the tools work
    print_status "Testing linuxdeploy tools..."
    if ! "$tools_dir/linuxdeploy-x86_64.AppImage" --version >/dev/null 2>&1; then
        print_warning "linuxdeploy test failed, this might cause issues"
    fi
}

# Build the application first
build_app() {
    print_status "Building application..."

    if [ ! -f "$BUILD_DIR/simsapadhammareader" ]; then
        print_status "Building simsapa..."
        make build -B
    else
        print_warning "Application already built. Use 'make build -B' to rebuild."
    fi

    if [ ! -f "$BUILD_DIR/simsapadhammareader" ]; then
        print_error "Build failed - executable not found"
        exit 1
    fi
}

# Create the AppDir structure
create_appdir() {
    print_status "Creating AppDir structure..."

    # Remove existing AppDir
    rm -rf "$APPDIR_NAME"

    # Create AppDir structure
    mkdir -p "$APPDIR_NAME/usr/bin"
    mkdir -p "$APPDIR_NAME/usr/share/applications"
    mkdir -p "$APPDIR_NAME/usr/share/icons/hicolor/256x256/apps"

    # Copy the executable
    cp "$BUILD_DIR/simsapadhammareader" "$APPDIR_NAME/usr/bin/"

    # Copy icon
    if [ -f "assets/icons/appicons/simsapa.png" ]; then
        cp "assets/icons/appicons/simsapa.png" "$APPDIR_NAME/usr/share/icons/hicolor/256x256/apps/"
        cp "assets/icons/appicons/simsapa.png" "$APPDIR_NAME/"
    else
        print_warning "Icon not found at assets/icons/appicons/simsapa.png"
    fi

    # Copy desktop file
    cp "simsapa.desktop" "$APPDIR_NAME/usr/share/applications/"
    cp "simsapa.desktop" "$APPDIR_NAME/"

    # Make the desktop file executable
    chmod +x "$APPDIR_NAME/simsapa.desktop"

    print_status "AppDir structure created"
}

# Create the AppImage
create_appimage() {
    print_status "Creating AppImage..."

    # Remove existing AppImage
    rm -f "$APPIMAGE_NAME"

    # Set Qt6 path - adjust this to your Qt6 installation
    local qt6_path=""

    # Check if QT_BASE_DIR is set from environment (e.g., GitHub Actions)
    if [ -n "$QT_BASE_DIR" ] && [ -d "$QT_BASE_DIR" ]; then
        qt6_path="$QT_BASE_DIR"
        print_status "Using Qt6 from QT_BASE_DIR environment variable: $qt6_path"
    # Try to find Qt6 installation in standard locations
    elif [ -d "$HOME/Qt/6.10.1/gcc_64" ]; then
        qt6_path="$HOME/Qt/6.10.1/gcc_64"
    elif [ -d "/opt/Qt/6.10.1/gcc_64" ]; then
        qt6_path="/opt/Qt/6.10.1/gcc_64"
    elif command -v qmake6 &> /dev/null; then
        qt6_path="$(dirname $(dirname $(which qmake6)))"
    else
        print_error "Qt6 installation not found. Please set QT_BASE_DIR environment variable."
        exit 1
    fi

    print_status "Using Qt6 from: $qt6_path"

    # Set environment variables for linuxdeploy
    export QT_BASE_DIR="$qt6_path"
    export LD_LIBRARY_PATH="$qt6_path/lib:$LD_LIBRARY_PATH"
    export QML_SOURCES_PATHS="$qt6_path/qml:./assets/qml"
    export PATH="$qt6_path/bin:$PATH"

    # Set qmake path for the Qt plugin (only if not already set)
    if [ -z "$QMAKE" ]; then
        export QMAKE="$qt6_path/bin/qmake"
    fi
    print_status "Using QMAKE: $QMAKE"

    # QtWebEngine specific settings
    export QT_QPA_PLATFORM_PLUGIN_PATH="$qt6_path/plugins"
    export QTWEBENGINE_RESOURCES_PATH="$qt6_path/resources"
    export QTWEBENGINE_LOCALES_PATH="$qt6_path/translations/qtwebengine_locales"

    # Workarounds for newer system libraries compatibility
    export NO_STRIP=1
    export DISABLE_COPYRIGHT_FILES_DEPLOYMENT=1

    # Try to use system strip if available and newer
    if command -v strip &> /dev/null; then
        export STRIP="$(which strip)"
    fi

    print_status "Running linuxdeploy..."

    # Manually copy QtWebEngine resources first
    print_status "Ensuring QtWebEngine resources are available..."
    if [ -d "$qt6_path/resources" ]; then
        mkdir -p "$APPDIR_NAME/usr/resources"
        cp -r "$qt6_path/resources"/* "$APPDIR_NAME/usr/resources/" 2>/dev/null || true
    fi

    if [ -d "$qt6_path/translations/qtwebengine_locales" ]; then
        mkdir -p "$APPDIR_NAME/usr/translations/qtwebengine_locales"
        cp -r "$qt6_path/translations/qtwebengine_locales"/* "$APPDIR_NAME/usr/translations/qtwebengine_locales/" 2>/dev/null || true
    fi

    # First attempt with standard options
    print_status "Running linuxdeploy with standard options..."

    if ! linuxdeploy-x86_64.AppImage \
        --appdir "$APPDIR_NAME" \
        --plugin qt \
        --output appimage \
        --desktop-file "$APPDIR_NAME/simsapa.desktop" \
        --icon-file "$APPDIR_NAME/simsapa.png" \
        --executable "$APPDIR_NAME/usr/bin/simsapadhammareader" 2>&1; then

        print_warning "First attempt failed, trying with fallback options..."

        # Fallback: try without stripping and with different options
        print_status "Trying fallback options with verbose output..."
        export LINUXDEPLOY_OUTPUT_VERSION=1

        if ! linuxdeploy-x86_64.AppImage \
            --appdir "$APPDIR_NAME" \
            --plugin qt \
            --output appimage \
            --desktop-file "$APPDIR_NAME/simsapa.desktop" \
            --icon-file "$APPDIR_NAME/simsapa.png" \
            --executable "$APPDIR_NAME/usr/bin/simsapadhammareader" \
            --verbosity=1 2>&1; then

            print_error "Both AppImage creation attempts failed"
            print_error "This might be due to newer system libraries incompatible with linuxdeploy"
            print_error "Try updating linuxdeploy or using a different build environment"
            exit 1
        fi
    fi

    # linuxdeploy creates AppImage with different naming convention
    # It uses the desktop file name, so look for the actual created file
    local created_appimage=""
    if [ -f "Simsapa-x86_64.AppImage" ]; then
        created_appimage="Simsapa-x86_64.AppImage"
    elif [ -f "$APPIMAGE_NAME" ]; then
        created_appimage="$APPIMAGE_NAME"
    fi

    if [ -n "$created_appimage" ]; then
        print_status "AppImage created successfully: $created_appimage"

        # Rename to expected filename if different
        if [ "$created_appimage" != "$APPIMAGE_NAME" ]; then
            print_status "Renaming $created_appimage to $APPIMAGE_NAME"
            mv "$created_appimage" "$APPIMAGE_NAME"
        fi

        # Make it executable
        chmod +x "$APPIMAGE_NAME"

        # Test AppImage functionality
        print_status "Testing AppImage functionality..."
        echo "File size: $(stat -c%s "$APPIMAGE_NAME") bytes"
        echo "File type: $(file "$APPIMAGE_NAME")"

        # Test if AppImage can extract itself (this is the real test)
        print_status "Testing AppImage self-extraction..."
        if "$APPIMAGE_NAME" --appimage-help > /dev/null 2>&1; then
            print_status "✓ AppImage runtime works correctly"
        else
            print_error "✗ AppImage runtime test failed!"
            print_error "This AppImage may not launch properly"
            # Try to get more info about why it failed
            echo "Attempting to get AppImage info:"
            "$APPIMAGE_NAME" --appimage-help 2>&1 || true
        fi

        # Show file info
        ls -lh "$APPIMAGE_NAME"
        file "$APPIMAGE_NAME"
    else
        print_error "AppImage creation failed - file not found"
        print_error "Expected: $APPIMAGE_NAME or Simsapa-x86_64.AppImage"
        exit 1
    fi
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --app-name)
                APP_NAME="$2"
                shift 2
                ;;
            --app-version)
                APP_VERSION="$2"
                shift 2
                ;;
            --arch)
                ARCH="$2"
                shift 2
                ;;
            --os-suffix)
                OS_SUFFIX="$2"
                shift 2
                ;;
            --clean)
                print_status "Clean build requested"
                rm -rf "$BUILD_DIR" "$APPDIR_NAME" appimage-tools simsapa-*.AppImage
                shift
                ;;
            --force-download)
                print_status "Forcing fresh tool downloads"
                rm -rf appimage-tools/
                shift
                ;;
            --help|-h)
                show_usage
                exit 0
                ;;
            *)
                print_error "Unknown option: $1"
                echo "Use --help for usage information"
                exit 1
                ;;
        esac
    done

    # Set defaults if not provided
    if [ -z "$APP_NAME" ]; then
        APP_NAME=$(get_app_name_from_desktop)
    fi
    if [ -z "$APP_VERSION" ]; then
        APP_VERSION=$(get_version_from_cargo_toml)
    fi

    # Set file names based on arguments
    APPDIR_NAME="${APP_NAME}.AppDir"
    if [ -n "$OS_SUFFIX" ]; then
        APPIMAGE_NAME="${APP_NAME}-${APP_VERSION}-${ARCH}${OS_SUFFIX}.AppImage"
    else
        APPIMAGE_NAME="${APP_NAME}-${APP_VERSION}-${ARCH}.AppImage"
    fi

    print_status "Building AppImage with:"
    print_status "  App Name: $APP_NAME"
    print_status "  App Version: $APP_VERSION"
    print_status "  Architecture: $ARCH"
    print_status "  OS Suffix: $OS_SUFFIX"
    print_status "  Target file: $APPIMAGE_NAME"
}

# Main execution
main() {
    parse_args "$@"

    print_status "Starting AppImage build for Simsapa..."

    check_dependencies
    download_tools
    build_app
    create_appdir
    create_appimage

    print_status "AppImage build completed successfully!"
    print_status "You can now run: ./$APPIMAGE_NAME"
    print_status ""
    print_status "Note: If you encounter QtWebEngine errors, ensure your system has:"
    print_status "- A properly configured Qt6 WebEngine installation"
    print_status "- Required system libraries (libxss1, libnss3, etc.)"
}

# Run main function
main "$@"
