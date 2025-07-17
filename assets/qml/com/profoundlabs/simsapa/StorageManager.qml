import QtQuick

Item {
    function get_storage_locations_json(): string {
        console.log("get_storage_locations_json()");
    }

    function save_storage_path(path: string, is_internal: bool) {
        console.log("save_storage_path(): " + path + ", is_internal: " + is_internal);
    }
}
