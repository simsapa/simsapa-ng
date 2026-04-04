#include "android_helpers.h"

#ifdef Q_OS_ANDROID
#include <QJniObject>
#include <QJniEnvironment>
#endif

extern "C" void log_info_c(const char* msg);
extern "C" void log_error_c(const char* msg);

void open_android_display_settings() {
    log_info_c("open_android_display_settings()");
#ifdef Q_OS_ANDROID
    QJniEnvironment env;

    QJniObject activity = QJniObject::callStaticObjectMethod(
        "org/qtproject/qt/android/QtNative",
        "activity",
        "()Landroid/app/Activity;"
    );

    if (!activity.isValid()) {
        log_error_c("Failed to get activity for opening settings");
        return;
    }

    QJniObject action = QJniObject::fromString("android.settings.DISPLAY_SETTINGS");

    QJniObject intent("android/content/Intent",
                      "(Ljava/lang/String;)V",
                      action.object<jstring>());

    if (!intent.isValid()) {
        log_error_c("Failed to create Intent for display settings");
        return;
    }

    activity.callMethod<void>("startActivity",
                              "(Landroid/content/Intent;)V",
                              intent.object());

    if (env.checkAndClearExceptions()) {
        log_error_c("JNI exception occurred while opening display settings");
    } else {
        log_info_c("Display settings opened successfully");
    }
#else
    log_info_c("open_android_display_settings() - not on Android platform");
#endif
}
