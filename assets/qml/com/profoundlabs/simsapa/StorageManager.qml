import QtQuick

Item {
    function get_app_data_storage_paths_json(): string {
        console.log("get_app_data_storage_paths_json()");
        return "[{}]";
    }

    function save_storage_path(path: string, is_internal: bool) {
        console.log("save_storage_path(): " + path + ", is_internal: " + is_internal);
    }
}
