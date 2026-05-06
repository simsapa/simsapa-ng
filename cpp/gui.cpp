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
#include "global_hotkey_manager.h"

#include <QClipboard>
#include <QKeySequence>
#include <QMessageBox>
#include <QRegularExpression>
#include <QTimer>

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
extern "C" void log_error_c(const char* msg);
extern "C" void log_info_with_options_c(const char* msg, bool start_new);

extern "C" bool global_hotkeys_enabled_c();
extern "C" char* get_global_hotkey_dictionary_lookup_c();

struct AppGlobals {
    static WindowManager* manager;
    static GlobalHotkeyManager* global_hotkey_manager;
};

WindowManager* AppGlobals::manager = nullptr;
GlobalHotkeyManager* AppGlobals::global_hotkey_manager = nullptr;

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

/// Sanitize the captured selection: strip control chars, collapse whitespace,
/// trim, and cap at 200 characters per PRD §4.5.
static QString sanitize_lookup_query(const QString& raw) {
  QString s = raw;
  s.replace(QRegularExpression("[\\r\\n\\t]+"), " ");
  s.replace(QRegularExpression("\\s+"), " ");
  s = s.trimmed();
  if (s.length() > 200) {
    s.truncate(200);
  }
  return s;
}

/// Run the dictionary lookup activation pipeline. The hotkey may have fired
/// just before the foreground app populated the clipboard with its own copy
/// (for `Ctrl+C+C` the second `C` is the user's own copy keystroke), so we
/// give X a brief moment to settle before reading.
static void run_global_hotkey_lookup(int handle) {
  Q_UNUSED(handle);
  // ~80 ms is conservative on X11; <50 ms is usually enough. Single-shot timer
  // avoids blocking the GUI thread.
  QTimer::singleShot(80, qApp, [](){
    QClipboard* clipboard = QGuiApplication::clipboard();
    if (!clipboard) return;
    QString raw = clipboard->text(QClipboard::Clipboard);
    QString query = sanitize_lookup_query(raw);
    if (query.isEmpty()) {
      log_info_c("global_hotkey: clipboard empty after sanitize, aborting");
      return;
    }
    log_info_c(QString("global_hotkey: running dictionary lookup for %1 chars")
               .arg(query.length()).toUtf8().constData());
    if (AppGlobals::manager) {
      // run_lookup_query in WindowManager creates/reuses the lookup window,
      // shows + raises it, and triggers the QML-side run_lookup_query()
      // which sets search area = Dictionary and runs the search.
      emit AppGlobals::manager->signal_run_lookup_query(query);
    }
  });
}

void callback_global_hotkey_activated(int handle) {
  log_info_c(QString("global_hotkey_activated: handle=%1").arg(handle).toUtf8().constData());
  run_global_hotkey_lookup(handle);
}

// One-time error dialog suppression flag (PRD §8.5/§8.6). Cleared whenever
// the user changes the configured sequence so a fresh conflict surfaces a
// fresh dialog.
static bool s_global_hotkey_error_shown = false;

/// Show a platform-appropriate one-time error dialog when registerHotkey()
/// fails. Recorded in `s_global_hotkey_error_shown` for the lifetime of the
/// session (or until the user changes the sequence).
static void show_global_hotkey_registration_error(const QString& sequence) {
  if (s_global_hotkey_error_shown) return;
  s_global_hotkey_error_shown = true;

  QString detail;
#if defined(Q_OS_LINUX)
  detail = "On Linux, make sure your X server has the RECORD extension enabled "
           "and that no other application is grabbing the same key combination.";
#elif defined(Q_OS_WIN)
  detail = "On Windows, the key combination may already be reserved by another "
           "application. Try a different sequence.";
#elif defined(Q_OS_MACOS)
  detail = "On macOS, ensure that Simsapa has Accessibility permission, and that "
           "the key combination isn't already used by another application.";
#else
  detail = "Global hotkeys are not supported on this platform.";
#endif

  QMessageBox::critical(nullptr,
    QStringLiteral("Global hotkey registration failed"),
    QStringLiteral("Could not register the global hotkey \"%1\".\n\n%2")
      .arg(sequence, detail));
}

