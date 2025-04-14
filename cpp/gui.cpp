#include <iostream>
#include <thread>

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

// #include <QtWebView/QtWebView>
// #include <QtWebEngineQuick/qtwebenginequickglobal.h>

#include "window_manager.h"

extern "C" void start_webserver();
extern "C" void shutdown_webserver();
extern "C" void download_small_database();

struct AppGlobals {
    static WindowManager* manager;
};

WindowManager* AppGlobals::manager = nullptr;

void callback_run_lookup_query(QString query_text) {
  std::cout << "callback_run_lookup_query(): " << query_text.toStdString() << std::endl;
  emit AppGlobals::manager->signal_run_lookup_query(query_text);
}

void start(int argc, char* argv[]) {
  std::cout << "gui::start()" << std::endl;

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

  // QApplication has to be constructed before other windows or dialogs.
  QApplication app(argc, argv);

  QCoreApplication::setApplicationName("Simsapa Dhamma Reader");
  QCoreApplication::setOrganizationName("Profound Labs");

  // TODO :/icons/simsapa-appicon doesn't work, perhaps wrong size?
  app.setWindowIcon(QIcon(":/qt/qml/com/profoundlabs/simsapa/assets/qml/icons/32x32/simsapa-tray.png"));

  // if DESKTOP_FILE_PATH is not None:
  //     app.setDesktopFileName(str(DESKTOP_FILE_PATH.with_suffix("")))
  //
  app.setApplicationVersion("v0.1.0");

  download_small_database();

  // Start the API server after checking for APP_DB. If this is the first run,
  // the server would create the userdata db, and we can't use it to test in
  // DownloadAppdataWindow() if this is the first ever start.
  //
  // The port is determined in start_webserver().
  std::thread daemon_server_thread(start_webserver);

  // app_windows = AppWindows(app, app_data, hotkeys_manager, enable_tray_icon)

  // setup_system_tray();

  std::cout << "setup_system_tray(): start" << std::endl;
  QSystemTrayIcon tray = QSystemTrayIcon(QIcon(":/qt/qml/com/profoundlabs/simsapa/assets/qml/icons/32x32/simsapa-tray.png"), &app);
  tray.setVisible(true);

  QMenu* menu = new QMenu();

  QAction* action_Quit = new QAction(QIcon(":/qt/qml/com/profoundlabs/simsapa/assets/qml/icons/32x32/fa_times-circle.png"), "Quit", &app);
  QObject::connect(action_Quit, SIGNAL(triggered()), QApplication::instance(), SLOT(quit()));

  menu->addAction(action_Quit);

  tray.setContextMenu(menu);

  std::cout << "setup_system_tray(): end" << std::endl;

  // === Create first window ===

  AppGlobals::manager = &WindowManager::instance(&app);
  AppGlobals::manager->create_sutta_search_window();
  // AppGlobals::manager->create_word_lookup_window("hey ho");

  std::cout << "app.exec()" << std::endl;
  int status = app.exec();

  shutdown_webserver();

  if (daemon_server_thread.joinable()) {
    daemon_server_thread.join();
  }

  std::cout << "Exiting with status " << status << "." << std::endl;
}
