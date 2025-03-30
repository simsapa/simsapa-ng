#include "sutta_search_window.h"

#include <QUrl>

SuttaSearchWindow::SuttaSearchWindow(QApplication* app, QObject* parent)
    : QObject(parent)
{
    this->m_app = app;
    setup_qml();
}

void SuttaSearchWindow::setup_qml() {
    const QUrl view_qml(QStringLiteral("qrc:/qt/qml/com/profound_labs/simsapa/qml/sutta_search_window.qml"));
    m_engine = new QQmlApplicationEngine(view_qml, this);
    m_root = m_engine->rootObjects().constFirst();
}

SuttaSearchWindow::~SuttaSearchWindow() {
    delete m_engine;
}

