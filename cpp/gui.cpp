#include <sstream>
#include <thread>

#include <QUrl>
#include <QDir>
#include <QMenu>
#include <QIcon>
#include <QAction>
#include <QObject>
#include <QString>
#include <QSysInfo>
#include <QSystemTrayIcon>
#include <QApplication>
#include <QMainWindow>
#include <QEventLoop>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQuickStyle>
#include <QQuickWindow>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>

// #include <QtWebView/QtWebView>
// #include <QtWebEngineQuick/qtwebenginequickglobal.h>

#include "errors.h"
#include "window_manager.h"
#include "sutta_search_window.h"

extern "C" void start_webserver();
extern "C" void shutdown_webserver();
extern "C" bool appdata_db_exists();
extern "C" void ensure_no_empty_db_files();
extern "C" void check_delete_files_for_upgrade();
extern "C" void remove_download_temp_folder();
extern "C" void init_app_globals();
extern "C" void init_app_data();
extern "C" void import_user_data_after_upgrade();
extern "C" void cleanup_stale_legacy_userdata();
extern "C" void check_and_configure_for_first_start();
extern "C" bool reconcile_dict_indexes_needed_c();
extern "C" void reconcile_dict_indexes_blocking_c();
extern "C" void create_or_update_linux_desktop_icon_file_ffi();

extern "C" char* get_desktop_file_path_ffi();
extern "C" void free_rust_string(char* s);
extern "C" void dotenv_c();
extern "C" bool find_port_set_env_c();

extern "C" void log_info_c(const char* msg);
extern "C" void log_info_with_options_c(const char* msg, bool start_new);

struct AppGlobals {
    static WindowManager* manager;
};

WindowManager* AppGlobals::manager = nullptr;

void callback_run_lookup_query(QString query_text) {
  emit AppGlobals::manager->signal_run_lookup_query(query_text);
}

void callback_run_summary_query(QString window_id, QString query_text) {
  emit AppGlobals::manager->signal_run_summary_query(window_id, query_text);
}

void callback_run_sutta_menu_action(QString window_id, QString action, QString query_text) {
  emit AppGlobals::manager->signal_run_sutta_menu_action(window_id, action, query_text);
}

void callback_run_dppn_dictionary_query(QString window_id, QString query) {
  emit AppGlobals::manager->signal_run_dppn_dictionary_query(window_id, query);
}

void callback_open_sutta_search_window(QString show_result_data_json) {
  emit AppGlobals::manager->signal_open_sutta_search_window(show_result_data_json);
}

void callback_open_sutta_tab(QString window_id, QString show_result_data_json) {
  emit AppGlobals::manager->signal_open_sutta_tab(window_id, show_result_data_json);
}

void callback_open_sutta_languages_window() {
  AppGlobals::manager->create_sutta_languages_window();
}

void callback_open_dictionaries_window() {
  AppGlobals::manager->create_dictionaries_window();
}

void callback_open_library_window() {
  AppGlobals::manager->create_library_window();
}

void callback_open_reference_search_window() {
  AppGlobals::manager->create_reference_search_window();
}

void callback_open_topic_index_window() {
  AppGlobals::manager->create_topic_index_window();
}

void callback_open_chanting_practice_window(QString window_id) {
  AppGlobals::manager->create_chanting_practice_window(window_id);
}

void callback_open_chanting_review_window(QString window_id, QString section_uid) {
  AppGlobals::manager->create_chanting_review_window(window_id, section_uid);
}

void callback_show_chapter_in_sutta_window(QString window_id, QString result_data_json) {
  AppGlobals::manager->show_chapter_in_sutta_window(window_id, result_data_json);
}

void callback_show_sutta_from_reference_search(QString window_id, QString result_data_json) {
  AppGlobals::manager->show_sutta_from_reference_search(window_id, result_data_json);
}

void callback_toggle_reading_mode(QString window_id, bool is_active) {
  emit AppGlobals::manager->signal_toggle_reading_mode(window_id, is_active);
}

void callback_open_in_lookup_window(QString result_data_json) {
  emit AppGlobals::manager->signal_open_in_lookup_window(result_data_json);
}

