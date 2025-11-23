import QtQuick

Item {
    function download_urls_and_extract(urls: list<string>) {
        console.log("download_urls_and_extract():")
        for (let i=0; i < urls.length; i++) {
            console.log(i);
        }
    }

    function get_available_languages(): list<string> {
        console.log("get_available_languages()");
        return [];
    }

    function get_init_languages(): string {
        console.log("get_init_languages()");
        return "";
    }

    function acquire_wake_lock_rust() {
        console.log("acquire_wake_lock_rust()");
    }

    function release_wake_lock_rust() {
        console.log("release_wake_lock_rust()");
    }

    signal downloadProgressChanged(op_msg: string, downloaded_bytes: int, total_bytes: int);
    signal downloadShowMsg(message: string);
    signal downloadsCompleted(message: string);
}
