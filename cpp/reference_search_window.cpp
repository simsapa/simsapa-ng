#include "reference_search_window.h"

#include <QUrl>

ReferenceSearchWindow::ReferenceSearchWindow(QApplication* app, QObject* parent)
    : QObject(parent)
{
    this->m_app = app;
    setup_qml();
}

void ReferenceSearchWindow::setup_qml() {
    const QUrl view_qml(QStringLiteral("qrc:/qt/qml/com/profoundlabs/simsapa/assets/qml/ReferenceSearchWindow.qml"));
    m_engine = new QQmlApplicationEngine(view_qml, this);
    m_root = m_engine->rootObjects().constFirst();
}

ReferenceSearchWindow::~ReferenceSearchWindow() {
    delete m_engine;
}