int start(int argc, char* argv[]) {
  dotenv_c();
  log_info_with_options_c("gui::start()", true);
  find_port_set_env_c();
  init_app_globals();
  remove_download_temp_folder();

  // There may be a 0-byte size db file remaining from a failed
  // install attempt.
  ensure_no_empty_db_files();

  // Check if database files should be deleted for an upgrade.
  // This is triggered by the delete_files_for_upgrade.txt marker file
  // created by prepare_for_database_upgrade().
  check_delete_files_for_upgrade();

  QString os(QSysInfo::productType());

  // Initialize a QtWebView / QtWebEngineView. Otherwise the app errors:
  //
  // QtWebEngineWidgets must be imported or Qt.AA_ShareOpenGLContexts must be
  // set before a QCoreApplication instance is created

  // TODO How to avoid the linker trying to link one which doesn't exist for the platform?
  // E.g. when building for desktop, we don't include QtWebView.
  //
  // gui.cpp:(.text+0x270): undefined reference to `QtWebView::initialize()'
  // collect2: error: ld returned 1 exit status

  // if (os == "android" || os == "ios") {
  //   QtWebView::initialize();
  // } else {
  //   QtWebEngineQuick::initialize();
  // }

  // Linux: Check if the .desktop file should be created or updated. When a
  // user updates the .AppImage, the file name contains a different version
  // number.
  create_or_update_linux_desktop_icon_file_ffi();

#ifdef Q_OS_ANDROID
  // Use the native Android multimedia backend instead of FFmpeg.
  // The FFmpeg backend's MediaRecorder fails on Android with
  // "Audio device has invalid preferred format" because
  // QAudioDevice::preferredFormat() returns an invalid format
  // (0 sample rate / 0 channels) from Android's AudioManager.
  qputenv("QT_MEDIA_BACKEND", "android");
#endif

  // QApplication has to be constructed before other windows or dialogs.
  QApplication app(argc, argv);

  QQuickStyle::setStyle("Fusion");

  QCoreApplication::setApplicationName("simsapa-ng");
  // NOTE: Don't use setOrganizationName(), because Qt adds it as a folder to the internal storage path.

  // TODO :/icons/simsapa-appicon doesn't work, perhaps wrong size?
  app.setWindowIcon(QIcon(":/qt/qml/com/profoundlabs/simsapa/assets/qml/icons/32x32/simsapa-tray.png"));

  // Set desktop file name for Linux desktop integration
  char* desktop_file_path = get_desktop_file_path_ffi();
  if (desktop_file_path != nullptr) {
    app.setDesktopFileName(QString::fromUtf8(desktop_file_path));
    free_rust_string(desktop_file_path);
  }

  app.setApplicationVersion("v0.4.0-alpha.1");

  // app_windows = AppWindows(app, app_data, hotkeys_manager, enable_tray_icon)

  // setup_system_tray();

  log_info_c("setup_system_tray(): start");
  QSystemTrayIcon tray = QSystemTrayIcon(QIcon(":/qt/qml/com/profoundlabs/simsapa/assets/qml/icons/32x32/simsapa-tray.png"), &app);
  tray.setVisible(true);

  QMenu* menu = new QMenu();

  QAction* action_Quit = new QAction(QIcon(":/qt/qml/com/profoundlabs/simsapa/assets/qml/icons/32x32/fa_times-circle.png"), "Quit", &app);
  QObject::connect(action_Quit, SIGNAL(triggered()), QApplication::instance(), SLOT(quit()));

  menu->addAction(action_Quit);

  tray.setContextMenu(menu);

  log_info_c("setup_system_tray(): end");

  // Determine if this is the first start and we need to open
  // DownloadAppdataWindow instead of the main app.

  AppGlobals::manager = &WindowManager::instance(&app);

  if (!appdata_db_exists()) {

    AppGlobals::manager->create_download_appdata_window();

    // QUrl view_qml(QStringLiteral("qrc:/qt/qml/com/profoundlabs/simsapa/assets/qml/DownloadAppdataWindow.qml"));
    // QQmlApplicationEngine engine(&app);
    // engine.load(view_qml);

    log_info_c("app.exec()");
    int status = app.exec();

    std::ostringstream msg;
    msg << "Exiting with status " << status << ".";
    log_info_c(msg.str().c_str());

    throw NormalExit("Exiting after DownloadAppdataWindow", status);
  }

  // Init AppData and start the API server after checking for APP_DB.
  init_app_data();

  // Import user data from the import-me folder if it exists.
  // This restores app settings and user-imported books after a database upgrade.
  import_user_data_after_upgrade();

  // Remove any stale legacy userdata.sqlite3 left behind after the one-shot
  // alpha-upgrade bridge completed. No-op when no import-me/ is pending.
  cleanup_stale_legacy_userdata();

  // Check if this is the first start and configure settings based on system memory
  check_and_configure_for_first_start();

  // The port is determined in start_webserver().
  std::thread daemon_server_thread(start_webserver);

  // Reconcile user-imported dictionary indexes before opening the main
  // window. Runs only if there is work to do (newly imported / renamed /
  // deleted user dictionaries, or orphan source_uids in the Tantivy dict
  // index from a release-upgrade DB swap). Tantivy writes happen here so
  // they never contend with a live searcher in `SuttaSearchWindow`.
  //
  // The QML window drives reconciliation through the `DictionaryManager`
  // bridge (worker thread + Qt signals) and closes itself on
  // `reconcileFinished`. We pump a local `QEventLoop` until the window's
  // QQuickWindow is destroyed, then proceed.
  if (reconcile_dict_indexes_needed_c()) {
    log_info_c("Showing dictionary index reconciliation window...");

    QQmlApplicationEngine reconcile_engine;
    reconcile_engine.load(QUrl(QStringLiteral(
      "qrc:/qt/qml/com/profoundlabs/simsapa/assets/qml/DictionaryIndexProgressWindow.qml")));

    auto roots = reconcile_engine.rootObjects();
    if (!roots.isEmpty()) {
      QObject* window_root = roots.constFirst();
      QEventLoop reconcile_loop;
      QObject::connect(window_root, &QObject::destroyed, &reconcile_loop, &QEventLoop::quit);
      QQuickWindow* qwin = qobject_cast<QQuickWindow*>(window_root);
      if (qwin) {
        QObject::connect(qwin, &QQuickWindow::visibleChanged, &reconcile_loop, [&reconcile_loop, qwin]() {
          if (!qwin->isVisible()) reconcile_loop.quit();
        });
      }
      reconcile_loop.exec();
      log_info_c("Dictionary index reconciliation complete.");
    } else {
      // Fallback — couldn't load the QML; run synchronously so the app still
      // makes progress.
      log_info_c("Reconciliation window failed to load; running synchronously.");
      reconcile_dict_indexes_blocking_c();
    }
  }

  // === Create the first app window ===

  AppGlobals::manager->create_sutta_search_window();

  // Restore last session if enabled
  AppGlobals::manager->restore_last_session();

  // Save last session on exit
  QObject::connect(&app, &QApplication::aboutToQuit, [&]() {
    log_info_c("aboutToQuit: saving last session");
    QJsonArray all_windows;
    for (auto w : AppGlobals::manager->sutta_search_windows) {
      if (w->m_root) {
        // Only save windows that are still visible (not closed/hidden)
        QVariant visible = w->m_root->property("visible");
        if (!visible.isValid() || !visible.toBool()) {
          continue;
        }
        QString session_json;
        QMetaObject::invokeMethod(w->m_root, "get_session_data_json",
          Q_RETURN_ARG(QString, session_json));
        if (!session_json.isEmpty()) {
          QJsonDocument doc = QJsonDocument::fromJson(session_json.toUtf8());
          if (!doc.isNull()) {
            all_windows.append(doc.object());
          }
        }
      }
    }
    if (!all_windows.isEmpty() && AppGlobals::manager->sutta_search_windows.length() > 0) {
      QString windows_json = QJsonDocument(all_windows).toJson(QJsonDocument::Compact);
      auto first_window = AppGlobals::manager->sutta_search_windows.first();
      if (first_window->m_root) {
        QMetaObject::invokeMethod(first_window->m_root, "save_last_session",
          Q_ARG(QString, windows_json));
      }
    }
  });

  log_info_c("app.exec()");
  int status = app.exec();

  shutdown_webserver();
  if (daemon_server_thread.joinable()) {
    daemon_server_thread.join();
  }
  std::ostringstream msg;
  msg << "Exiting with status " << status << ".";
  log_info_c(msg.str().c_str());

  return status;
}
