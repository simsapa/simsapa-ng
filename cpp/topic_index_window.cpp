#include "topic_index_window.h"

#include <QUrl>

TopicIndexWindow::TopicIndexWindow(QApplication* app, QObject* parent)
    : QObject(parent)
{
    this->m_app = app;
    setup_qml();
}

void TopicIndexWindow::setup_qml() {
    const QUrl view_qml(QStringLiteral("qrc:/qt/qml/com/profoundlabs/simsapa/assets/qml/TopicIndexWindow.qml"));
    m_engine = new QQmlApplicationEngine(view_qml, this);
    m_root = m_engine->rootObjects().constFirst();
}

TopicIndexWindow::~TopicIndexWindow() {
    delete m_engine;
}
