#ifndef DICTIONARIES_WINDOW_H
#define DICTIONARIES_WINDOW_H

#include <QObject>
#include <QApplication>
#include <QQmlApplicationEngine>

class DictionariesWindow : public QObject {
    Q_OBJECT

public:
    explicit DictionariesWindow(QApplication* app, QObject* parent = nullptr);
    ~DictionariesWindow();

    QApplication* m_app;
    QObject* m_root;
    QQmlApplicationEngine *m_engine;

private:
    void setup_qml();
};

#endif
