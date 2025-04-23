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
    QUrl view_qml;
    view_qml = QUrl(QStringLiteral("qrc:/qt/qml/com/profoundlabs/simsapa/assets/qml/SuttaSearchWindow.qml"));
    m_engine = new QQmlApplicationEngine(view_qml, this);
    m_root = m_engine->rootObjects().constFirst();
}

SuttaSearchWindow::~SuttaSearchWindow() {
    delete m_engine;
}

