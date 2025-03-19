cmake_minimum_required(VERSION 3.24)

project(example_simsapa)

# Rust always links against non-debug Windows runtime on *-msvc targets
# Note it is best to set this on the command line to ensure all targets are consistent
# https://github.com/corrosion-rs/corrosion/blob/master/doc/src/common_issues.md#linking-debug-cc-libraries-into-rust-fails-on-windows-msvc-targets
# https://github.com/rust-lang/rust/issues/39016
if (CMAKE_CXX_COMPILER_ID STREQUAL "MSVC")
  set(CMAKE_MSVC_RUNTIME_LIBRARY "MultiThreadedDLL")
endif()

set(CMAKE_AUTOMOC ON)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

set(CXXQT_QTCOMPONENTS Core Gui Qml QuickControls2 QuickTest Test)
set(CXXQT_QTCOMPONENTS ${CXXQT_QTCOMPONENTS} QmlImportScanner)

find_package(Qt6 COMPONENTS ${CXXQT_QTCOMPONENTS} Quick WebEngineQuick WebEngineWidgets)

find_package(CxxQt QUIET)
if(NOT CxxQt_FOUND)
    include(FetchContent)
    FetchContent_Declare(
        CxxQt
        GIT_REPOSITORY https://github.com/kdab/cxx-qt-cmake.git
        GIT_TAG main
    )

    FetchContent_MakeAvailable(CxxQt)
endif()

# CXX-Qt (using Corrosion) creates a CMake target with the same name as the crate.
cxx_qt_import_crate(
    MANIFEST_PATH rust/Cargo.toml 
    CRATES simsapa_lib
    LOCKED
    QT_MODULES Qt::Core Qt::Gui Qt::Qml Qt::QuickControls2 Qt::Network Qt::Quick Qt::WebEngineQuick Qt6::WebEngineWidgets
    QMAKE /usr/bin/qmake6
)

cxx_qt_import_qml_module(simsapa_qml_module
    URI "com.profound_labs.simsapa"
    SOURCE_CRATE simsapa_lib)

qt_add_resources(icons "assets/icons.qrc")

add_executable(example_simsapa ${icons} cpp/main.cpp)

# Link to the qml module, which in turn links to the Rust simsapa library
target_link_libraries(example_simsapa
    PRIVATE simsapa_qml_module
    PUBLIC simsapa_lib sqlite3)

# If we are using a statically linked Qt then we need to import any qml plugins
qt_import_qml_plugins(example_simsapa)
