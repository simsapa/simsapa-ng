#ifndef SUTTA_LANGUAGES_WINDOW_H
#define SUTTA_LANGUAGES_WINDOW_H

#include <QObject>
#include <QApplication>
#include <QQmlApplicationEngine>

class SuttaLanguagesWindow : public QObject {
    Q_OBJECT

public:
    explicit SuttaLanguagesWindow(QApplication* app, QObject* parent = nullptr);
    ~SuttaLanguagesWindow();

    QApplication* m_app;
    QObject* m_root;
    QQmlApplicationEngine *m_engine;

private:
    void setup_qml();
};

#endif
