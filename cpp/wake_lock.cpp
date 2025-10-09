#include "wake_lock.h"

#ifdef Q_OS_ANDROID
#include <QJniObject>
#include <QJniEnvironment>

static QJniObject wakeLock;
#endif

extern "C" void log_info_c(const char* msg);
extern "C" void log_error_c(const char* msg);

void acquire_wake_lock() {
    log_info_c("acquire_wake_lock()");
#ifdef Q_OS_ANDROID
    QJniEnvironment env;
    
    QJniObject activity = QJniObject::callStaticObjectMethod(
        "org/qtproject/qt/android/QtNative",
        "activity",
        "()Landroid/app/Activity;"
    );
    
    if (!activity.isValid()) {
        log_error_c("Failed to get activity for wake lock");
        return;
    }
    
    QJniObject serviceName = QJniObject::getStaticObjectField(
        "android/content/Context",
        "POWER_SERVICE",
        "Ljava/lang/String;"
    );
    
    QJniObject powerManager = activity.callObjectMethod(
        "getSystemService",
        "(Ljava/lang/String;)Ljava/lang/Object;",
        serviceName.object()
    );
    
    if (!powerManager.isValid()) {
        log_error_c("Failed to get PowerManager");
        return;
    }
    
    jint wakeLockFlag = QJniObject::getStaticField<jint>(
        "android/os/PowerManager",
        "PARTIAL_WAKE_LOCK"
    );
    
    QJniObject tag = QJniObject::fromString("SimsapaDownloadWakeLock");
    
    wakeLock = powerManager.callObjectMethod(
        "newWakeLock",
        "(ILjava/lang/String;)Landroid/os/PowerManager$WakeLock;",
        wakeLockFlag,
        tag.object<jstring>()
    );
    
    if (wakeLock.isValid()) {
        wakeLock.callMethod<void>("acquire", "()V");
        log_info_c("Wake lock acquired");
    } else {
        log_error_c("Failed to create wake lock");
    }
    
    if (env.checkAndClearExceptions()) {
        log_error_c("JNI exception occurred while acquiring wake lock");
    }
#endif
}

void release_wake_lock() {
    log_info_c("release_wake_lock()");
#ifdef Q_OS_ANDROID
    QJniEnvironment env;
    
    if (wakeLock.isValid()) {
        jboolean isHeld = wakeLock.callMethod<jboolean>("isHeld", "()Z");
        
        if (isHeld) {
            wakeLock.callMethod<void>("release", "()V");
            log_info_c("Wake lock released");
        }
        
        wakeLock = QJniObject();
    }
    
    if (env.checkAndClearExceptions()) {
        log_error_c("JNI exception occurred while releasing wake lock");
    }
#endif
}
