#include "chanting_review_window.h"

#include <QUrl>
#include <QQmlContext>

ChantingReviewWindow::ChantingReviewWindow(QApplication* app, const QString& section_uid, QObject* parent)
    : QObject(parent)
{
    this->m_app = app;
    this->m_section_uid = section_uid;
    setup_qml();
}

void ChantingReviewWindow::setup_qml() {
    const QUrl view_qml(QStringLiteral("qrc:/qt/qml/com/profoundlabs/simsapa/assets/qml/ChantingPracticeReviewWindow.qml"));
    m_engine = new QQmlApplicationEngine(this);
    m_engine->rootContext()->setContextProperty("section_uid", m_section_uid);
    m_engine->load(view_qml);
    m_root = m_engine->rootObjects().constFirst();
}

ChantingReviewWindow::~ChantingReviewWindow() {
    delete m_engine;
}
