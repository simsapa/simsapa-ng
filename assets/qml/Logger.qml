import QtQuick

QtObject {
    id: logger

    enum Level {
        Silent = 0,
        Error = 1,
        Warn = 2,
        Info = 3,
        Debug = 4
    }

    property int level: Logger.Level.Error

    function debug(message) {
        if (level >= Logger.Level.Debug) {
            console.log("[DEBUG]", message)
        }
    }

    function info(message) {
        if (level >= Logger.Level.Info) {
            console.log("[INFO]", message)
        }
    }

    function warn(message) {
        if (level >= Logger.Level.Warn) {
            console.warn("[WARN]", message)
        }
    }

    function error(message) {
        if (level >= Logger.Level.Error) {
            console.error("[ERROR]", message)
        }
    }
}
