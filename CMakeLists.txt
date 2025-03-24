cmake_minimum_required(VERSION 3.24)

project(example_simsapa LANGUAGES CXX)

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

set(CMAKE_AUTORCC_OPTIONS --format-version 1)
list(APPEND CMAKE_AUTORCC_OPTIONS --compress-algo zlib)

set(CXXQT_QTCOMPONENTS Core Gui Qml QuickControls2 QuickTest Test)
set(CXXQT_QTCOMPONENTS ${CXXQT_QTCOMPONENTS} QmlImportScanner)

find_package(Qt6 COMPONENTS ${CXXQT_QTCOMPONENTS} Quick WebView)

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
    QMAKE /home/gambhiro/Qt/6.8.2/android_x86_64/bin/qmake6

    QT_MODULES
        Qt::Core
        Qt::Gui
        Qt::Qml
        Qt::Quick
        Qt::QuickControls2
        Qt::Network
        Qt::WebView
)

cxx_qt_import_qml_module(simsapa_qml_module
    URI "com.profound_labs.simsapa"
    SOURCE_CRATE simsapa_lib)

qt_add_resources(icons "assets/icons.qrc")

qt_add_executable(example_simsapa
    cpp/main.cpp

    ${icons}

    android/AndroidManifest.xml
    android/build.gradle
    android/gradle.properties
    android/gradlew
    android/gradlew.bat
    android/gradle/wrapper/gradle-wrapper.jar
    android/gradle/wrapper/gradle-wrapper.properties
    android/res/values/libs.xml
    android/res/xml/network_security_config.xml
    android/res/xml/qtprovider_paths.xml
)

set_target_properties(example_simsapa PROPERTIES
    WIN32_EXECUTABLE TRUE
    MACOSX_BUNDLE TRUE
    QT_ANDROID_PACKAGE_SOURCE_DIR ${CMAKE_CURRENT_SOURCE_DIR}/android
)

# Link to the qml module, which in turn links to the Rust simsapa library
target_link_libraries(example_simsapa
    PRIVATE simsapa_qml_module
    PUBLIC
        simsapa_lib
        Qt6::Core
        Qt6::Gui
        Qt6::Qml
        Qt6::Quick
        Qt::QuickControls2
        Qt::Network
        Qt::WebView
    )

# If we are using a statically linked Qt then we need to import any qml plugins
qt_import_qml_plugins(example_simsapa)

add_custom_command(
    TARGET example_simsapa PRE_BUILD
    COMMAND ${CMAKE_COMMAND} -E echo "Copy assets..."
    COMMAND ${CMAKE_CURRENT_SOURCE_DIR}/scripts/android_assets.sh ${PROJECT_NAME} ${PROJECT_SOURCE_DIR}
)
