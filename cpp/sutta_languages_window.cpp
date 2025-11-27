#include "sutta_languages_window.h"

#include <QUrl>

SuttaLanguagesWindow::SuttaLanguagesWindow(QApplication* app, QObject* parent)
    : QObject(parent)
{
    this->m_app = app;
    setup_qml();
}

void SuttaLanguagesWindow::setup_qml() {
    const QUrl view_qml(QStringLiteral("qrc:/qt/qml/com/profoundlabs/simsapa/assets/qml/SuttaLanguagesWindow.qml"));
    m_engine = new QQmlApplicationEngine(view_qml, this);
    m_root = m_engine->rootObjects().constFirst();
}

SuttaLanguagesWindow::~SuttaLanguagesWindow() {
    delete m_engine;
}
