#ifndef ANDROID_HELPERS_H
#define ANDROID_HELPERS_H

#include <QString>

extern "C++" {
    void open_android_display_settings();
    // Returns "granted", "denied", or "undetermined"
    QString check_microphone_permission_impl();
    void request_microphone_permission_impl();
}

#endif
