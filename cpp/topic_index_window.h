#ifndef TOPIC_INDEX_WINDOW_H
#define TOPIC_INDEX_WINDOW_H

#include <QObject>
#include <QApplication>
#include <QQmlApplicationEngine>

class TopicIndexWindow : public QObject {
    Q_OBJECT

public:
    explicit TopicIndexWindow(QApplication* app, QObject* parent = nullptr);
    ~TopicIndexWindow();

    QApplication* m_app;
    QObject* m_root;
    QQmlApplicationEngine *m_engine;

private:
    void setup_qml();
};

#endif
