#include <iostream>
#include <qcoreevent.h>
#include <qguiapplication.h>
#include <qsysinfo.h>
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
#include <QGuiApplication>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include "window_manager.h"

extern "C" void start_webserver();
extern "C" void shutdown_webserver();

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

  // QApplication has to be constructed before other windows or dialogs.
  QApplication app(argc, argv);

  QCoreApplication::setApplicationName("Simsapa Dhamma Reader");
  QCoreApplication::setOrganizationName("Profound Labs");

  // TODO :/icons/simsapa-appicon doesn't work, perhaps wrong size?
  app.setWindowIcon(QIcon(":/icons/simsapa-tray"));

  // if DESKTOP_FILE_PATH is not None:
  //     app.setDesktopFileName(str(DESKTOP_FILE_PATH.with_suffix("")))
  //
  app.setApplicationVersion("v0.1.0");

  // Start the API server after checking for APP_DB. If this is the first run,
  // the server would create the userdata db, and we can't use it to test in
  // DownloadAppdataWindow() if this is the first ever start.
  //
  // The port is determined in start_webserver().
  std::thread daemon_server_thread(start_webserver);

  // app_windows = AppWindows(app, app_data, hotkeys_manager, enable_tray_icon)

  // setup_system_tray();

  std::cout << "setup_system_tray(): start" << std::endl;
  QSystemTrayIcon tray = QSystemTrayIcon(QIcon(":/icons/simsapa-tray"), &app);
  tray.setVisible(true);

  QMenu* menu = new QMenu();

  QAction* action_Quit = new QAction(QIcon(":/icons/close"), "Quit", &app);
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
