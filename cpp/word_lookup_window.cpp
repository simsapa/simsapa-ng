#include "word_lookup_window.h"

#include <QUrl>

WordLookupWindow::WordLookupWindow(QApplication* app, const QString& word, QObject* parent)
    : QObject(parent)
{
    this->m_app = app;
    setup_qml();
    m_root->setProperty("word", word);
    m_root->setProperty("definition_plain", "Deinition of " + word + ":\nLorem ipsum...");
}

void WordLookupWindow::setup_qml() {
    const QUrl view_qml(QStringLiteral("qrc:/qt/qml/com/profoundlabs/simsapa/assets/qml/word_lookup_window.qml"));
    m_engine = new QQmlApplicationEngine(view_qml, this);
    m_root = m_engine->rootObjects().constFirst();
}

WordLookupWindow::~WordLookupWindow() {
    delete m_engine;
}
