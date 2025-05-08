#include "window_manager.h"
#include "sutta_search_window.h"
#include "download_appdata_window.h"
#include "word_lookup_window.h"

WindowManager* WindowManager::m_instance = nullptr;

WindowManager& WindowManager::instance(QApplication* app) {
    if (!m_instance) {
        m_instance = new WindowManager(app);
    }
    return *m_instance;
}

WindowManager::WindowManager(QApplication* app, QObject* parent)
    : QObject(parent)
{
    this->m_app = app;

    QObject::connect(this, &WindowManager::signal_run_lookup_query, this, &WindowManager::run_lookup_query);
}

WindowManager::~WindowManager() {
    // Clean up all windows
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
    // w->m_root->show();
    return w;
}

DownloadAppdataWindow* WindowManager::create_download_appdata_window() {
    DownloadAppdataWindow* w = new DownloadAppdataWindow(this->m_app);
    download_appdata_windows.append(w);
    // w->m_root->show();
    return w;
}

WordLookupWindow* WindowManager::create_word_lookup_window(const QString& word) {
    WordLookupWindow* w = new WordLookupWindow(this->m_app, word);
    word_lookup_windows.append(w);
    // w->show();
    return w;
}

void WindowManager::run_lookup_query(const QString& word) {
    this->create_word_lookup_window(word);
}
