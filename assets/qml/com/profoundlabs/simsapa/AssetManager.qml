import QtQuick

Item {
    Logger { id: logger }

    function download_urls_and_extract(urls: list<string>) {
        logger.log("download_urls_and_extract():")
        for (let i=0; i < urls.length; i++) {
            logger.log(i);
        }
    }

    signal downloadProgressChanged(op_msg: string, downloaded_bytes: int, total_bytes: int);
    signal downloadShowMsg(message: string);
    signal downloadsCompleted(message: string);
}
