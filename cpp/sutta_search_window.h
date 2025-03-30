#ifndef SUTTA_SEARCH_WINDOW_H
#define SUTTA_SEARCH_WINDOW_H

#include <QObject>
#include <QApplication>
#include <QQmlApplicationEngine>

class SuttaSearchWindow : public QObject {
    Q_OBJECT

public:
    explicit SuttaSearchWindow(QApplication* app, QObject* parent = nullptr);
    ~SuttaSearchWindow();

    QApplication* m_app;
    QObject* m_root;
    QQmlApplicationEngine *m_engine;

private:
    void setup_qml();
};

#endif
