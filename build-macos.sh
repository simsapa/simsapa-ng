#!/bin/bash

set -e

# macOS build script for Simsapa
# This script creates a macOS .app bundle and .dmg file from the built Qt6 application

# Default values - can be overridden by command line arguments
APP_NAME=""
APP_VERSION=""
BUNDLE_ID="com.profoundlabs.simsapa"
BUILD_DIR="./build/simsapadhammareader"
ARCH="$(uname -m)"  # arm64 or x86_64

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
    echo "  --bundle-id ID       Set bundle identifier (default: com.profoundlabs.simsapa)"
    echo "  --clean              Clean build artifacts before building"
    echo "  --skip-dmg           Skip DMG creation, only create .app bundle"
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

# Check if running on macOS
check_platform() {
    if [[ "$OSTYPE" != "darwin"* ]]; then
        print_error "This script must be run on macOS"
        exit 1
    fi
}

# Check if required tools are installed
check_dependencies() {
    print_status "Checking dependencies..."

    local missing_deps=()

    if ! command -v macdeployqt &> /dev/null; then
        print_warning "macdeployqt not found in PATH"
        print_warning "Will try to find it in Qt installation directory"
    fi

    if ! command -v cmake &> /dev/null; then
        missing_deps+=("cmake")
    fi

    if [ ${#missing_deps[@]} -ne 0 ]; then
        print_error "Missing dependencies: ${missing_deps[*]}"
        print_error "Please install: brew install ${missing_deps[*]}"
        exit 1
    fi
}

# Find macdeployqt tool
find_macdeployqt() {
    local macdeployqt_path=""

    # Check if already in PATH
    if command -v macdeployqt &> /dev/null; then
        macdeployqt_path="$(which macdeployqt)"
    # Try to find in standard Qt installation locations
    elif [ -f "$HOME/Qt/6.8.3/macos/bin/macdeployqt" ]; then
        macdeployqt_path="$HOME/Qt/6.8.3/macos/bin/macdeployqt"
    elif [ -f "/opt/Qt/6.8.3/macos/bin/macdeployqt" ]; then
        macdeployqt_path="/opt/Qt/6.8.3/macos/bin/macdeployqt"
    elif [ -f "/usr/local/Qt/6.8.3/macos/bin/macdeployqt" ]; then
        macdeployqt_path="/usr/local/Qt/6.8.3/macos/bin/macdeployqt"
    else
        print_error "macdeployqt not found. Please ensure Qt 6.8.3 is installed."
        exit 1
    fi

    echo "$macdeployqt_path"
}

# Find create-dmg tool or install it
find_or_install_create_dmg() {
    if command -v create-dmg &> /dev/null; then
        print_status "create-dmg found: $(which create-dmg)"
        return 0
    fi

    print_warning "create-dmg not found"
    
    if command -v brew &> /dev/null; then
        print_status "Installing create-dmg via Homebrew..."
        brew install create-dmg
    else
        print_error "create-dmg not found and Homebrew is not available"
        print_error "Please install Homebrew or create-dmg manually:"
        print_error "  brew install create-dmg"
        print_error "  OR download from: https://github.com/create-dmg/create-dmg"
        exit 1
    fi
}

# Build the application first
build_app() {
    print_status "Building application..."

    if [ ! -f "$BUILD_DIR/simsapadhammareader.app/Contents/MacOS/simsapadhammareader" ]; then
        print_status "Building simsapa..."
        cmake -S . -B "$BUILD_DIR" && cmake --build "$BUILD_DIR"
    else
        print_warning "Application already built. Use 'make build -B' to rebuild."
    fi

    if [ ! -f "$BUILD_DIR/simsapadhammareader.app/Contents/MacOS/simsapadhammareader" ]; then
        print_error "Build failed - executable not found"
        exit 1
    fi
}

# Update Info.plist with proper metadata
update_info_plist() {
    local app_bundle="$1"
    local info_plist="$app_bundle/Contents/Info.plist"

    print_status "Updating Info.plist..."

    if [ ! -f "$info_plist" ]; then
        print_error "Info.plist not found at: $info_plist"
        exit 1
    fi

    # Update version strings
    /usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString ${APP_VERSION#v}" "$info_plist" 2>/dev/null || \
        /usr/libexec/PlistBuddy -c "Add :CFBundleShortVersionString string ${APP_VERSION#v}" "$info_plist"
    
    /usr/libexec/PlistBuddy -c "Set :CFBundleVersion ${APP_VERSION#v}" "$info_plist" 2>/dev/null || \
        /usr/libexec/PlistBuddy -c "Add :CFBundleVersion string ${APP_VERSION#v}" "$info_plist"

    # Update bundle identifier
    /usr/libexec/PlistBuddy -c "Set :CFBundleIdentifier $BUNDLE_ID" "$info_plist" 2>/dev/null || \
        /usr/libexec/PlistBuddy -c "Add :CFBundleIdentifier string $BUNDLE_ID" "$info_plist"

    # Update display name
    /usr/libexec/PlistBuddy -c "Set :CFBundleDisplayName $APP_NAME" "$info_plist" 2>/dev/null || \
        /usr/libexec/PlistBuddy -c "Add :CFBundleDisplayName string $APP_NAME" "$info_plist"

    # Set minimum macOS version
    /usr/libexec/PlistBuddy -c "Set :LSMinimumSystemVersion 11.0" "$info_plist" 2>/dev/null || \
        /usr/libexec/PlistBuddy -c "Add :LSMinimumSystemVersion string 11.0" "$info_plist"

    # Set category
    /usr/libexec/PlistBuddy -c "Set :LSApplicationCategoryType public.app-category.education" "$info_plist" 2>/dev/null || \
        /usr/libexec/PlistBuddy -c "Add :LSApplicationCategoryType string public.app-category.education" "$info_plist"

    # Set high resolution capable
    /usr/libexec/PlistBuddy -c "Set :NSHighResolutionCapable true" "$info_plist" 2>/dev/null || \
        /usr/libexec/PlistBuddy -c "Add :NSHighResolutionCapable bool true" "$info_plist"

    print_status "Info.plist updated successfully"
}

# Deploy Qt frameworks and dependencies
deploy_qt_frameworks() {
    local app_bundle="$1"
    local macdeployqt="$2"

    print_status "Deploying Qt frameworks with macdeployqt..."

    # Run macdeployqt
    if ! "$macdeployqt" "$app_bundle" -qmldir=./assets/qml -verbose=1; then
        print_error "macdeployqt failed"
        exit 1
    fi

    print_status "Qt frameworks deployed successfully"
}

# Create the final .app bundle in a distribution folder
create_app_bundle() {
    print_status "Creating distribution .app bundle..."

    local macdeployqt=$(find_macdeployqt)
    print_status "Using macdeployqt: $macdeployqt"

    local source_app="$BUILD_DIR/simsapadhammareader.app"
    local dist_dir="./dist"
    local dist_app="$dist_dir/$APP_NAME.app"

    # Remove existing distribution directory
    rm -rf "$dist_dir"
    mkdir -p "$dist_dir"

    # Copy the app bundle
    print_status "Copying app bundle to distribution folder..."
    cp -R "$source_app" "$dist_app"

    # Update Info.plist
    update_info_plist "$dist_app"

    # Deploy Qt frameworks
    deploy_qt_frameworks "$dist_app" "$macdeployqt"

    # Verify the bundle
    print_status "Verifying app bundle..."
    if [ -f "$dist_app/Contents/MacOS/simsapadhammareader" ]; then
        print_status "✓ App bundle created successfully: $dist_app"
        ls -lh "$dist_app/Contents/MacOS/simsapadhammareader"
    else
        print_error "✗ App bundle verification failed"
        exit 1
    fi
}

# Create DMG file
create_dmg() {
    print_status "Creating DMG file..."

    find_or_install_create_dmg

    local dist_dir="./dist"
    local dmg_name="${APP_NAME}-${APP_VERSION}-${ARCH}.dmg"
    local app_bundle="$dist_dir/$APP_NAME.app"

    # Remove existing DMG
    rm -f "$dmg_name"

    # Create a temporary directory for DMG contents
    local dmg_temp="$dist_dir/dmg-temp"
    rm -rf "$dmg_temp"
    mkdir -p "$dmg_temp"

    # Copy app bundle to temp directory
    cp -R "$app_bundle" "$dmg_temp/"

    # Create symbolic link to Applications folder
    ln -s /Applications "$dmg_temp/Applications"

    # Try to create DMG with create-dmg (with nice styling)
    print_status "Creating styled DMG..."
    
    # Check if background image exists
    local background_image=""
    if [ -f "assets/icons/appicons/dmg-background.png" ]; then
        background_image="assets/icons/appicons/dmg-background.png"
    fi

    # Check if icon exists
    local icon_file=""
    if [ -f "assets/icons/appicons/simsapa.icns" ]; then
        icon_file="assets/icons/appicons/simsapa.icns"
    fi

    # Build create-dmg command
    local create_dmg_cmd=(
        create-dmg
        --volname "$APP_NAME"
        --window-pos 200 120
        --window-size 800 400
        --icon-size 100
        --icon "$APP_NAME.app" 200 190
        --hide-extension "$APP_NAME.app"
        --app-drop-link 600 185
    )

    # Add optional parameters if files exist
    if [ -n "$icon_file" ]; then
        create_dmg_cmd+=(--volicon "$icon_file")
    fi

    if [ -n "$background_image" ]; then
        create_dmg_cmd+=(--background "$background_image")
    fi

    # Add output DMG and source folder
    create_dmg_cmd+=("$dmg_name" "$dmg_temp")

    # Run create-dmg
    if "${create_dmg_cmd[@]}" 2>&1 | grep -v "Device not configured"; then
        print_status "DMG created successfully: $dmg_name"
    else
        print_warning "create-dmg had some warnings, but DMG may still be created"
    fi

    # Clean up temp directory
    rm -rf "$dmg_temp"

    # Verify DMG was created
    if [ -f "$dmg_name" ]; then
        print_status "✓ DMG file created successfully"
        ls -lh "$dmg_name"
        
        # Test if DMG is valid
        print_status "Verifying DMG integrity..."
        if hdiutil verify "$dmg_name" > /dev/null 2>&1; then
            print_status "✓ DMG integrity verified"
        else
            print_warning "DMG integrity check returned warnings (this may be normal)"
        fi
    else
        print_error "✗ DMG file creation failed"
        exit 1
    fi
}

# Code signing (optional - requires Apple Developer account)
code_sign() {
    local app_bundle="$1"
    
    # Check if signing identity is available
    if [ -n "$APPLE_SIGNING_IDENTITY" ]; then
        print_status "Code signing with identity: $APPLE_SIGNING_IDENTITY"
        
        codesign --deep --force --verify --verbose \
            --sign "$APPLE_SIGNING_IDENTITY" \
            --options runtime \
            --entitlements entitlements.plist \
            "$app_bundle"
        
        print_status "Code signing completed"
    else
        print_warning "Skipping code signing (set APPLE_SIGNING_IDENTITY to enable)"
        print_warning "App will not be notarized and may show security warnings"
    fi
}

# Parse command line arguments
SKIP_DMG=0

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
            --bundle-id)
                BUNDLE_ID="$2"
                shift 2
                ;;
            --clean)
                print_status "Clean build requested"
                rm -rf "$BUILD_DIR" ./dist ./*.dmg
                shift
                ;;
            --skip-dmg)
                SKIP_DMG=1
                print_status "DMG creation will be skipped"
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

    print_status "Building macOS package with:"
    print_status "  App Name: $APP_NAME"
    print_status "  App Version: $APP_VERSION"
    print_status "  Bundle ID: $BUNDLE_ID"
    print_status "  Architecture: $ARCH"
}

# Main execution
main() {
    parse_args "$@"

    print_status "Starting macOS build for Simsapa..."

    check_platform
    check_dependencies
    build_app
    create_app_bundle

    if [ $SKIP_DMG -eq 0 ]; then
        create_dmg
        print_status "macOS build completed successfully!"
        print_status "Distribution files created:"
        print_status "  - ./dist/$APP_NAME.app"
        print_status "  - ./${APP_NAME}-${APP_VERSION}-${ARCH}.dmg"
    else
        print_status "App bundle created successfully!"
        print_status "Distribution file created:"
        print_status "  - ./dist/$APP_NAME.app"
    fi

    print_status ""
    print_status "Note: If not code signed, users may need to:"
    print_status "  1. Right-click the app and select 'Open'"
    print_status "  2. Or go to System Preferences > Security & Privacy to allow it"
}

# Run main function
main "$@"
