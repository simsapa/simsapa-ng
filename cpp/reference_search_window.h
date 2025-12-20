#ifndef REFERENCE_SEARCH_WINDOW_H
#define REFERENCE_SEARCH_WINDOW_H

#include <QObject>
#include <QApplication>
#include <QQmlApplicationEngine>

class ReferenceSearchWindow : public QObject {
    Q_OBJECT

public:
    explicit ReferenceSearchWindow(QApplication* app, QObject* parent = nullptr);
    ~ReferenceSearchWindow();

    QApplication* m_app;
    QObject* m_root;
    QQmlApplicationEngine *m_engine;

private:
    void setup_qml();
};

#endif
