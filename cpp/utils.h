#ifndef UTILS_H
#define UTILS_H

#include <QString>

extern "C++" {
    QString get_internal_storage_path();
    QString get_app_assets_path();
}

QString copy_apk_assets_to_internal_storage(QString apk_asset_path = QString());
QStringList list_qrc_assets();
QString copy_qrc_app_assets_to_internal_storage();

#endif
