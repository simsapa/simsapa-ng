#include "word_lookup_window.h"

#include <QUrl>
#include <QMenuBar>
#include <QQmlEngine>
#include <QQuickWidget>

WordLookupWindow::WordLookupWindow(QApplication* app, const QString& word, QWidget* parent)
    : QMainWindow(parent)
{
    this->m_app = app;
    setup_qml();
    setup_menu_bar();
    m_root->setProperty("word", word);
    m_root->setProperty("definition_plain", "Deinition of " + word + ":\nLorem ipsum...");
}


void WordLookupWindow::setup_qml() {
    // FIXME set minimum / base size
    // resize(1024, 768);

    const QUrl view_qml(QStringLiteral("qrc:/qt/qml/com/profound_labs/simsapa/qml/word_lookup_window.qml"));

    m_engine = new QQmlEngine();
    m_view = new QQuickWidget(m_engine, this);
    m_view->setSource(view_qml);
    m_root = this->m_view->rootObject();
    this->setCentralWidget(m_view);
}

void WordLookupWindow::setup_menu_bar() {
    QMenuBar *menu_bar = new QMenuBar();

    QMenu *menu_file = menu_bar->addMenu("&File");

    auto action_close_window = new QAction("&Close Window", this->m_view);
    menu_file->addAction(action_close_window);

    connect(action_close_window, &QAction::triggered, [this](bool checked) { this->close(); });

    auto action_quit = new QAction(QIcon(":/icons/close"), "&Quit Simsapa", this->m_view);
    menu_file->addAction(action_quit);

    action_quit->setShortcut(QKeySequence::Quit);
    connect(action_quit, &QAction::triggered, [this](bool checked) { qApp->quit(); });

    this->setMenuBar(menu_bar);
}

WordLookupWindow::~WordLookupWindow() {
    delete m_view;
    delete m_engine;
}
