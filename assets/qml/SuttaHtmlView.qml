import QtQuick

Loader {
    id: loader
    property url url
    /* signal loadingChanged(var loadRequest) */

    source: {
        if (Qt.platform.os === "android" || Qt.platform.os === "ios") {
            return "SuttaHtmlView_Mobile.qml";
        } else {
            return "SuttaHtmlView_Desktop.qml";
        }
    }

    onLoaded: {
        loader.item.url = Qt.binding(() => url);
        /* loader.item.loadingChanged.connect(loader.loadingChanged); */
    }
}
