#ifndef WORD_LOOKUP_WINDOW_H
#define WORD_LOOKUP_WINDOW_H

#include <QMainWindow>
#include <QQmlEngine>
#include <QQuickWidget>
#include <QQuickItem>

class WordLookupWindow : public QMainWindow {
    Q_OBJECT

public:
    explicit WordLookupWindow(QApplication* app, const QString& word, QWidget* parent = nullptr);
    ~WordLookupWindow();

    QApplication* m_app;
    QQuickWidget* m_view;
    QQuickItem* m_root;
    QQmlEngine* m_engine;

private:
    void setup_qml();
    void setup_menu_bar();
};

#endif
