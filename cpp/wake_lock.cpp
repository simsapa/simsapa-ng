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
    
    log_info_c("Getting Android activity for wake lock");
    QJniObject activity = QJniObject::callStaticObjectMethod(
        "org/qtproject/qt/android/QtNative",
        "activity",
        "()Landroid/app/Activity;"
    );
    
    if (!activity.isValid()) {
        log_error_c("Failed to get activity for wake lock");
        return;
    }
    log_info_c("Activity obtained successfully");
    
    log_info_c("Getting PowerManager service");
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
    log_info_c("PowerManager obtained successfully");
    
    log_info_c("Creating wake lock");
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
        log_info_c("Wake lock object created, acquiring...");
        wakeLock.callMethod<void>("acquire", "()V");
        log_info_c("Wake lock acquired successfully");
    } else {
        log_error_c("Failed to create wake lock");
    }
    
    if (env.checkAndClearExceptions()) {
        log_error_c("JNI exception occurred while acquiring wake lock");
    }
#else
    log_info_c("acquire_wake_lock() - not on Android platform");
#endif
}

void release_wake_lock() {
    log_info_c("release_wake_lock()");
#ifdef Q_OS_ANDROID
    QJniEnvironment env;
    
    if (wakeLock.isValid()) {
        log_info_c("Wake lock is valid, checking if held");
        jboolean isHeld = wakeLock.callMethod<jboolean>("isHeld", "()Z");
        
        if (isHeld) {
            log_info_c("Wake lock is held, releasing...");
            wakeLock.callMethod<void>("release", "()V");
            log_info_c("Wake lock released successfully");
        } else {
            log_info_c("Wake lock was not held");
        }
        
        wakeLock = QJniObject();
    } else {
        log_info_c("Wake lock was not valid (already released or never acquired)");
    }
    
    if (env.checkAndClearExceptions()) {
        log_error_c("JNI exception occurred while releasing wake lock");
    }
#else
    log_info_c("release_wake_lock() - not on Android platform");
#endif
}
