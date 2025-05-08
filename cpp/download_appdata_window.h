#ifndef DOWNLOAD_APPDATA_WINDOW_H
#define DOWNLOAD_APPDATA_WINDOW_H

#include <QObject>
#include <QApplication>
#include <QQmlApplicationEngine>

class DownloadAppdataWindow : public QObject {
    Q_OBJECT

public:
    explicit DownloadAppdataWindow(QApplication* app, QObject* parent = nullptr);
    ~DownloadAppdataWindow();

    QApplication* m_app;
    QObject* m_root;
    QQmlApplicationEngine *m_engine;

private:
    void setup_qml();
};

#endif
