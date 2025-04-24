import QtQuick

Loader {
    id: loader
    required property string sutta_uid

    /* signal loadingChanged(var loadRequest) */

    source: {
        if (Qt.platform.os === "android" || Qt.platform.os === "ios") {
            return "SuttaHtmlView_Mobile.qml";
        } else {
            return "SuttaHtmlView_Desktop.qml";
        }
    }

    onLoaded: {
        loader.item.sutta_uid = Qt.binding(() => sutta_uid);
        /* loader.item.loadingChanged.connect(loader.loadingChanged); */
    }

}
