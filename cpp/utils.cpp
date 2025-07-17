#include <vector>
#include <string>

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

#include "utils.h"

QString get_internal_storage_path() {
    QString path = QStandardPaths::writableLocation(QStandardPaths::AppDataLocation);
    return path;
}

QString get_app_assets_path() {
    QString path = get_internal_storage_path() + "/app-assets";
    return path;
}

QJsonArray get_storage_locations() {
    QJsonArray storageArray;

    // Get the standard app data location to identify internal storage
    QString appDataPath = QStandardPaths::writableLocation(QStandardPaths::AppDataLocation);
    QString internalRoot;

    // Find the root of the internal storage by comparing mount points
    QList<QStorageInfo> volumes = QStorageInfo::mountedVolumes();

    for (const QStorageInfo &storage : volumes) {
        if (storage.isValid() && storage.isReady()) {
            QString rootPath = storage.rootPath();
            if (appDataPath.startsWith(rootPath)) {
                if (rootPath.length() > internalRoot.length()) {
                    internalRoot = rootPath;
                }
            }
        }
    }

    // Process each storage volume
    for (const QStorageInfo &storage : volumes) {
        if (!storage.isValid() || !storage.isReady()) {
            continue;
        }

        QJsonObject item;
        QString rootPath = storage.rootPath();

        // Skip system-only mounts (like /proc, /sys on Linux)
        std::vector<std::string> restrictedPaths = {"/boot", "/dev", "/proc", "/run", "/sys", "/tmp", "/var"};

        if (std::any_of(restrictedPaths.begin(), restrictedPaths.end(),
                        [&rootPath](const std::string &path) {
                          return rootPath.startsWith(QString::fromStdString(path));
                        })) {
          continue;
        }

        item["path"] = rootPath;
        item["label"] = storage.displayName().isEmpty() ?
                       QDir(rootPath).dirName() : storage.displayName();
        item["is_internal"] = (rootPath == internalRoot);
        item["megabytes_total"] = static_cast<int>(storage.bytesTotal() / (1024 * 1024));
        item["megabytes_available"] = static_cast<int>(storage.bytesAvailable() / (1024 * 1024));

        storageArray.append(item);
    }

    return storageArray;
}

QString get_storage_locations_json() {
    QJsonArray storageArray = get_storage_locations();
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
