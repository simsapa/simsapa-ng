#include <iostream>
#include <thread>
#include <QtGui/QGuiApplication>
#include <QtQml/QQmlApplicationEngine>
#include <QtQml/QQmlContext>
// #include <typeinfo>

extern "C" void start_webserver();
extern "C" void shutdown_webserver();

int main(int argc, char* argv[]) {
  std::thread webserver_thread(start_webserver);

  QCoreApplication::setApplicationName("Simsapa Dhamma Reader");
  QCoreApplication::setOrganizationName("Profound Labs");

  QGuiApplication app(argc, argv);

  QQmlApplicationEngine engine;

  const QUrl sutta_search_window_qml(QStringLiteral("qrc:/qt/qml/com/profound_labs/simsapa/qml/sutta_search_window.qml"));
  QObject::connect(
    &engine,
    &QQmlApplicationEngine::objectCreated,
    &app,
    [sutta_search_window_qml](QObject* obj, const QUrl& objUrl) {
      if (!obj && sutta_search_window_qml == objUrl)
        QCoreApplication::exit(-1);
    },
    Qt::QueuedConnection);

  engine.load(sutta_search_window_qml);

  // QMetaObject::invokeMethod(engine.rootObjects().constFirst(),
  //                           "load_url",
  //                           Q_ARG(QVariant, QUrl("http://localhost:8484/index.html")));

  int app_ret = app.exec();

  shutdown_webserver();

  if (webserver_thread.joinable()) {
    webserver_thread.join();
  }

  std::cout << "Exiting with status " << app_ret << "." << std::endl;

  return app_ret;
}
