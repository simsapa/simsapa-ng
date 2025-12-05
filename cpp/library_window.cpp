#include "library_window.h"

#include <QUrl>

LibraryWindow::LibraryWindow(QApplication* app, QObject* parent)
    : QObject(parent)
{
    this->m_app = app;
    setup_qml();
}

void LibraryWindow::setup_qml() {
    const QUrl view_qml(QStringLiteral("qrc:/qt/qml/com/profoundlabs/simsapa/assets/qml/LibraryWindow.qml"));
    m_engine = new QQmlApplicationEngine(view_qml, this);
    m_root = m_engine->rootObjects().constFirst();
}

LibraryWindow::~LibraryWindow() {
    delete m_engine;
}
