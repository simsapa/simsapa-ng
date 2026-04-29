import QtQuick

Item {
    function import_zip(zip_path: string, label: string, lang: string): string {
        console.log("import_zip():", zip_path, label, lang);
        return "ok";
    }

    function delete_dictionary(dictionary_id: int): string {
        console.log("delete_dictionary():", dictionary_id);
        return "ok";
    }

    function rename_label(dictionary_id: int, new_label: string): string {
        console.log("rename_label():", dictionary_id, new_label);
        return "ok";
    }

    function list_user_dictionaries(): string {
        return "[]";
    }

    function list_shipped_source_uids(): string {
        return "[]";
    }

    function dpd_source_uids(): string {
        return "[\"dpd\"]";
    }

    function commentary_definitions_source_uids(): string {
        return "[]";
    }

    function label_status(label: string): string {
        return "available";
    }

    function suggested_label_for_zip(zip_path: string): string {
        return "";
    }

    function is_known_tokenizer_lang(lang: string): bool {
        return true;
    }

    function get_user_dict_enabled(label: string): bool {
        return true;
    }

    function set_user_dict_enabled(label: string, enabled: bool) {
        console.log("set_user_dict_enabled():", label, enabled);
    }

    function get_user_dict_enabled_map(): string {
        return "{}";
    }

    function get_dpd_enabled(): bool {
        return true;
    }

    function set_dpd_enabled(enabled: bool) {
        console.log("set_dpd_enabled():", enabled);
    }

    function get_commentary_definitions_enabled(): bool {
        return true;
    }

    function set_commentary_definitions_enabled(enabled: bool) {
        console.log("set_commentary_definitions_enabled():", enabled);
    }

    function reconcile_needed(): bool {
        return false;
    }

    function start_reconcile() {
        console.log("start_reconcile()");
    }

    signal importProgress(stage: string, done: int, total: int);
    signal importFinished(dictionary_id: int, label: string);
    signal importFailed(message: string);
    signal reconcileProgress(stage: string, done: int, total: int);
    signal reconcileFinished();
}
