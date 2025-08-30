import QtQuick

Item {
    Logger { id: logger }

    function get_app_data_storage_paths_json(): string {
        logger.log("get_app_data_storage_paths_json()");
        return "[{}]";
    }

    function save_storage_path(path: string, is_internal: bool) {
        logger.log("save_storage_path(): " + path + ", is_internal: " + is_internal);
    }
}
