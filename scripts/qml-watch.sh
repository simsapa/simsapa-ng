#!/usr/bin/env bash

if ! command -v inotifywait &> /dev/null; then
    echo "Error: inotifywait is missing."
    exit 1
fi

if ! command -v qml &> /dev/null; then
    echo "Error: qml is missing."
    exit 1
fi

if ! command -v xdotool &> /dev/null; then
    echo "Error: xdotool is missing."
    exit 1
fi

if [ "$#" -ne 1 ]; then
    echo "Usage: $0 path/to/file.qml"
    exit 2
fi

FILE="$1"

if [ ! -f "$FILE" ]; then
    echo "Error: File '$FILE' not found."
    exit 2
fi

QML_DIR=$(dirname "$FILE")

start_qml() {
    QML_IMPORT_PATH="$QML_DIR" qml "$FILE" &
    local pid=$!
    # The xdotool search --sync query will block until the new window is created.
    xdotool search --sync --pid $pid > /dev/null 2>&1
    echo $pid
}

read cur_pid < <(start_qml)

while true; do
    inotifywait -qq -e close_write "$FILE" "$QML_DIR"/*.qml

    # First, open a new window, which qtile will position where the previous one is.
    read new_pid < <(start_qml)

    # Then, kill the old window.
    kill "$cur_pid"
    wait "$cur_pid" 2>/dev/null

    cur_pid=$new_pid
done
