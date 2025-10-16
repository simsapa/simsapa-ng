#include "window_manager.h"
#include "sutta_search_window.h"
#include "download_appdata_window.h"
#include "word_lookup_window.h"
#include <QVariant>

WindowManager* WindowManager::m_instance = nullptr;

WindowManager& WindowManager::instance(QApplication* app) {
    if (!m_instance) {
        m_instance = new WindowManager(app);
        m_instance->m_window_id_count = 0;
    }
    return *m_instance;
}

WindowManager::WindowManager(QApplication* app, QObject* parent)
    : QObject(parent)
{
    this->m_app = app;

    QObject::connect(this, &WindowManager::signal_run_lookup_query, this, &WindowManager::run_lookup_query);
    QObject::connect(this, &WindowManager::signal_run_summary_query, this, &WindowManager::run_summary_query);
    QObject::connect(this, &WindowManager::signal_run_sutta_menu_action, this, &WindowManager::run_sutta_menu_action);
}

WindowManager::~WindowManager() {
    // Clean up all windows
    // FIXME: does this clean up work?
    while (!sutta_search_windows.isEmpty()) {
        auto w = sutta_search_windows.takeFirst();
        w->deleteLater();
    }

    while (!download_appdata_windows.isEmpty()) {
        auto w = download_appdata_windows.takeFirst();
        w->deleteLater();
    }

    while (!word_lookup_windows.isEmpty()) {
        auto w = word_lookup_windows.takeFirst();
        w->deleteLater();
    }
}

SuttaSearchWindow* WindowManager::create_sutta_search_window() {
    SuttaSearchWindow* w = new SuttaSearchWindow(this->m_app);
    sutta_search_windows.append(w);
    w->m_root->setProperty("window_id", QString("window_%1").arg(this->m_window_id_count));
    this->m_window_id_count++;
    return w;
}

DownloadAppdataWindow* WindowManager::create_download_appdata_window() {
    DownloadAppdataWindow* w = new DownloadAppdataWindow(this->m_app);
    download_appdata_windows.append(w);
    return w;
}

WordLookupWindow* WindowManager::create_word_lookup_window(const QString& word) {
    WordLookupWindow* w = new WordLookupWindow(this->m_app, word);
    word_lookup_windows.append(w);
    return w;
}

void WindowManager::run_lookup_query(const QString& query_text) {
    this->create_word_lookup_window(query_text);
}

void WindowManager::run_summary_query(const QString& window_id, const QString& query_text) {
    // NOTE: .isEmpty() returns true even when .length() > 0
    if (this->sutta_search_windows.length() == 0) {
        return;
    }

    SuttaSearchWindow* target_window = nullptr;
    for (auto w : this->sutta_search_windows) {
        QVariant prop = w->m_root->property("window_id");
        if (prop.isValid() && prop.toString() == window_id) {
            target_window = w;
            break;
        }
    }

    if (target_window == nullptr) {
        return;
    }

    QMetaObject::invokeMethod(target_window->m_root, "set_summary_query", Q_ARG(QString, query_text));
}

void WindowManager::run_sutta_menu_action(const QString& window_id, const QString& action, const QString& query_text) {
    if (this->sutta_search_windows.length() == 0) {
        return;
    }

    SuttaSearchWindow* target_window = nullptr;
    for (auto w : this->sutta_search_windows) {
        QVariant prop = w->m_root->property("window_id");
        if (prop.isValid() && prop.toString() == window_id) {
            target_window = w;
            break;
        }
    }

    if (target_window == nullptr) {
        return;
    }

    QMetaObject::invokeMethod(target_window->m_root, "run_sutta_menu_action", Q_ARG(QString, action), Q_ARG(QString, query_text));
}
