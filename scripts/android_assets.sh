#!/usr/bin/env bash

# This script runs in the generated build folder, e.g. build/Android_Qt_6_8_2_Clang_x86_64-Debug/

set -e

pwd

PROJECT_NAME="$1"
PROJECT_SOURCE_DIR="$2"
ANDROID_BUILD_DIR="./android-build-$PROJECT_NAME"
ASSETS_DIR="$ANDROID_BUILD_DIR/assets"

if [ ! -d "$ASSETS_DIR" ]; then mkdir -p "$ASSETS_DIR"; fi

cp "$PROJECT_SOURCE_DIR/appdata.sqlite3" "$ASSETS_DIR"
cp -r "$PROJECT_SOURCE_DIR"/assets/* "$ASSETS_DIR"
