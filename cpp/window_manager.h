#ifndef WINDOW_MANAGER_H
#define WINDOW_MANAGER_H

#include <QObject>
#include <QString>
#include <QList>
#include <QMainWindow>

class SuttaSearchWindow;
class DownloadAppdataWindow;
class WordLookupWindow;

class WindowManager : public QObject {
        Q_OBJECT
    public:
        static WindowManager& instance(QApplication* app);
        static void lookup_word(const QString& word);

        void create_plain_sutta_search_window();
        SuttaSearchWindow* create_sutta_search_window();
        DownloadAppdataWindow* create_download_appdata_window();
        WordLookupWindow* create_word_lookup_window(const QString& word);

        static WindowManager *m_instance;
        QApplication* m_app;
        int m_window_id_count;
        QList<SuttaSearchWindow*> sutta_search_windows;
        QList<DownloadAppdataWindow*> download_appdata_windows;
        QList<WordLookupWindow*> word_lookup_windows;

    private:
        WindowManager(QApplication* app, QObject *parent = nullptr);
        ~WindowManager();

    signals:
        void signal_run_lookup_query(const QString& query_text);
        void signal_run_summary_query(const QString& window_id, const QString& query_text);

    public slots:
        void run_lookup_query(const QString& query_text);
        void run_summary_query(const QString& window_id, const QString& query_text);

};

#endif
