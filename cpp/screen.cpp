#include "screen.h"

#ifdef Q_OS_ANDROID
#include <QJniObject>
#include <QtCore/private/qandroidextras_p.h>

// android.view.WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON
static constexpr jint FLAG_KEEP_SCREEN_ON = 0x00000080;
#endif

extern "C" void log_info_c(const char* msg);
extern "C" void log_error_c(const char* msg);

void keep_screen_on(bool on) {
    if (on) {
        log_info_c("keep_screen_on(true)");
    } else {
        log_info_c("keep_screen_on(false)");
    }
#ifdef Q_OS_ANDROID
    // Window-flag changes must happen on the Android UI (main) thread.
    QtAndroidPrivate::runOnAndroidMainThread([on] {
        QJniObject activity = QNativeInterface::QAndroidApplication::context();
        if (!activity.isValid()) {
            log_error_c("keep_screen_on: failed to get Android activity");
            return;
        }

        QJniObject window = activity.callObjectMethod(
            "getWindow", "()Landroid/view/Window;");
        if (!window.isValid()) {
            log_error_c("keep_screen_on: failed to get activity window");
            return;
        }

        if (on) {
            window.callMethod<void>("addFlags", "(I)V", FLAG_KEEP_SCREEN_ON);
            log_info_c("keep_screen_on: FLAG_KEEP_SCREEN_ON added");
        } else {
            window.callMethod<void>("clearFlags", "(I)V", FLAG_KEEP_SCREEN_ON);
            log_info_c("keep_screen_on: FLAG_KEEP_SCREEN_ON cleared");
        }
    });
#else
    log_info_c("keep_screen_on() - not on Android platform, no-op");
#endif
}
