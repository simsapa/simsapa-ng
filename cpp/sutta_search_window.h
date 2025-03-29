#ifndef SUTTA_SEARCH_WINDOW_H
#define SUTTA_SEARCH_WINDOW_H

#include <QMainWindow>
#include <QQmlEngine>
#include <QQuickWidget>
#include <QQuickItem>
#include <QAction>

class SuttaSearchWindow : public QMainWindow {
    Q_OBJECT

public:
    explicit SuttaSearchWindow(QApplication* app, QWidget* parent = nullptr);
    ~SuttaSearchWindow();

    QApplication* m_app;
    QQuickWidget* m_view;
    QQuickItem* m_root;
    QQmlEngine* m_engine;

private:
    void setup_qml();
    void setup_menu_bar();
};

#endif
