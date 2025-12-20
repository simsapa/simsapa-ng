#include "window_manager.h"
#include "sutta_search_window.h"
#include "download_appdata_window.h"
#include "word_lookup_window.h"
#include "sutta_languages_window.h"
#include "library_window.h"
#include "reference_search_window.h"
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
    QObject::connect(this, &WindowManager::signal_open_sutta_search_window, this, &WindowManager::open_sutta_search_window_with_query);
    QObject::connect(this, &WindowManager::signal_open_sutta_tab, this, &WindowManager::open_sutta_tab_in_window);
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

    while (!sutta_languages_windows.isEmpty()) {
        auto w = sutta_languages_windows.takeFirst();
        w->deleteLater();
    }

    while (!library_windows.isEmpty()) {
        auto w = library_windows.takeFirst();
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

SuttaLanguagesWindow* WindowManager::create_sutta_languages_window() {
    SuttaLanguagesWindow* w = new SuttaLanguagesWindow(this->m_app);
    sutta_languages_windows.append(w);
    return w;
}

LibraryWindow* WindowManager::create_library_window() {
    LibraryWindow* w = new LibraryWindow(this->m_app);
    library_windows.append(w);
    return w;
}

ReferenceSearchWindow* WindowManager::create_reference_search_window() {
    ReferenceSearchWindow* w = new ReferenceSearchWindow(this->m_app);
    reference_search_windows.append(w);
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

void WindowManager::open_sutta_search_window_with_query(const QString& show_result_data_json) {
    SuttaSearchWindow* w = this->create_sutta_search_window();

    // If result data JSON is provided, show the sutta directly
    if (!show_result_data_json.isEmpty() && w && w->m_root) {
        QMetaObject::invokeMethod(w->m_root, "show_result_in_html_view_with_json",
            Q_ARG(QString, show_result_data_json),
            Q_ARG(QVariant, QVariant(false)));  // Don't create new tab in fresh window
    }
}

void WindowManager::open_sutta_tab_in_window(const QString& window_id, const QString& show_result_data_json) {
    // Find the window with matching window_id
    SuttaSearchWindow* target_window = nullptr;

    if (this->sutta_search_windows.length() == 0) {
        return;
    }

    if (window_id.isEmpty()) {
        // Fall back to last window if no window_id provided
        target_window = this->sutta_search_windows.last();
    } else {
        // Find the window with matching window_id
        for (auto w : this->sutta_search_windows) {
            QVariant prop = w->m_root->property("window_id");
            if (prop.isValid() && prop.toString() == window_id) {
                target_window = w;
                break;
            }
        }
    }

    if (target_window && target_window->m_root) {
        // Show and raise the window
        QMetaObject::invokeMethod(target_window->m_root, "show");
        QMetaObject::invokeMethod(target_window->m_root, "raise");

        // Show the sutta in a new tab
        QMetaObject::invokeMethod(target_window->m_root, "show_result_in_html_view_with_json",
            Q_ARG(QString, show_result_data_json),
            Q_ARG(QVariant, QVariant(true)));  // Pass true to create a new tab
    }
}

void WindowManager::show_chapter_in_sutta_window(const QString& window_id, const QString& result_data_json) {
    // If window_id is empty, fall back to the last window (for backwards compatibility)
    // Otherwise, find the specific window by window_id
    SuttaSearchWindow* target_window = nullptr;

    if (this->sutta_search_windows.length() == 0) {
        return;
    }

    if (window_id.isEmpty()) {
        // Fall back to last window if no window_id provided
        target_window = this->sutta_search_windows.last();
    } else {
        // Find the window with matching window_id
        for (auto w : this->sutta_search_windows) {
            QVariant prop = w->m_root->property("window_id");
            if (prop.isValid() && prop.toString() == window_id) {
                target_window = w;
                break;
            }
        }
    }

    if (target_window && target_window->m_root) {
        // Show and raise the window
        QMetaObject::invokeMethod(target_window->m_root, "show");
        QMetaObject::invokeMethod(target_window->m_root, "raise");

        // Show the chapter in the HTML view (replace current tab, don't create new)
        QMetaObject::invokeMethod(target_window->m_root, "show_result_in_html_view_with_json",
            Q_ARG(QString, result_data_json),
            Q_ARG(QVariant, QVariant(false)));
    }
}
