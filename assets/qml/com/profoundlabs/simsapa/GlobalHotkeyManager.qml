import QtQuick

Item {
    function get_global_hotkeys_json(): string {
        return '{"enabled":false,"bindings":{"dictionary_lookup":"Ctrl+C+C"}}';
    }

    function get_default_global_hotkeys_json(): string {
        return '{"enabled":false,"bindings":{"dictionary_lookup":"Ctrl+C+C"}}';
    }

    function set_global_hotkey(action_id: string, sequence: string) {
    }

    function set_global_hotkeys_enabled(enabled: bool) {
    }

    function is_wayland(): bool {
        return false;
    }

    function get_api_url(): string {
        return "http://localhost:4848";
    }

    signal globalHotkeysChanged();

    signal globalDictionaryLookupRequested(query: string);
}
