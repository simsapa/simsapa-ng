#include "sutta_search_window.h"

#include <QAction>
#include <QObject>
#include <QUrl>
#include <QQmlEngine>
#include <QQuickWidget>
#include <QCoreApplication>
#include <QApplication>
#include <QMenuBar>
#include <QKeySequence>

SuttaSearchWindow::SuttaSearchWindow(QApplication* app, QWidget* parent)
    : QMainWindow(parent)
{
    this->m_app = app;
    setup_qml();
    setup_menu_bar();
}

void SuttaSearchWindow::setup_qml() {
    // FIXME set minimum / base size
    // resize(1024, 768);

    const QUrl view_qml(QStringLiteral("qrc:/qt/qml/com/profound_labs/simsapa/qml/sutta_search_window.qml"));

    m_engine = new QQmlEngine();
    m_view = new QQuickWidget(m_engine, this);
    m_view->setSource(view_qml);
    m_root = this->m_view->rootObject();
    this->setCentralWidget(m_view);

    // Alternative way to load the view QQuickView. The root item (Item or
    // Rectangle) is not auto-resized to the window size when first opened.

    // QQuickView *view = new QQuickView(view_qml);
    // QWidget *container = QWidget::createWindowContainer(view);
    // this->setCentralWidget(container);
}

void SuttaSearchWindow::setup_menu_bar() {
    QMenuBar *menu_bar = new QMenuBar();

    QMenu *menu_file = menu_bar->addMenu("&File");

    auto action_close_window = new QAction("&Close Window", this->m_view);
    menu_file->addAction(action_close_window);

    connect(action_close_window, &QAction::triggered, [this](bool checked) { this->close(); });

    auto action_quit = new QAction(QIcon(":/icons/close"), "&Quit Simsapa", this->m_view);
    menu_file->addAction(action_quit);

    action_quit->setShortcut(QKeySequence::Quit);
    connect(action_quit, &QAction::triggered, [this](bool checked) { qApp->quit(); });

    QMenu *menu_windows = menu_bar->addMenu("&Windows");

    auto action_sutta_search = new QAction(QIcon(":/icons/book"), "&Sutta Search", this->m_view);
    menu_windows->addAction(action_sutta_search);

    action_sutta_search->setShortcut(Qt::Key_F5);

    this->setMenuBar(menu_bar);
}

SuttaSearchWindow::~SuttaSearchWindow() {
    delete m_view;
    delete m_engine;
}

