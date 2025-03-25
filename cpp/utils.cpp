#include <QDir>
#include <QFile>
#include <QString>
#include <QSysInfo>
#include <QStandardPaths>
#include "utils.h"

QString get_internal_storage_path() {
    QString path = QStandardPaths::writableLocation(QStandardPaths::AppDataLocation);
    return path;
}

QString get_app_assets_path() {
    QString os(QSysInfo::productType());
    if (os == "android") {
        QString path = get_internal_storage_path() + "/app_assets";
        return path;
    } else {
        QString path = "./assets";
        return path;
    }
}

QString copy_file(QString source_file, QString destination_file) {
    QFileInfo fileInfo(source_file);
    if (fileInfo.isDir()) {
        return QString("Error: Is a directory: " + source_file);
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