/// Read the configured sequence from settings, register it with the C++
/// manager, and surface a one-time error dialog on failure. Safe to call
/// repeatedly (it does NOT call unregisterAll() — the caller is responsible
/// for that ordering when re-registering after a settings change).
static void register_dictionary_lookup_from_settings() {
  auto* m = AppGlobals::global_hotkey_manager;
  if (!m) return;
  if (!global_hotkeys_enabled_c()) {
    log_info_c("global_hotkeys: disabled in settings, manager idle");
    return;
  }
  if (!m->isInitialized()) {
    log_error_c("global_hotkeys: platform backend not available (Wayland?), skipping registration");
    return;
  }
  char* seq_c = get_global_hotkey_dictionary_lookup_c();
  if (!seq_c) {
    log_info_c("global_hotkeys: no dictionary_lookup binding configured");
    return;
  }
  QString seq = QString::fromUtf8(seq_c);
  free_rust_string(seq_c);

  // QKeySequence parses '+' as the modifier separator only. The user-facing
  // double-tap form "Ctrl+C+C" must be converted to Qt's chord-separator
  // form "Ctrl+C, C" first.
  const QString normalized = GlobalHotkeyManager::normalizeSequenceString(seq);
  QKeySequence ks(normalized);
  if (m->registerHotkey(ks, /*handle*/ 0)) {
    log_info_c(QString("global_hotkeys: registered dictionary_lookup as %1 (parsed as %2)")
               .arg(seq, normalized).toUtf8().constData());
  } else {
    log_error_c(QString("global_hotkeys: failed to register %1 (normalized: %2)")
                .arg(seq, normalized).toUtf8().constData());
    show_global_hotkey_registration_error(seq);
  }
}

/// Construct the GlobalHotkeyManager and, if enabled in settings, register
/// the configured `dictionary_lookup` sequence. Connects the activation
/// signal to the lookup pipeline. Safe to call once from start() after
/// QApplication has been constructed.
static void init_global_hotkey_manager(QApplication* app) {
  if (AppGlobals::global_hotkey_manager) return;

  auto* m = new GlobalHotkeyManager(app);
  AppGlobals::global_hotkey_manager = m;

  QObject::connect(m, &GlobalHotkeyManager::hotkeyActivated,
                   m, [](int handle){ callback_global_hotkey_activated(handle); },
                   Qt::QueuedConnection);

  register_dictionary_lookup_from_settings();
}

/// FFI: invoked by the Rust bridge when global-hotkey settings change so the
/// C++ manager unregisters old grabs and re-registers from current settings —
/// no app restart required (PRD §4.7 / task 8.3).
extern "C" void reregister_global_hotkeys_c() {
  auto* m = AppGlobals::global_hotkey_manager;
  if (!m) return;
  m->unregisterAll();
  register_dictionary_lookup_from_settings();
}

/// FFI: clears the "registration error already shown this session" flag so a
/// fresh attempt with a new sequence can surface a fresh dialog (task 8.6).
extern "C" void reset_global_hotkey_error_flag_c() {
  s_global_hotkey_error_shown = false;
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

  // Construct and (if enabled in settings) register the OS-level global
  // hotkey for dictionary lookup. Created after the first sutta window so
  // that activations have a window to deliver to. Lifetime: parented to the
  // QApplication, destroyed on app shutdown which unregisters X grabs.
  init_global_hotkey_manager(&app);

  // Release OS-level global hotkey grabs cleanly on shutdown (task 8.4).
  // Connected before the session-save handler so grabs are released even if
  // the latter throws.
  QObject::connect(&app, &QApplication::aboutToQuit, [&]() {
    if (AppGlobals::global_hotkey_manager) {
      log_info_c("aboutToQuit: unregistering global hotkeys");
      AppGlobals::global_hotkey_manager->unregisterAll();
    }
  });

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
