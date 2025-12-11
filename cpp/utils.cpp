#include <QDir>
#include <QDirIterator>
#include <QFile>
#include <QString>
#include <QSysInfo>
#include <QStandardPaths>
#include <QStorageInfo>
#include <QJsonArray>
#include <QJsonObject>
#include <QJsonDocument>

#ifdef Q_OS_ANDROID
#include <QJniObject>
#include <QJniEnvironment>
#endif

#include "utils.h"

QString get_internal_storage_path() {
    QString path = QStandardPaths::writableLocation(QStandardPaths::AppDataLocation);
    return path;
}

QString get_app_assets_path() {
    QString path = get_internal_storage_path() + "/app-assets";
    return path;
}

int get_status_bar_height() {
#ifdef Q_OS_ANDROID
    // Get the status bar height from Android system resources
    QJniEnvironment env;
    QJniObject activity = QJniObject::callStaticObjectMethod(
        "org/qtproject/qt/android/QtNative",
        "activity",
        "()Landroid/app/Activity;"
    );

    if (activity.isValid()) {
        // Get the Resources object
        QJniObject resources = activity.callObjectMethod(
            "getResources",
            "()Landroid/content/res/Resources;"
        );

        if (resources.isValid()) {
            // Get resource ID for status_bar_height
            jint resourceId = resources.callMethod<jint>(
                "getIdentifier",
                "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I",
                QJniObject::fromString("status_bar_height").object<jstring>(),
                QJniObject::fromString("dimen").object<jstring>(),
                QJniObject::fromString("android").object<jstring>()
            );

            if (resourceId > 0) {
                // Get the actual dimension value in pixels
                jint heightInPixels = resources.callMethod<jint>(
                    "getDimensionPixelSize",
                    "(I)I",
                    resourceId
                );

                // Get display metrics to convert pixels to density-independent pixels
                QJniObject displayMetrics = resources.callObjectMethod(
                    "getDisplayMetrics",
                    "()Landroid/util/DisplayMetrics;"
                );

                if (displayMetrics.isValid()) {
                    jfloat density = displayMetrics.getField<jfloat>("density");

                    // Convert pixels to density-independent pixels (dp)
                    int heightInDp = static_cast<int>(heightInPixels / density);

                    // Clear any pending JNI exceptions
                    if (env.checkAndClearExceptions()) {
                        // Exception was cleared
                    }

                    return heightInDp;
                }
            }
        }
    }

    // Clear any pending JNI exceptions
    if (env.checkAndClearExceptions()) {
        // Exception was cleared
    }

    // Default fallback value for Android if we can't get the actual height
    return 24;
#else
    // On non-Android platforms, return 0 (no status bar offset needed)
    return 0;
#endif
}

// Helper function to create storage info JSON object
QJsonObject createStorageInfo(const QString& path, const QString& internalAppDataPath) {
    QJsonObject item;
    item["path"] = path;

    // Get storage info for the path
    QStorageInfo storage(path);

    // Set label
    QString label = storage.displayName().isEmpty() ?
        QDir(storage.rootPath()).dirName() :
        storage.displayName();
    item["label"] = label;

    // Check if internal
    item["is_internal"] = (path == internalAppDataPath);

    // Storage sizes in megabytes
    item["megabytes_total"] = static_cast<int>(storage.bytesTotal() / (1024 * 1024));
    item["megabytes_available"] = static_cast<int>(storage.bytesAvailable() / (1024 * 1024));

    return item;
}

QJsonArray get_app_data_storage_paths() {
    QJsonArray storageArray;

    // Get internal app data path (common for all platforms)
    QString internalAppDataPath = QStandardPaths::writableLocation(QStandardPaths::AppDataLocation);

    // Add internal storage path
    if (!internalAppDataPath.isEmpty()) {
        storageArray.append(createStorageInfo(internalAppDataPath, internalAppDataPath));
    }

#ifdef Q_OS_ANDROID
    // On Android, get external storage paths
    QJniEnvironment env;
    QJniObject activity = QJniObject::callStaticObjectMethod(
        "org/qtproject/qt/android/QtNative",
        "activity",
        "()Landroid/app/Activity;"
    );

    if (activity.isValid()) {
        // Call getExternalFilesDirs(null) to get all external storage paths
        QJniObject externalDirs = activity.callObjectMethod(
            "getExternalFilesDirs",
            "(Ljava/lang/String;)[Ljava/io/File;",
            nullptr
        );

        if (externalDirs.isValid()) {
            // Get the array length
            jsize length = env->GetArrayLength(externalDirs.object<jobjectArray>());

            for (int i = 0; i < length; ++i) {
                QJniObject fileObject = env->GetObjectArrayElement(
                    externalDirs.object<jobjectArray>(),
                    i
                );

                if (fileObject.isValid()) {
                    // Get the absolute path of the File object
                    QJniObject pathObject = fileObject.callObjectMethod(
                        "getAbsolutePath",
                        "()Ljava/lang/String;"
                    );

                    if (pathObject.isValid()) {
                        QString externalPath = pathObject.toString();

                        // Only add if it's different from internal path and not empty
                        if (!externalPath.isEmpty() && externalPath != internalAppDataPath) {
                            storageArray.append(createStorageInfo(externalPath, internalAppDataPath));
                        }
                    }
                }
            }
        }
    }

    // Clear any pending JNI exceptions
    if (env.checkAndClearExceptions()) {
        // Exception was cleared
    }
#endif

    return storageArray;
}

