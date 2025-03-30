#ifndef WORD_LOOKUP_WINDOW_H
#define WORD_LOOKUP_WINDOW_H

#include <QObject>
#include <QApplication>
#include <QQmlApplicationEngine>

class WordLookupWindow : public QObject {
    Q_OBJECT

public:
    explicit WordLookupWindow(QApplication* app, const QString& word, QObject* parent = nullptr);
    ~WordLookupWindow();

    QApplication* m_app;
    QObject* m_root;
    QQmlApplicationEngine *m_engine;

private:
    void setup_qml();
};

#endif
