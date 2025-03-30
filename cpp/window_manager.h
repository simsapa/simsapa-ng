#ifndef WINDOW_MANAGER_H
#define WINDOW_MANAGER_H

#include <QObject>
#include <QString>
#include <QList>
#include <QMainWindow>

class SuttaSearchWindow;
class WordLookupWindow;

class WindowManager : public QObject {
        Q_OBJECT
    public:
        static WindowManager& instance(QApplication* app);
        static void lookup_word(const QString& word);

        void create_plain_sutta_search_window();
        SuttaSearchWindow* create_sutta_search_window();
        WordLookupWindow* create_word_lookup_window(const QString& word);

        static WindowManager *m_instance;
        QApplication* m_app;
        QList<SuttaSearchWindow*> sutta_search_windows;
        QList<WordLookupWindow*> word_lookup_windows;

    private:
        WindowManager(QApplication* app, QObject *parent = nullptr);
        ~WindowManager();

    signals:
        void signal_run_lookup_query(const QString& query_text);

    public slots:
        void run_lookup_query(const QString& word);

};

#endif
