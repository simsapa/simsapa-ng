#include <iostream>
#include <thread>
#include <QtGui/QGuiApplication>
#include <QtQml/QQmlApplicationEngine>
#include <QtQml/QQmlContext>
#include <QDir>
#include <QFile>
#include <QStandardPaths>
// #include <typeinfo>

extern "C" void start_webserver();
extern "C" void shutdown_webserver();

QString get_internal_storage_path() {
    QString path = QStandardPaths::writableLocation(QStandardPaths::AppDataLocation);
    return path;
}

QString copy_assets_to_internal_storage(QString assetPath = QString("")) {
    QString destinationPath = get_internal_storage_path();

    // Ensure the directory exists
    QDir dir(destinationPath);
    if (!dir.exists()) {
        if (!dir.mkpath(".")) {
            qWarning() << "Failed to create directory:" << destinationPath;
            return QString();
        }
    }

    // If no specific path provided, copy all assets
    if (assetPath.isEmpty()) {
        assetPath = "/";
    }

    // Remove trailing slash if present
    if (assetPath.endsWith('/')) {
        assetPath.chop(1);
    }

    // Create destination directory for the asset path
    QString destDir = destinationPath + assetPath;
    QDir destDirObj(destDir);
    if (!destDirObj.exists()) {
        if (!destDirObj.mkpath(".")) {
            qWarning() << "Failed to create directory:" << destDir;
            return QString();
        }
    }

    // Check if assetPath is a directory
    QDir assetsDir("assets:" + assetPath);
    if (assetsDir.exists()) {
        // Copy directory contents recursively
        QStringList entries = assetsDir.entryList(QDir::AllEntries | QDir::Hidden | QDir::System);

        foreach (const QString& entry, entries) {
            if (entry == "." || entry == "..") {
                continue;
            }

            QString sourcePath = "assets:" + assetPath + "/" + entry;
            QString destinationFile = destDir + "/" + entry;

            QFileInfo fileInfo(sourcePath);
            if (fileInfo.isDir()) {
                // Recursive directory copy
                QString result = copy_assets_to_internal_storage(assetPath + "/" + entry);
                if (result.isEmpty()) {
                    qWarning() << "Failed to copy directory:" << sourcePath;
                    return QString();
                }
            } else {
                // Copy single file
                QFile source(sourcePath);

                if (!source.copy(destinationFile)) {
                    qWarning() << "Failed to copy file:" << sourcePath << ", error:" << source.errorString();
                    return QString();
                }

                // Set proper permissions
                QFile::setPermissions(destinationFile,
                    QFileDevice::ReadUser |
                    QFileDevice::WriteUser |
                    QFileDevice::ReadOwner |
                    QFileDevice::WriteOwner);
            }
        }
    } else {
        // Handle single file copy
        QString sourcePath = "assets:" + assetPath;
        QString destinationFile = destDir;

        QFile source(sourcePath);
        if (!source.copy(destinationFile)) {
            qWarning() << "Failed to copy file:" << sourcePath << ", error:" << source.errorString();
            return QString();
        }

        // Set proper permissions
        QFile::setPermissions(destinationFile,
            QFileDevice::ReadUser |
            QFileDevice::WriteUser |
            QFileDevice::ReadOwner |
            QFileDevice::WriteOwner);
    }

    return destinationPath + assetPath;
}

int main(int argc, char* argv[]) {
  std::thread webserver_thread(start_webserver);

  QCoreApplication::setApplicationName("Simsapa Dhamma Reader");
  QCoreApplication::setOrganizationName("Profound Labs");

  QGuiApplication app(argc, argv);

  // Check if we copied assets already by testing the db file.
  QFile db_file(get_internal_storage_path() + "/appdata.sqlite3");
  if (db_file.exists()) {
    QString r = copy_assets_to_internal_storage();
    if (!r.isEmpty()) {
      qDebug() << "All assets copied to:" << r;
    } else {
      qDebug() << "Failed to copy assets";
    }
  }

  QQmlApplicationEngine engine;

  const QUrl sutta_search_window_qml(QStringLiteral("qrc:/qt/qml/com/profound_labs/simsapa/qml/sutta_search_window.qml"));
  QObject::connect(
    &engine,
    &QQmlApplicationEngine::objectCreated,
    &app,
    [sutta_search_window_qml](QObject* obj, const QUrl& objUrl) {
      if (!obj && sutta_search_window_qml == objUrl)
        QCoreApplication::exit(-1);
    },
    Qt::QueuedConnection);

  engine.load(sutta_search_window_qml);

  // QMetaObject::invokeMethod(engine.rootObjects().constFirst(),
  //                           "load_url",
  //                           Q_ARG(QVariant, QUrl("http://localhost:8484/index.html")));

  int app_ret = app.exec();

  shutdown_webserver();

  if (webserver_thread.joinable()) {
    webserver_thread.join();
  }

  std::cout << "Exiting with status " << app_ret << "." << std::endl;

  return app_ret;
}
