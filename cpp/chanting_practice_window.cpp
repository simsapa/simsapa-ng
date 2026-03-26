#include "chanting_practice_window.h"

#include <QUrl>

ChantingPracticeWindow::ChantingPracticeWindow(QApplication* app, QObject* parent)
    : QObject(parent)
{
    this->m_app = app;
    setup_qml();
}

void ChantingPracticeWindow::setup_qml() {
    const QUrl view_qml(QStringLiteral("qrc:/qt/qml/com/profoundlabs/simsapa/assets/qml/ChantingPracticeWindow.qml"));
    m_engine = new QQmlApplicationEngine(view_qml, this);
    m_root = m_engine->rootObjects().constFirst();
}

ChantingPracticeWindow::~ChantingPracticeWindow() {
    delete m_engine;
}