QString get_app_data_storage_paths_json() {
    QJsonArray storageArray = get_app_data_storage_paths();
    QJsonDocument doc(storageArray);
    return doc.toJson(QJsonDocument::Compact);
}

QString copy_file(QString source_file, QString destination_file) {
    QFileInfo fileInfo(source_file);
    if (fileInfo.isDir()) {
        return QString("Error: Is a directory: " + source_file);
    }

    QDir dest_dir = QFileInfo(destination_file).dir();
    if (!dest_dir.exists()) {
        if (!dest_dir.mkpath(".")) {
            QString ret_msg = QString("Failed to create directory for: " + destination_file);
            qWarning() << ret_msg;
            return ret_msg;
        }
    }

    QFile source(source_file);
    if (!source.copy(destination_file)) {
        QString ret_msg("Failed to copy file: " + source_file + ", error: " + source.errorString());
        qWarning() << ret_msg;
        return ret_msg;
    }

    QFile::setPermissions(destination_file,
        QFileDevice::ReadUser |
        QFileDevice::WriteUser |
        QFileDevice::ReadOwner |
        QFileDevice::WriteOwner);

    return QString("");
}

QString copy_apk_assets_to_internal_storage(QString apk_asset_path /* = QString("") */) {
    QString assets_storage = get_app_assets_path();
    QString ret_msg = QString("");

    QDir assets_storage_dir(assets_storage);
    if (!assets_storage_dir.exists()) {
        if (!assets_storage_dir.mkpath(".")) {
            ret_msg = QString("Failed to create directory: " + assets_storage);
            qWarning() << ret_msg;
            return ret_msg;
        }
    }

    // If no specific path provided, copy all assets
    if (apk_asset_path.isEmpty()) {
        apk_asset_path = "/";
    }

    // Create destination directory for the asset path
    QString dest_dir_path = assets_storage + apk_asset_path;
    QDir dest_dir(dest_dir_path);
    if (!dest_dir.exists()) {
        if (!dest_dir.mkpath(".")) {
            ret_msg = QString("Failed to create directory: " + dest_dir_path);
            qWarning() << ret_msg;
            return ret_msg;
        }
    }

    QDir apk_assets_dir("assets:" + apk_asset_path);
    if (apk_assets_dir.exists()) {
        // Copy directory contents recursively
        QStringList entries = apk_assets_dir.entryList(QDir::AllEntries | QDir::Hidden | QDir::System);

        foreach (const QString& entry, entries) {
            if (entry == "." || entry == "..") {
                continue;
            }

            QString source_path = "assets:" + apk_asset_path + "/" + entry;
            QString destination_file = dest_dir_path + "/" + entry;

            QFileInfo fileInfo(source_path);
            if (fileInfo.isDir()) {
                // Recursive directory copy
                QString r = copy_apk_assets_to_internal_storage(apk_asset_path + "/" + entry);
                if (!r.isEmpty()) {
                    qWarning() << r;
                    return r;
                }
            } else {
                copy_file(source_path, destination_file);
            }
        }

    } else {
        // Handle single file copy
        QString source_path = "assets:" + apk_asset_path;
        QString destination_file = dest_dir_path;

        copy_file(source_path, destination_file);
    }

    return ret_msg;
}

QStringList list_qrc_assets() {
    qWarning() << "list_qrc_assets()";
    QStringList resource_files;
    // QDirIterator it(":/app-assets", QStringList() << "*", QDir::AllEntries | QDir::NoDotAndDotDot, QDirIterator::Subdirectories);
    QDirIterator it(":",                                  QDir::AllEntries | QDir::NoDotAndDotDot, QDirIterator::Subdirectories);
    while (it.hasNext()) {
        QString i(it.next());
        resource_files.append(i);
        qWarning() << i;
    }
    qWarning() << resource_files.length();
    return resource_files;
}

QString copy_qrc_app_assets_to_internal_storage() {
    qWarning() << "copy_qrc_app_assets_to_internal_storage()";
    QString assets_storage = get_app_assets_path();
    QString ret_msg = QString("");

    QDir assets_storage_dir(assets_storage);
    if (!assets_storage_dir.exists()) {
        if (!assets_storage_dir.mkpath(".")) {
            ret_msg = QString("Failed to create directory: " + assets_storage);
            qWarning() << ret_msg;
            return ret_msg;
        }
    }

    QStringList resource_files;
    QDirIterator it(":/app-assets", QStringList() << "*", QDir::AllEntries | QDir::NoDotAndDotDot, QDirIterator::Subdirectories);
    while (it.hasNext()) {
        QString i(it.next());
        qWarning() << i;
        resource_files.append(i);
    }

    qWarning() << resource_files.length();

    foreach (const QString& source_path, resource_files) {
        // Remove ":/app-assets/" prefix
        QString relative_path = source_path.mid(12);
        QString destination_path = assets_storage + "/" + relative_path;

        qWarning() << "relative_path: " << relative_path;
        // qWarning() << "destination_path: " << destination_path;

        QFileInfo fileInfo(source_path);
        if (fileInfo.isDir()) {
            continue;
        }

        QString r = copy_file(source_path, destination_path);
        if (!r.isEmpty()) {
            qWarning() << r;
            return r;
        }
    }

    return ret_msg;
}
