#ifndef LIBRARY_WINDOW_H
#define LIBRARY_WINDOW_H

#include <QObject>
#include <QApplication>
#include <QQmlApplicationEngine>

class LibraryWindow : public QObject {
    Q_OBJECT

public:
    explicit LibraryWindow(QApplication* app, QObject* parent = nullptr);
    ~LibraryWindow();

    QApplication* m_app;
    QObject* m_root;
    QQmlApplicationEngine *m_engine;

private:
    void setup_qml();
};

#endif
