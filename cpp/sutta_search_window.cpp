#include "sutta_search_window.h"

#include <QSysInfo>
#include <QUrl>

SuttaSearchWindow::SuttaSearchWindow(QApplication* app, QObject* parent)
    : QObject(parent)
{
    this->m_app = app;
    setup_qml();
}

void SuttaSearchWindow::setup_qml() {
    QString os(QSysInfo::productType());
    QUrl view_qml;
    if (os == "android" || os == "ios") {
        view_qml = QUrl(QStringLiteral("qrc:/qt/qml/com/profoundlabs/simsapa/qml/sutta_search_window_mobile.qml"));
    } else {
        view_qml = QUrl(QStringLiteral("qrc:/qt/qml/com/profoundlabs/simsapa/qml/sutta_search_window_desktop.qml"));
    }
    m_engine = new QQmlApplicationEngine(view_qml, this);
    m_root = m_engine->rootObjects().constFirst();
}

SuttaSearchWindow::~SuttaSearchWindow() {
    delete m_engine;
}

