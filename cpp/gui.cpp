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
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQuickStyle>

// #include <QtWebView/QtWebView>
// #include <QtWebEngineQuick/qtwebenginequickglobal.h>

#include "errors.h"
#include "window_manager.h"

extern "C" void start_webserver();
extern "C" void shutdown_webserver();
extern "C" bool appdata_db_exists();
extern "C" void ensure_no_empty_db_files();
extern "C" void remove_download_temp_folder();
extern "C" void init_app_globals();
extern "C" void init_app_data();
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

void callback_open_sutta_search_window() {
  AppGlobals::manager->create_sutta_search_window();
}

void start(int argc, char* argv[]) {
  dotenv_c();
  log_info_with_options_c("gui::start()", true);
  find_port_set_env_c();
  init_app_globals();
  remove_download_temp_folder();

  // There may be a 0-byte size db file remaining from a failed
  // install attempt.
  ensure_no_empty_db_files();

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

  app.setApplicationVersion("v0.1.3");

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

  // Init AppData and start the API server after checking for APP_DB. If this is the first run,
  // init_app_data() would create the userdata db, and we can't use it to test in
  // DownloadAppdataWindow() if this is the first ever start.
  init_app_data();

  // The port is determined in start_webserver().
  std::thread daemon_server_thread(start_webserver);

  // === Create the first app window ===

  AppGlobals::manager->create_sutta_search_window();
  // AppGlobals::manager->create_word_lookup_window("hey ho");

  log_info_c("app.exec()");
  int status = app.exec();

  shutdown_webserver();
  if (daemon_server_thread.joinable()) {
    daemon_server_thread.join();
  }
  std::ostringstream msg;
  msg << "Exiting with status " << status << ".";
  log_info_c(msg.str().c_str());
}
