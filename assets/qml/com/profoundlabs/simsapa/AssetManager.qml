import QtQuick

Item {
    function download_urls(urls: list<string>) {
        console.log("download_urls():")
        for (let i=0; i < urls.length; i++) {
            console.log(i);
        }
    }

    signal downloadProgressChanged(op_msg: string, downloaded_bytes: int, total_bytes: int);
    signal downloadFinished(message: string);
}
