#include "chanting_practice_window.h"

#include <QUrl>

ChantingPracticeWindow::ChantingPracticeWindow(QApplication* app, const QString& window_id, QObject* parent)
    : QObject(parent)
{
    this->m_app = app;
    this->m_window_id = window_id;
    setup_qml();
}

void ChantingPracticeWindow::setup_qml() {
    const QUrl view_qml(QStringLiteral("qrc:/qt/qml/com/profoundlabs/simsapa/assets/qml/ChantingPracticeWindow.qml"));
    m_engine = new QQmlApplicationEngine(view_qml, this);
    m_root = m_engine->rootObjects().constFirst();
    m_root->setProperty("window_id", m_window_id);
}

ChantingPracticeWindow::~ChantingPracticeWindow() {
    delete m_engine;
}
