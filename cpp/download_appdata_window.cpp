#include "download_appdata_window.h"

#include <QSysInfo>
#include <QUrl>

DownloadAppdataWindow::DownloadAppdataWindow(QApplication* app, QObject* parent)
    : QObject(parent)
{
    this->m_app = app;
    setup_qml();
}

void DownloadAppdataWindow::setup_qml() {
    QUrl view_qml;
    view_qml = QUrl(QStringLiteral("qrc:/qt/qml/com/profoundlabs/simsapa/assets/qml/DownloadAppdataWindow.qml"));
    m_engine = new QQmlApplicationEngine(view_qml, this);
    m_root = m_engine->rootObjects().constFirst();
}

DownloadAppdataWindow::~DownloadAppdataWindow() {
    delete m_engine;
}

