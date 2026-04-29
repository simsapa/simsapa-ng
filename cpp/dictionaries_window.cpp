#include "dictionaries_window.h"

#include <QUrl>

DictionariesWindow::DictionariesWindow(QApplication* app, QObject* parent)
    : QObject(parent)
{
    this->m_app = app;
    setup_qml();
}

void DictionariesWindow::setup_qml() {
    const QUrl view_qml(QStringLiteral("qrc:/qt/qml/com/profoundlabs/simsapa/assets/qml/DictionariesWindow.qml"));
    m_engine = new QQmlApplicationEngine(view_qml, this);
    m_root = m_engine->rootObjects().constFirst();
}

DictionariesWindow::~DictionariesWindow() {
    delete m_engine;
}
